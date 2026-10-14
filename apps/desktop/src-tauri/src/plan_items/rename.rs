use super::{fetch, PlanItemDto};
use crate::sync_log::{self, HlcClock};
use rusqlite::{params, Connection};

pub fn rename(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    id: &str,
    title: &str,
) -> rusqlite::Result<PlanItemDto> {
    let title = title.trim();
    if title.is_empty() {
        return Err(rusqlite::Error::InvalidParameterName("title".into()));
    }
    let tx = conn.transaction()?;
    fetch(&tx, id)?;
    tx.execute(
        "UPDATE plan_items SET title = ?1 WHERE id = ?2",
        params![title, id],
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "plan_item",
        id,
        "update",
        &serde_json::json!({ "title": title }),
    )?;
    let dto = fetch(&tx, id)?;
    tx.commit()?;
    Ok(dto)
}
