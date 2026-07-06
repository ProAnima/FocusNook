use crate::sync_log::{self, HlcClock};
use rusqlite::{params, Connection, Row};
use serde::Serialize;

// Раздел 8 ТЗ: PlanItem.
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
    let mut stmt = conn.prepare(
        "SELECT id, title, status, progress_percent FROM plan_items ORDER BY created_at",
    )?;
    let items = stmt.query_map([], row_to_dto)?.collect();
    items
}

// Раздел 9 ТЗ, Iteration 2 (первый локальный шаг): каждая мутация пишет и
// саму строку, и запись в sync_operations одной транзакцией — если что-то не
// закоммитится, не закоммитится ничего. &mut Connection вместо &Connection —
// Connection::transaction(&mut self) требует мутабельного заимствования.
pub fn create(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    title: &str,
) -> rusqlite::Result<PlanItemDto> {
    let tx = conn.transaction()?;
    let id = uuid::Uuid::now_v7().to_string();
    tx.execute(
        "INSERT INTO plan_items (id, title, status, progress_percent, created_at)
         VALUES (?1, ?2, 'open', NULL, datetime('now'))",
        params![id, title],
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "plan_item",
        &id,
        "create",
        &serde_json::json!({ "title": title, "status": "open", "progressPercent": null }),
    )?;
    tx.commit()?;
    Ok(PlanItemDto {
        id,
        title: title.to_string(),
        status: "open".to_string(),
        progress_percent: None,
    })
}

fn fetch(conn: &Connection, id: &str) -> rusqlite::Result<PlanItemDto> {
    conn.query_row(
        "SELECT id, title, status, progress_percent FROM plan_items WHERE id = ?1",
        params![id],
        row_to_dto,
    )
}

pub fn toggle_done(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    id: &str,
) -> rusqlite::Result<PlanItemDto> {
    let tx = conn.transaction()?;
    let current: String = tx.query_row(
        "SELECT status FROM plan_items WHERE id = ?1",
        params![id],
        |r| r.get(0),
    )?;
    let next = if current == "done" { "open" } else { "done" };
    tx.execute(
        "UPDATE plan_items SET status = ?1, progress_percent = NULL WHERE id = ?2",
        params![next, id],
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "plan_item",
        id,
        "update",
        &serde_json::json!({ "status": next }),
    )?;
    let dto = fetch(&tx, id)?;
    tx.commit()?;
    Ok(dto)
}

