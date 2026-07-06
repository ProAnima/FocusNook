use crate::sync_log::{self, HlcClock};
use rusqlite::{params, Connection, Row};
use serde::Serialize;

// Раздел 8 ТЗ: Reminder. Срабатывание/очередь/окно живут в alerts.rs —
// этот модуль только данные.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDto {
    pub id: String,
    pub title: String,
    pub trigger_at_utc: String,
    pub status: String,
}

fn row_to_dto(row: &Row) -> rusqlite::Result<ReminderDto> {
    Ok(ReminderDto {
        id: row.get(0)?,
        title: row.get(1)?,
        trigger_at_utc: row.get(2)?,
        status: row.get(3)?,
    })
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<ReminderDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, trigger_at_utc, status FROM reminders ORDER BY trigger_at_utc ASC",
    )?;
    let reminders = stmt.query_map([], row_to_dto)?.collect();
    reminders
}

// Раздел 9 ТЗ, Iteration 2 (первый локальный шаг): мутация и запись в
// sync_operations идут одной транзакцией — см. plan_items.rs::toggle_done
// для развёрнутого объяснения, здесь тот же паттерн.
pub fn create(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    title: &str,
    trigger_at_utc: &str,
) -> rusqlite::Result<ReminderDto> {
    let tx = conn.transaction()?;
    let id = uuid::Uuid::now_v7().to_string();
    tx.execute(
        "INSERT INTO reminders (id, title, trigger_at_utc, status, created_at)
         VALUES (?1, ?2, ?3, 'scheduled', datetime('now'))",
        params![id, title, trigger_at_utc],
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "reminder",
        &id,
        "create",
        &serde_json::json!({ "title": title, "triggerAtUtc": trigger_at_utc }),
    )?;
    tx.commit()?;
    Ok(ReminderDto {
        id,
        title: title.to_string(),
        trigger_at_utc: trigger_at_utc.to_string(),
        status: "scheduled".to_string(),
    })
}

// datetime(...) в SQLite нормализует и ISO-с-T-и-Z, и свой формат 'now' к
// одному виду перед сравнением — не нужен отдельный date/time crate в Rust
// только ради проверки "наступило ли время".
pub fn due(conn: &Connection) -> rusqlite::Result<Vec<ReminderDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, trigger_at_utc, status FROM reminders
         WHERE status = 'scheduled' AND datetime(trigger_at_utc) <= datetime('now')
         ORDER BY trigger_at_utc ASC",
    )?;
    let due = stmt.query_map([], row_to_dto)?.collect();
    due
}

// Не логируется в sync_operations: это внутреннее срабатывание планировщика
// (alerts::check_due_reminders), не действие пользователя — логировать его
// значило бы шуметь в журнале и создавать гонку с правилом "более поздний
// user action побеждает scheduled state" (раздел 14), которое пока не с чем
// разрешать (нет второго устройства/лога).
pub fn mark_firing(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE reminders SET status = 'firing' WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn acknowledge(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    id: &str,
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE reminders SET status = 'acknowledged' WHERE id = ?1",
        params![id],
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "reminder",
        id,
        "update",
        &serde_json::json!({ "status": "acknowledged" }),
    )?;
    tx.commit()?;
    Ok(())
}

pub fn delete(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    id: &str,
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM reminders WHERE id = ?1", params![id])?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "reminder",
        id,
        "delete",
        &serde_json::json!({}),
    )?;
    tx.commit()?;
    Ok(())
}

// Snooze-политика: возвращает напоминание в 'scheduled' независимо от того,
// в каком статусе оно было (firing) — иначе "отложить на 5 минут" не
// сработало бы после срабатывания алерта.
pub fn reschedule(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    id: &str,
    new_trigger_at_utc: &str,
) -> rusqlite::Result<ReminderDto> {
    let tx = conn.transaction()?;
    let dto = tx.query_row(
        "UPDATE reminders SET status = 'scheduled', trigger_at_utc = ?1 WHERE id = ?2
         RETURNING id, title, trigger_at_utc, status",
        params![new_trigger_at_utc, id],
        row_to_dto,
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "reminder",
        id,
        "update",
        &serde_json::json!({ "status": "scheduled", "triggerAtUtc": new_trigger_at_utc }),
    )?;
    tx.commit()?;
    Ok(dto)
}

// Раздел 11 ТЗ: Android-alarm нужен epoch-millis, а не строка. trigger_at_utc
// всегда приходит из JS Date.prototype.toISOString() — фиксированный формат
// "YYYY-MM-DDTHH:mm:ss.sssZ", поэтому раскладываем по позициям вручную и
// считаем дни от эпохи алгоритмом Хинанта (days_from_civil), не добавляя
// отдельный date/time crate ради одной конвертации.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

