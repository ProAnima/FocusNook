use rusqlite::{params, Connection, Row};
use serde::Serialize;

// Раздел 8 ТЗ: Note. Пока только kind="text" — audio/transcript придут вместе
// со speech-to-text спайком (Iteration 3), это отдельный технический риск.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDto {
    pub id: String,
    pub title: Option<String>,
    pub body: String,
    pub kind: String,
}

fn row_to_dto(row: &Row) -> rusqlite::Result<NoteDto> {
    Ok(NoteDto {
        id: row.get(0)?,
        title: row.get(1)?,
        body: row.get(2)?,
        kind: row.get(3)?,
    })
}

pub fn list(conn: &Connection) -> rusqlite::Result<Vec<NoteDto>> {
    let mut stmt =
        conn.prepare("SELECT id, title, body, kind FROM notes ORDER BY created_at DESC")?;
    let notes = stmt.query_map([], row_to_dto)?.collect();
    notes
}

pub fn create(conn: &Connection, body: &str) -> rusqlite::Result<NoteDto> {
    let id = uuid::Uuid::now_v7().to_string();
    conn.execute(
        "INSERT INTO notes (id, title, body, kind, created_at)
         VALUES (?1, NULL, ?2, 'text', datetime('now'))",
        params![id, body],
    )?;
    Ok(NoteDto {
        id,
        title: None,
        body: body.to_string(),
        kind: "text".to_string(),
    })
}