// Раздел 12 ТЗ: "progress для partial: маленький slider/stepper, а не
// отдельная форма" — реализовано как степпер по клику: open -> 25% -> 50% ->
// 75% -> обратно open. Прогрессия — бизнес-правило, поэтому решение о
// следующем шаге принимает Rust, а не фронт (фронт просто дергает "следующий шаг").
pub fn cycle_progress(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    id: &str,
) -> rusqlite::Result<PlanItemDto> {
    let tx = conn.transaction()?;
    let (status, progress): (String, Option<i64>) = tx.query_row(
        "SELECT status, progress_percent FROM plan_items WHERE id = ?1",
        params![id],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let (next_status, next_progress) = match (status.as_str(), progress) {
        ("partial", Some(p)) if p < 75 => ("partial", Some(p + 25)),
        ("partial", _) => ("open", None),
        _ => ("partial", Some(25)),
    };
    tx.execute(
        "UPDATE plan_items SET status = ?1, progress_percent = ?2 WHERE id = ?3",
        params![next_status, next_progress, id],
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "plan_item",
        id,
        "update",
        &serde_json::json!({ "status": next_status, "progressPercent": next_progress }),
    )?;
    let dto = fetch(&tx, id)?;
    tx.commit()?;
    Ok(dto)
}

pub fn toggle_deferred(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    id: &str,
) -> rusqlite::Result<PlanItemDto> {
    let tx = conn.transaction()?;
    let current: String = tx.query_row(
        "SELECT status FROM plan_items WHERE id = ?1",
        params![id],
        |r| r.get(0),
    )?;
    let next = if current == "deferred" {
        "open"
    } else {
        "deferred"
    };
    tx.execute(
        "UPDATE plan_items SET status = ?1, progress_percent = NULL WHERE id = ?2",
        params![next, id],
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "plan_item",
        id,
        "update",
        &serde_json::json!({ "status": next }),
    )?;
    let dto = fetch(&tx, id)?;
    tx.commit()?;
    Ok(dto)
}

pub fn delete(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    id: &str,
) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM plan_items WHERE id = ?1", params![id])?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "plan_item",
        id,
        "delete",
        &serde_json::json!({}),
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    const PROFILE: &str = "profile-1";

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE plan_items (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                progress_percent INTEGER,
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

    // Отдельное in-memory соединение без таблиц — HlcClock::load своим
    // unwrap_or((0,0)) не различает "таблицы нет" и "строки нет", так что для
    // одноразового тестового счётчика это не хуже честного пустого состояния.
    fn test_clock() -> HlcClock {
        let conn = Connection::open_in_memory().unwrap();
        HlcClock::load(&conn, "test-device".to_string()).unwrap()
    }

    fn operation_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM sync_operations", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn create_starts_open_with_no_progress() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Дело").unwrap();
        assert_eq!(item.status, "open");
        assert_eq!(item.progress_percent, None);
    }

    #[test]
    fn create_writes_a_matching_operation_log_row() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Дело").unwrap();

        let (entity_type, op, patch): (String, String, String) = conn
            .query_row(
                "SELECT entity_type, op, patch FROM sync_operations WHERE entity_id = ?1",
                params![item.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(entity_type, "plan_item");
        assert_eq!(op, "create");
        let patch: serde_json::Value = serde_json::from_str(&patch).unwrap();
        assert_eq!(patch["title"], "Дело");
        assert_eq!(patch["status"], "open");
    }

    // Атомарность: если запись в sync_operations не может пройти (здесь —
    // таблицы попросту нет), вся транзакция должна откатиться целиком, а не
    // оставить дело созданным без записи в журнале.
    #[test]
    fn create_is_atomic_when_the_operation_log_write_fails() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE plan_items (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, status TEXT NOT NULL,
                progress_percent INTEGER, created_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        // sync_operations нарочно не создана.
        let mut clock = test_clock();

        assert!(create(&mut conn, &mut clock, PROFILE, "Дело").is_err());
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn toggle_done_is_a_round_trip() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Дело").unwrap();
        let done = toggle_done(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(done.status, "done");
        let open = toggle_done(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(open.status, "open");
        assert_eq!(operation_count(&conn), 3); // create + 2 toggle
    }

    #[test]
    fn toggle_done_from_partial_clears_progress() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Дело").unwrap();
        cycle_progress(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        let done = toggle_done(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(done.status, "done");
        assert_eq!(done.progress_percent, None);
    }

    // Раздел 12 ТЗ: степпер open -> 25 -> 50 -> 75 -> обратно open — сама
    // прогрессия это бизнес-правило в Rust, а не что-то, что решает фронт.
    #[test]
    fn cycle_progress_steps_through_25_50_75_then_back_to_open() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Дело").unwrap();

        let step1 = cycle_progress(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(
            (step1.status.as_str(), step1.progress_percent),
            ("partial", Some(25))
        );

        let step2 = cycle_progress(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(
            (step2.status.as_str(), step2.progress_percent),
            ("partial", Some(50))
        );

        let step3 = cycle_progress(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(
            (step3.status.as_str(), step3.progress_percent),
            ("partial", Some(75))
        );

        let step4 = cycle_progress(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(
            (step4.status.as_str(), step4.progress_percent),
            ("open", None)
        );
    }

    #[test]
    fn cycle_progress_from_deferred_jumps_straight_to_partial_25() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Дело").unwrap();
        toggle_deferred(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        let result = cycle_progress(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(
            (result.status.as_str(), result.progress_percent),
            ("partial", Some(25))
        );
    }

    #[test]
    fn toggle_deferred_is_a_round_trip_and_clears_progress() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Дело").unwrap();
        cycle_progress(&mut conn, &mut clock, PROFILE, &item.id).unwrap();

        let deferred = toggle_deferred(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(deferred.status, "deferred");
        assert_eq!(deferred.progress_percent, None);

        let open = toggle_deferred(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(open.status, "open");
    }

    #[test]
    fn delete_removes_the_row_and_logs_a_tombstone() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Дело").unwrap();
        delete(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert!(list(&conn).unwrap().is_empty());

        // ORDER BY hlc, не created_at: datetime('now') в SQLite — разрешение
        // до секунды, create/delete в этом тесте вполне попадают в одну и ту
        // же секунду. hlc монотонен даже внутри одной секунды — в этом и есть
        // его смысл.
        let op: String = conn
            .query_row(
                "SELECT op FROM sync_operations WHERE entity_id = ?1 ORDER BY hlc DESC LIMIT 1",
                params![item.id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(op, "delete");
    }
}
