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

pub fn create(conn: &Connection, title: &str, trigger_at_utc: &str) -> rusqlite::Result<ReminderDto> {
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO reminders (id, title, trigger_at_utc, status, created_at)
         VALUES (?1, ?2, ?3, 'scheduled', datetime('now'))",
        params![id, title, trigger_at_utc],
    )?;
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

pub fn mark_firing(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE reminders SET status = 'firing' WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn acknowledge(conn: &Connection, id: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE reminders SET status = 'acknowledged' WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn reschedule(conn: &Connection, id: &str, new_trigger_at_utc: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE reminders SET status = 'scheduled', trigger_at_utc = ?1 WHERE id = ?2",
        params![new_trigger_at_utc, id],
    )?;
    Ok(())
}
