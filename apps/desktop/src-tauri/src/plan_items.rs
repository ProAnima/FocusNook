use rusqlite::{params, Connection, Row};
use serde::Serialize;

// Раздел 8 ТЗ: PlanItem. Пока реализованы только open/done — partial/deferred
// появятся вместе с их UI (progress-слайдер, "отложить"), см. AGENTS.md.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItemDto {
    pub id: String,
    pub title: String,
    pub status: String,
    pub progress_percent: Option<i64>,
}

fn row_to_dto(row: &Row) -> rusqlite::Result<PlanItemDto> {
    Ok(PlanItemDto {
        id: row.get(0)?,
        title: row.get(1)?,
        status: row.get(2)?,
        progress_percent: row.get(3)?,
    })
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<PlanItemDto>> {
    let mut stmt = conn
        .prepare("SELECT id, title, status, progress_percent FROM plan_items ORDER BY created_at")?;
    let items = stmt.query_map([], row_to_dto)?.collect();
    items
}

pub fn create(conn: &Connection, title: &str) -> rusqlite::Result<PlanItemDto> {
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO plan_items (id, title, status, progress_percent, created_at)
         VALUES (?1, ?2, 'open', NULL, datetime('now'))",
        params![id, title],
    )?;
    Ok(PlanItemDto {
        id,
        title: title.to_string(),
        status: "open".to_string(),
        progress_percent: None,
    })
}

pub fn toggle_done(conn: &Connection, id: &str) -> rusqlite::Result<PlanItemDto> {
    let current: String =
        conn.query_row("SELECT status FROM plan_items WHERE id = ?1", params![id], |r| {
            r.get(0)
        })?;
    let next = if current == "done" { "open" } else { "done" };
    conn.execute("UPDATE plan_items SET status = ?1 WHERE id = ?2", params![next, id])?;
    conn.query_row(
        "SELECT id, title, status, progress_percent FROM plan_items WHERE id = ?1",
        params![id],
        row_to_dto,
    )
}
