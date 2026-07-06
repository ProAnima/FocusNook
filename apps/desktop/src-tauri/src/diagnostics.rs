use crate::alerts::{self, AlertState};
use rusqlite::Connection;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// Раздел 19 ТЗ перечисляет active sync provider/last sync/pending operations
// (Iteration 2 — до sync их не существует), notification permission и exact
// alarm permission (Android-специфика — вне текущего фокуса на десктоп), и
// device id hash (чисто sync-концепция, device linking — тоже Iteration 2).
// Заполнять их выдуманными значениями сейчас хуже, чем честно не заводить
// поле: набор ниже — то, что реально применимо к desktop-only стадии без sync.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsBundle {
    pub app_version: String,
    pub platform: String,
    pub generated_at: String,
    pub profile_id_hash: String,
    pub profile_count: usize,
    pub plan_item_count: i64,
    pub note_count: i64,
    pub reminder_count: i64,
    pub scheduler_last_poll_seconds_ago: Option<i64>,
}

// std::hash (SipHash), не криптографический хэш — цель не защита от
// целенаправленного восстановления id, а не светить его как есть в бандле,
// который пользователь может вставить в переписку с поддержкой.
fn hash_id(id: &str) -> String {
    let mut hasher = DefaultHasher::new();
    id.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn count(conn: &Connection, table: &str) -> Result<i64, String> {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
    .map_err(|e| e.to_string())
}

pub fn build(
    conn: &Connection,
    app_version: &str,
    profile_count: usize,
    active_profile_id: &str,
    alert_state: &AlertState,
) -> Result<DiagnosticsBundle, String> {
    let generated_at: String = conn
        .query_row("SELECT datetime('now')", [], |row| row.get(0))
        .map_err(|e| e.to_string())?;
    Ok(DiagnosticsBundle {
        app_version: app_version.to_string(),
        platform: std::env::consts::OS.to_string(),
        generated_at,
        profile_id_hash: hash_id(active_profile_id),
        profile_count,
        plan_item_count: count(conn, "plan_items")?,
        note_count: count(conn, "notes")?,
        reminder_count: count(conn, "reminders")?,
        scheduler_last_poll_seconds_ago: alerts::seconds_since_last_poll(alert_state),
    })
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute("CREATE TABLE plan_items (id TEXT PRIMARY KEY)", [])
            .unwrap();
        conn.execute("CREATE TABLE notes (id TEXT PRIMARY KEY)", [])
            .unwrap();
        conn.execute("CREATE TABLE reminders (id TEXT PRIMARY KEY)", [])
            .unwrap();
        conn.execute("INSERT INTO plan_items VALUES ('1'), ('2')", [])
            .unwrap();
        conn.execute("INSERT INTO notes VALUES ('a')", []).unwrap();
        conn
    }

    // Это ровно тот путь, который я не мог кликнуть в реальном нативном
    // окне (нет инструмента для клика внутри WebView2) — прямой вызов той же
    // функции, что дергает Tauri-команда export_diagnostics, закрывает этот
    // пробел без UI: SQL-подсчёты, хэш и сериализация проверены по-настоящему,
    // а не только статическим ревью кода.
    #[test]
    fn builds_bundle_with_correct_counts_and_hashed_id() {
        let conn = setup_conn();
        let alert_state = AlertState::default();
        let bundle = build(&conn, "0.1.0", 2, "profile-123", &alert_state).unwrap();

        assert_eq!(bundle.app_version, "0.1.0");
        assert_eq!(bundle.profile_count, 2);
        assert_eq!(bundle.plan_item_count, 2);
        assert_eq!(bundle.note_count, 1);
        assert_eq!(bundle.reminder_count, 0);
        assert_ne!(bundle.profile_id_hash, "profile-123");
        assert!(!bundle.profile_id_hash.is_empty());
        assert_eq!(bundle.scheduler_last_poll_seconds_ago, None);
    }

    #[test]
    fn hash_is_deterministic_but_not_reversible_at_a_glance() {
        assert_eq!(hash_id("same-id"), hash_id("same-id"));
        assert_ne!(hash_id("id-a"), hash_id("id-b"));
    }

    #[test]
    fn serialized_json_uses_camel_case_and_omits_the_raw_id() {
        let conn = setup_conn();
        let alert_state = AlertState::default();
        let bundle = build(&conn, "0.1.0", 1, "raw-profile-id", &alert_state).unwrap();
        let json = serde_json::to_string(&bundle).unwrap();

        assert!(json.contains("\"appVersion\":\"0.1.0\""));
        assert!(json.contains("\"profileIdHash\""));
        assert!(!json.contains("raw-profile-id"));
    }
}
