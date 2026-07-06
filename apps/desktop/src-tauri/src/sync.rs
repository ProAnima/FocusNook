use crate::config::SyncProvidersConfig;
use crate::oauth::{self, ProviderId};
use crate::{profiles, sync_tokens};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionStatus {
    pub connected: bool,
}

fn credentials_for(
    config: &SyncProvidersConfig,
    provider: ProviderId,
) -> Option<&crate::config::ProviderCredentials> {
    match provider {
        ProviderId::GoogleDrive => config.google.as_ref(),
        ProviderId::YandexDisk => config.yandex.as_ref(),
    }
}

#[tauri::command]
pub async fn start_provider_auth(
    app: tauri::AppHandle,
    profiles_state: tauri::State<'_, profiles::ProfilesState>,
    config: tauri::State<'_, SyncProvidersConfig>,
    provider: ProviderId,
) -> Result<(), String> {
    // Синхронный лок дропается сразу, до единственного await ниже — тот же
    // принцип, что и у остальных команд этого шага, только здесь особенно
    // важно проговорить явно: run_flow ждёт клика пользователя в браузере,
    // который может занять сколько угодно времени, и держать что-либо
    // залоченным всё это время было бы серьёзной ошибкой, а не мелочью.
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let creds = credentials_for(&config, provider)
        .ok_or_else(|| {
            "провайдер не настроен — впишите client_id/secret в sync_providers.json".to_string()
        })?
        .clone();
    oauth::run_flow(&app, provider, &creds, &profile_id).await
}

#[tauri::command]
pub fn connection_status(
    profiles_state: tauri::State<profiles::ProfilesState>,
    provider: ProviderId,
) -> Result<ConnectionStatus, String> {
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    // "Подключено" здесь значит "есть сохранённый refresh-токен", не "токен
    // подтверждённо рабочий прямо сейчас" — живая проверка потребовала бы
    // того же HTTP-клиента, что и push/pull, которых в этом шаге ещё нет.
    // Осознанно более слабая гарантия, см. архитектурный документ.
    let token = sync_tokens::load_refresh_token(&profile_id, provider)?;
    Ok(ConnectionStatus {
        connected: token.is_some(),
    })
}

#[tauri::command]
pub fn disconnect_provider(
    profiles_state: tauri::State<profiles::ProfilesState>,
    provider: ProviderId,
) -> Result<(), String> {
    // Стирает токен только локально — не отзывает его на стороне провайдера
    // (у Google/Yandex есть свои revoke-эндпоинты, не вызываются здесь).
    // Осознанный, но стоящий явного упоминания в UI пробел.
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    sync_tokens::delete_refresh_token(&profile_id, provider)
}