// Обратное к days_from_civil (тот же алгоритм Хинанта) — нужно sync_log.rs
// для форматирования HLC в ISO8601 (millis -> "YYYY-MM-DD"), не только тестам.
pub(crate) fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn parse_trigger_millis(trigger_at_utc: &str) -> Option<i64> {
    let bytes = trigger_at_utc.as_bytes();
    if bytes.len() < 20 || *bytes.last()? != b'Z' {
        return None;
    }
    let year: i64 = trigger_at_utc.get(0..4)?.parse().ok()?;
    let month: i64 = trigger_at_utc.get(5..7)?.parse().ok()?;
    let day: i64 = trigger_at_utc.get(8..10)?.parse().ok()?;
    let hour: i64 = trigger_at_utc.get(11..13)?.parse().ok()?;
    let minute: i64 = trigger_at_utc.get(14..16)?.parse().ok()?;
    let second: i64 = trigger_at_utc.get(17..19)?.parse().ok()?;
    let millis: i64 = if bytes.len() >= 24 && bytes[19] == b'.' {
        trigger_at_utc.get(20..23)?.parse().ok()?
    } else {
        0
    };

    let days = days_from_civil(year, month, day);
    let seconds_of_day = hour * 3600 + minute * 60 + second;
    Some(days * 86_400_000 + seconds_of_day * 1000 + millis)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    const PROFILE: &str = "profile-1";

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE reminders (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                trigger_at_utc TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'scheduled',
                created_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE sync_operations (
                operation_id TEXT PRIMARY KEY,
                profile_id TEXT NOT NULL,
                device_id TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                op TEXT NOT NULL,
                patch TEXT NOT NULL,
                hlc TEXT NOT NULL,
                schema_version INTEGER NOT NULL,
                created_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "CREATE TABLE sync_clock_state (
                id INTEGER PRIMARY KEY CHECK (id = 0),
                last_millis INTEGER NOT NULL,
                last_counter INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();
        conn
    }

    fn test_clock() -> HlcClock {
        let conn = Connection::open_in_memory().unwrap();
        HlcClock::load(&conn, "test-device".to_string()).unwrap()
    }

    fn iso_offset(delta: Duration, in_past: bool) -> String {
        let now = SystemTime::now();
        let at = if in_past { now - delta } else { now + delta };
        let millis = at.duration_since(UNIX_EPOCH).unwrap().as_millis();
        // Ручной ISO8601 вместо date crate — то же обоснование, что и у
        // parse_trigger_millis, которую эти тесты и проверяют.
        let secs = (millis / 1000) as i64;
        let days = secs.div_euclid(86_400);
        let sec_of_day = secs.rem_euclid(86_400);
        let (y, m, d) = civil_from_days(days);
        format!(
            "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
            sec_of_day / 3600,
            (sec_of_day % 3600) / 60,
            sec_of_day % 60,
            millis % 1000
        )
    }

    #[test]
    fn create_starts_scheduled() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let reminder = create(
            &mut conn,
            &mut clock,
            PROFILE,
            "Позвонить",
            "2030-01-01T10:00:00.000Z",
        )
        .unwrap();
        assert_eq!(reminder.status, "scheduled");
    }

    #[test]
    fn create_writes_a_matching_operation_log_row() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let reminder = create(
            &mut conn,
            &mut clock,
            PROFILE,
            "Позвонить",
            "2030-01-01T10:00:00.000Z",
        )
        .unwrap();

        let (op, patch): (String, String) = conn
            .query_row(
                "SELECT op, patch FROM sync_operations WHERE entity_id = ?1",
                params![reminder.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(op, "create");
        let patch: serde_json::Value = serde_json::from_str(&patch).unwrap();
        assert_eq!(patch["title"], "Позвонить");
    }

    // Атомарность: если запись в sync_operations не может пройти, вся
    // транзакция откатывается целиком — напоминание не должно появиться без
    // соответствующей записи в журнале.
    #[test]
    fn create_is_atomic_when_the_operation_log_write_fails() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE reminders (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, trigger_at_utc TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'scheduled', created_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        // sync_operations нарочно не создана.
        let mut clock = test_clock();

        assert!(create(
            &mut conn,
            &mut clock,
            PROFILE,
            "Дело",
            "2030-01-01T10:00:00.000Z"
        )
        .is_err());
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn due_returns_only_past_scheduled_reminders() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let past = iso_offset(Duration::from_secs(3600), true);
        let future = iso_offset(Duration::from_secs(3600), false);
        create(&mut conn, &mut clock, PROFILE, "Просрочено", &past).unwrap();
        let upcoming = create(&mut conn, &mut clock, PROFILE, "Пока не время", &future).unwrap();

        let due_list = due(&conn).unwrap();
        assert_eq!(due_list.len(), 1);
        assert_eq!(due_list[0].title, "Просрочено");

        acknowledge(&mut conn, &mut clock, PROFILE, &upcoming.id).unwrap();
        assert_eq!(due(&conn).unwrap().len(), 1); // ack не влияет на будущее напоминание
    }

    #[test]
    fn due_excludes_non_scheduled_status_even_if_past() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let past = iso_offset(Duration::from_secs(60), true);
        let reminder = create(&mut conn, &mut clock, PROFILE, "Дело", &past).unwrap();
        mark_firing(&conn, &reminder.id).unwrap();

        assert!(due(&conn).unwrap().is_empty());
    }

    #[test]
    fn mark_firing_and_acknowledge_update_status() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let reminder = create(
            &mut conn,
            &mut clock,
            PROFILE,
            "Дело",
            "2030-01-01T10:00:00.000Z",
        )
        .unwrap();

        mark_firing(&conn, &reminder.id).unwrap();
        let firing: String = conn
            .query_row(
                "SELECT status FROM reminders WHERE id = ?1",
                params![reminder.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(firing, "firing");

        acknowledge(&mut conn, &mut clock, PROFILE, &reminder.id).unwrap();
        let acked: String = conn
            .query_row(
                "SELECT status FROM reminders WHERE id = ?1",
                params![reminder.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(acked, "acknowledged");
    }

    // Snooze-политика: reschedule должен возвращать напоминание обратно в
    // 'scheduled' независимо от того, в каком статусе оно было (firing) —
    // иначе "отложить на 5 минут" не сработало бы после срабатывания алерта.
    #[test]
    fn reschedule_resets_firing_reminder_back_to_scheduled_with_new_time() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let reminder = create(
            &mut conn,
            &mut clock,
            PROFILE,
            "Дело",
            "2030-01-01T10:00:00.000Z",
        )
        .unwrap();
        mark_firing(&conn, &reminder.id).unwrap();

        let snoozed = reschedule(
            &mut conn,
            &mut clock,
            PROFILE,
            &reminder.id,
            "2030-01-01T10:05:00.000Z",
        )
        .unwrap();
        assert_eq!(snoozed.status, "scheduled");
        assert_eq!(snoozed.trigger_at_utc, "2030-01-01T10:05:00.000Z");
    }

    #[test]
    fn delete_removes_the_row() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let reminder = create(
            &mut conn,
            &mut clock,
            PROFILE,
            "Дело",
            "2030-01-01T10:00:00.000Z",
        )
        .unwrap();
        delete(&mut conn, &mut clock, PROFILE, &reminder.id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn parse_trigger_millis_matches_unix_epoch() {
        assert_eq!(parse_trigger_millis("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(parse_trigger_millis("1970-01-01T00:00:00.001Z"), Some(1));
        assert_eq!(
            parse_trigger_millis("1970-01-02T00:00:00.000Z"),
            Some(86_400_000)
        );
    }

    // 946684800000 — известное контрольное значение (2000-01-01T00:00:00Z в
    // мс от эпохи), проверяет days_from_civil независимо от самого алгоритма.
    #[test]
    fn parse_trigger_millis_matches_known_y2k_reference() {
        assert_eq!(
            parse_trigger_millis("2000-01-01T00:00:00.000Z"),
            Some(946_684_800_000)
        );
        assert_eq!(
            parse_trigger_millis("1999-12-31T23:59:59.999Z"),
            Some(946_684_799_999)
        );
    }

    #[test]
    fn parse_trigger_millis_handles_leap_year_day() {
        let feb29 = parse_trigger_millis("2024-02-29T00:00:00.000Z").unwrap();
        let mar1 = parse_trigger_millis("2024-03-01T00:00:00.000Z").unwrap();
        assert_eq!(mar1 - feb29, 86_400_000);
    }

    #[test]
    fn parse_trigger_millis_accepts_missing_milliseconds() {
        assert_eq!(parse_trigger_millis("1970-01-01T00:00:05Z"), Some(5000));
    }

    #[test]
    fn parse_trigger_millis_rejects_malformed_input() {
        assert_eq!(parse_trigger_millis(""), None);
        assert_eq!(parse_trigger_millis("not-a-date"), None);
        assert_eq!(parse_trigger_millis("2026-07-05T12:00:00.000"), None); // без Z
    }
}
