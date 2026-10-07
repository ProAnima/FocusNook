use super::apply::{apply_exchange_response, ExchangeScope};
use super::attachments::{download_pending_blobs, transfer_status_message, upload_pending_blobs};
use super::credentials::{
    endpoint_from_config, load_credentials, server_profile_id, status_for_profile,
    ServerSyncCredentials, ServerSyncStatus,
};
use super::journal::{
    last_pulled_hlc, mark_full_reconciled, prepare_account_scope_if_needed,
    ready_unsynced_operations, should_run_full_reconcile, store_last_pulled_hlc,
};
use super::transport::{exchange_with_server, wait_for_server_event};
use super::{
    MAX_EXCHANGE_ROUNDS, MAX_OPS_PER_EXCHANGE, PERIODIC_SYNC_INTERVAL, SERVER_EVENT_ERROR_BACKOFF,
    SYNC_IN_FLIGHT, SYNC_RERUN_REQUESTED,
};
use crate::{config, profiles, sync_log};
use std::sync::atomic::{AtomicU32, Ordering};
use tauri::{Emitter, Manager};

/// Серия повторяющихся сбоев (например, нет сети): первый сбой — `warn`,
/// повторы — `debug`, восстановление — `info`. Иначе офлайн-устройство писало бы
/// предупреждение каждые 10–60 секунд и вытесняло полезную историю из ротации логов.
pub(super) struct FailureStreak(AtomicU32);

impl FailureStreak {
    pub(super) const fn new() -> Self {
        Self(AtomicU32::new(0))
    }

    /// Возвращает `true`, если сбой открыл новую серию (записан как `warn`).
    pub(super) fn failed(&self, what: &str, err: &str) -> bool {
        let first = self.0.fetch_add(1, Ordering::SeqCst) == 0;
        if first {
            log::warn!("{what}: {err}");
        } else {
            log::debug!("{what} (повтор): {err}");
        }
        first
    }

    /// Возвращает число сбоев закрытой серии (0 — серии не было).
    pub(super) fn succeeded(&self, what: &str) -> u32 {
        let failures = self.0.swap(0, Ordering::SeqCst);
        if failures > 0 {
            log::info!("{what}: восстановлено после {failures} неудачных попыток");
        }
        failures
    }
}

static SYNC_FAILURES: FailureStreak = FailureStreak::new();
static EVENT_LISTENER_FAILURES: FailureStreak = FailureStreak::new();

async fn perform_sync(app: tauri::AppHandle) -> Result<ServerSyncStatus, String> {
    let started = std::time::Instant::now();
    log::debug!("sync cycle: старт");
    let result = run_sync_cycle(app).await;
    let elapsed_ms = started.elapsed().as_millis();
    match &result {
        Ok(_) => log::debug!("sync cycle: завершён за {elapsed_ms} мс"),
        Err(err) => log::debug!("sync cycle: ошибка через {elapsed_ms} мс: {err}"),
    }
    result
}

async fn run_sync_cycle(app: tauri::AppHandle) -> Result<ServerSyncStatus, String> {
    let db = app.state::<crate::db::Db>();
    let config = app.state::<config::SyncProvidersConfig>();
    let profiles_state = app.state::<profiles::ProfilesState>();
    let hlc_state = app.state::<sync_log::HlcClockState>();
    let audio_key_state = app.state::<crate::AudioKeyState>();
    if !profiles::active_sync_enabled(&profiles_state)? {
        return Err("server sync is disabled for this account".to_string());
    }
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let configured = endpoint_from_config(&config).is_ok();
    let audio_dir = profiles::audio_dir(&profiles_state)?;
    let audio_key = audio_key_state.0.lock().map_err(|e| e.to_string())?.clone();

    let credentials = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        load_credentials(Some(&conn), &profile_id)?
            .ok_or_else(|| "server sync account is not connected".to_string())?
    };
    let remote_profile_id = server_profile_id(&credentials, &profile_id);

    {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        prepare_account_scope_if_needed(
            &conn,
            &profile_id,
            &remote_profile_id,
            &credentials.device_id,
        )?;
    }

    let mut snapshot_pending = {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        !crate::sync_snapshot::is_seeded(&conn, &remote_profile_id)?
    };
    let mut full_reconcile_active = snapshot_pending || {
        let conn = db.0.lock().map_err(|e| e.to_string())?;
        should_run_full_reconcile(&conn, &remote_profile_id)?
    };
    let mut full_reconcile_cursor = None;

    for _ in 0..MAX_EXCHANGE_ROUNDS {
        let blocked_uploads = upload_pending_blobs(
            &db,
            &credentials,
            &profile_id,
            &remote_profile_id,
            &audio_dir,
            audio_key.as_deref(),
        )
        .await?;

        let (last_pulled, operations) = {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            let last_pulled = if full_reconcile_active {
                full_reconcile_cursor.clone()
            } else {
                last_pulled_hlc(&conn, &remote_profile_id)?
            };
            let operations = ready_unsynced_operations(&conn, &profile_id, &remote_profile_id)?;
            (last_pulled, operations)
        };
        let sent_full_page = operations.len() == MAX_OPS_PER_EXCHANGE;
        let sent_operation_ids = operations
            .iter()
            .map(|operation| operation.operation_id.clone())
            .collect::<Vec<_>>();
        let response =
            exchange_with_server(&credentials, &remote_profile_id, last_pulled, &operations)
                .await?;
        let pulled_full_page = response.operations.len() == MAX_OPS_PER_EXCHANGE;
        log::debug!(
            "sync exchange: отправлено операций {}, получено {}",
            operations.len(),
            response.operations.len()
        );

        let (next_pull_hlc, _) = {
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            let scope = ExchangeScope {
                app: &app,
                hlc_state: &hlc_state,
                profile_id: &profile_id,
                remote_profile_id: &remote_profile_id,
                credentials: &credentials,
            };
            apply_exchange_response(&scope, &conn, &sent_operation_ids, response)?
        };
        let (snapshot_queued, exchange_complete) = {
            let mut conn = db.0.lock().map_err(|e| e.to_string())?;
            store_last_pulled_hlc(&conn, &remote_profile_id, next_pull_hlc.as_deref())?;
            if full_reconcile_active {
                full_reconcile_cursor = next_pull_hlc.clone();
                if !pulled_full_page {
                    mark_full_reconciled(&conn, &remote_profile_id)?;
                    full_reconcile_active = false;
                }
            }
            let snapshot_queued = snapshot_pending && !full_reconcile_active;
            if snapshot_queued {
                let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
                crate::sync_snapshot::ensure_queued(
                    &mut conn,
                    &mut clock,
                    &profile_id,
                    &remote_profile_id,
                )?;
                snapshot_pending = false;
            }
            (snapshot_queued, !sent_full_page && !pulled_full_page)
        };
        let unavailable_audio = download_pending_blobs(
            &db,
            &credentials,
            &profile_id,
            &remote_profile_id,
            &audio_dir,
            audio_key.as_deref(),
        )
        .await?;
        if snapshot_queued {
            continue;
        }

        if exchange_complete {
            let message = transfer_status_message(blocked_uploads, unavailable_audio);
            let conn = db.0.lock().map_err(|e| e.to_string())?;
            return status_for_profile(Some(&conn), &profile_id, configured, message);
        }
    }

    Err("server sync backlog is too large, please run sync again".to_string())
}

pub fn spawn_best_effort(app: tauri::AppHandle) {
    let profiles_state = app.state::<profiles::ProfilesState>();
    if !profiles::active_sync_enabled(&profiles_state).unwrap_or(false) {
        return;
    }
    if SYNC_IN_FLIGHT
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        SYNC_RERUN_REQUESTED.store(true, Ordering::SeqCst);
        return;
    }
    tauri::async_runtime::spawn(async move {
        let mut result;
        loop {
            SYNC_RERUN_REQUESTED.store(false, Ordering::SeqCst);
            result = perform_sync(app.clone()).await;
            if !SYNC_RERUN_REQUESTED.swap(false, Ordering::SeqCst) {
                break;
            }
        }
        SYNC_IN_FLIGHT.store(false, Ordering::SeqCst);
        if SYNC_RERUN_REQUESTED.swap(false, Ordering::SeqCst) {
            spawn_best_effort(app);
            return;
        }
        match result {
            Ok(_) => {
                SYNC_FAILURES.succeeded("best-effort sync");
                let _ = app.emit("server-sync-completed", ());
            }
            Err(err) => {
                SYNC_FAILURES.failed("best-effort sync failed", &err);
                let _ = app.emit("server-sync-failed", err);
            }
        }
    });
}

pub fn spawn_periodic_best_effort(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(PERIODIC_SYNC_INTERVAL).await;
            spawn_best_effort(app.clone());
        }
    });
}

fn event_listener_credentials(
    app: &tauri::AppHandle,
) -> Result<Option<ServerSyncCredentials>, String> {
    let db = app.state::<crate::db::Db>();
    let profiles_state = app.state::<profiles::ProfilesState>();
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    load_credentials(Some(&conn), &profile_id)
}

pub fn spawn_server_event_listener(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last_sequence = 0;
        let mut last_scope = None;
        loop {
            let credentials = match event_listener_credentials(&app) {
                Ok(credentials) => credentials,
                Err(err) => {
                    EVENT_LISTENER_FAILURES.failed("cannot load event listener credentials", &err);
                    tokio::time::sleep(SERVER_EVENT_ERROR_BACKOFF).await;
                    continue;
                }
            };

            let Some(credentials) = credentials else {
                last_scope = None;
                last_sequence = 0;
                tokio::time::sleep(PERIODIC_SYNC_INTERVAL).await;
                continue;
            };
            let scope = format!(
                "{}|{}",
                credentials.endpoint,
                credentials
                    .account_user_id
                    .as_deref()
                    .unwrap_or(&credentials.token)
            );
            if last_scope.as_deref() != Some(scope.as_str()) {
                last_scope = Some(scope);
                last_sequence = 0;
            }

            match wait_for_server_event(&credentials, last_sequence).await {
                Ok(event) if event.changed => {
                    EVENT_LISTENER_FAILURES.succeeded("event listener");
                    last_sequence = event.sequence.max(last_sequence);
                    spawn_best_effort(app.clone());
                }
                Ok(_) => {
                    EVENT_LISTENER_FAILURES.succeeded("event listener");
                }
                Err(err) => {
                    EVENT_LISTENER_FAILURES.failed("event listener failed", &err);
                    tokio::time::sleep(SERVER_EVENT_ERROR_BACKOFF).await;
                }
            }
        }
    });
}

#[tauri::command]
pub async fn sync_server_now(app: tauri::AppHandle) -> Result<ServerSyncStatus, String> {
    perform_sync(app).await
}

#[tauri::command]
pub fn request_server_sync(app: tauri::AppHandle) {
    spawn_best_effort(app);
}

#[cfg(test)]
mod tests {
    use super::FailureStreak;

    #[test]
    fn failure_streak_warns_once_per_streak_and_reports_recovery() {
        let streak = FailureStreak::new();
        assert_eq!(streak.succeeded("sync"), 0);
        assert!(streak.failed("sync failed", "offline"));
        assert!(!streak.failed("sync failed", "offline"));
        assert!(!streak.failed("sync failed", "offline"));
        assert_eq!(streak.succeeded("sync"), 3);
        assert!(streak.failed("sync failed", "offline"));
    }
}
