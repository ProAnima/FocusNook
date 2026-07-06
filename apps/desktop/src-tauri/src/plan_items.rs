use crate::sync_log::{self, HlcClock};
use rusqlite::{params, Connection, Row};
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanItemDto {
    pub id: String,
    pub title: String,
    pub status: String,
    pub progress_percent: Option<i64>,
    pub plan_date: String,
}

fn row_to_dto(row: &Row) -> rusqlite::Result<PlanItemDto> {
    Ok(PlanItemDto {
        id: row.get(0)?,
        title: row.get(1)?,
        status: row.get(2)?,
        progress_percent: row.get(3)?,
        plan_date: row.get(4)?,
    })
}

pub fn list(conn: &Connection, plan_date: &str) -> rusqlite::Result<Vec<PlanItemDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, progress_percent, plan_date
         FROM plan_items
         WHERE plan_date = ?1
         ORDER BY created_at",
    )?;
    let items = stmt
        .query_map(params![plan_date], row_to_dto)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(items)
}

pub fn list_range(
    conn: &Connection,
    start_date: &str,
    end_date: &str,
) -> rusqlite::Result<Vec<PlanItemDto>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, status, progress_percent, plan_date
         FROM plan_items
         WHERE plan_date >= ?1 AND plan_date <= ?2
         ORDER BY plan_date, created_at",
    )?;
    let items = stmt
        .query_map(params![start_date, end_date], row_to_dto)?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(items)
}

pub fn create(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    title: &str,
    plan_date: &str,
) -> rusqlite::Result<PlanItemDto> {
    let tx = conn.transaction()?;
    let id = uuid::Uuid::now_v7().to_string();
    tx.execute(
        "INSERT INTO plan_items (id, title, status, progress_percent, plan_date, created_at)
         VALUES (?1, ?2, 'open', NULL, ?3, datetime('now'))",
        params![id, title, plan_date],
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "plan_item",
        &id,
        "create",
        &serde_json::json!({
            "title": title,
            "status": "open",
            "progressPercent": null,
            "planDate": plan_date
        }),
    )?;
    tx.commit()?;
    Ok(PlanItemDto {
        id,
        title: title.to_string(),
        status: "open".to_string(),
        progress_percent: None,
        plan_date: plan_date.to_string(),
    })
}

fn fetch(conn: &Connection, id: &str) -> rusqlite::Result<PlanItemDto> {
    conn.query_row(
        "SELECT id, title, status, progress_percent, plan_date FROM plan_items WHERE id = ?1",
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
        ("partial", _) => ("done", None),
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

pub fn move_to_date(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    id: &str,
    plan_date: &str,
) -> rusqlite::Result<PlanItemDto> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE plan_items SET plan_date = ?1 WHERE id = ?2",
        params![plan_date, id],
    )?;
    let hlc = clock.next(&tx)?;
    sync_log::record_operation(
        &tx,
        &hlc,
        profile_id,
        "plan_item",
        id,
        "update",
        &serde_json::json!({ "planDate": plan_date }),
    )?;
    let dto = fetch(&tx, id)?;
    tx.commit()?;
    Ok(dto)
}

pub fn roll_over_pending(
    conn: &mut Connection,
    clock: &mut HlcClock,
    profile_id: &str,
    target_date: &str,
) -> rusqlite::Result<usize> {
    let tx = conn.transaction()?;
    let ids = {
        let mut stmt = tx.prepare(
            "SELECT id FROM plan_items
             WHERE plan_date < ?1 AND status != 'done'
             ORDER BY plan_date, created_at",
        )?;
        let rows = stmt
            .query_map(params![target_date], |row| row.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows
    };

    for id in &ids {
        tx.execute(
            "UPDATE plan_items SET plan_date = ?1 WHERE id = ?2",
            params![target_date, id],
        )?;
        let hlc = clock.next(&tx)?;
        sync_log::record_operation(
            &tx,
            &hlc,
            profile_id,
            "plan_item",
            id,
            "update",
            &serde_json::json!({ "planDate": target_date, "rolledOver": true }),
        )?;
    }

    let moved = ids.len();
    tx.commit()?;
    Ok(moved)
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
    const TODAY: &str = "2026-07-06";
    const TOMORROW: &str = "2026-07-07";

    fn setup_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE plan_items (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                status TEXT NOT NULL,
                progress_percent INTEGER,
                plan_date TEXT NOT NULL,
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

    fn operation_count(conn: &Connection) -> i64 {
        conn.query_row("SELECT COUNT(*) FROM sync_operations", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn create_starts_open_with_date_and_no_progress() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Task", TODAY).unwrap();
        assert_eq!(item.status, "open");
        assert_eq!(item.progress_percent, None);
        assert_eq!(item.plan_date, TODAY);
    }

    #[test]
    fn list_filters_by_plan_date() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        create(&mut conn, &mut clock, PROFILE, "Today", TODAY).unwrap();
        create(&mut conn, &mut clock, PROFILE, "Tomorrow", TOMORROW).unwrap();

        let today = list(&conn, TODAY).unwrap();
        assert_eq!(today.len(), 1);
        assert_eq!(today[0].title, "Today");
    }

    #[test]
    fn list_range_returns_visible_calendar_items() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        create(&mut conn, &mut clock, PROFILE, "Today", TODAY).unwrap();
        create(&mut conn, &mut clock, PROFILE, "Tomorrow", TOMORROW).unwrap();

        let items = list_range(&conn, TODAY, TOMORROW).unwrap();
        assert_eq!(items.len(), 2);
    }

    #[test]
    fn create_writes_a_matching_operation_log_row() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Task", TODAY).unwrap();

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
        assert_eq!(patch["title"], "Task");
        assert_eq!(patch["status"], "open");
        assert_eq!(patch["planDate"], TODAY);
    }

    #[test]
    fn create_is_atomic_when_the_operation_log_write_fails() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE plan_items (
                id TEXT PRIMARY KEY, title TEXT NOT NULL, status TEXT NOT NULL,
                progress_percent INTEGER, plan_date TEXT NOT NULL, created_at TEXT NOT NULL
            )",
            [],
        )
        .unwrap();
        let mut clock = test_clock();

        assert!(create(&mut conn, &mut clock, PROFILE, "Task", TODAY).is_err());
        assert!(list(&conn, TODAY).unwrap().is_empty());
    }

    #[test]
    fn toggle_done_is_a_round_trip() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Task", TODAY).unwrap();
        let done = toggle_done(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(done.status, "done");
        let open = toggle_done(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(open.status, "open");
        assert_eq!(operation_count(&conn), 3);
    }

    #[test]
    fn toggle_done_from_partial_clears_progress() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Task", TODAY).unwrap();
        cycle_progress(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        let done = toggle_done(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(done.status, "done");
        assert_eq!(done.progress_percent, None);
    }

    #[test]
    fn cycle_progress_steps_through_25_50_75_then_done() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Task", TODAY).unwrap();

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
            ("done", None)
        );
    }

    #[test]
    fn cycle_progress_from_deferred_jumps_straight_to_partial_25() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Task", TODAY).unwrap();
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
        let item = create(&mut conn, &mut clock, PROFILE, "Task", TODAY).unwrap();
        cycle_progress(&mut conn, &mut clock, PROFILE, &item.id).unwrap();

        let deferred = toggle_deferred(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(deferred.status, "deferred");
        assert_eq!(deferred.progress_percent, None);

        let open = toggle_deferred(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert_eq!(open.status, "open");
    }

    #[test]
    fn move_to_date_updates_date_and_logs_patch() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Task", TODAY).unwrap();
        let moved = move_to_date(&mut conn, &mut clock, PROFILE, &item.id, TOMORROW).unwrap();
        assert_eq!(moved.plan_date, TOMORROW);
        assert!(list(&conn, TODAY).unwrap().is_empty());
        assert_eq!(list(&conn, TOMORROW).unwrap().len(), 1);

        let patch: String = conn
            .query_row(
                "SELECT patch FROM sync_operations WHERE entity_id = ?1 ORDER BY hlc DESC LIMIT 1",
                params![item.id],
                |r| r.get(0),
            )
            .unwrap();
        let patch: serde_json::Value = serde_json::from_str(&patch).unwrap();
        assert_eq!(patch["planDate"], TOMORROW);
    }

    #[test]
    fn roll_over_pending_moves_only_unfinished_past_items() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let open = create(&mut conn, &mut clock, PROFILE, "Open", TODAY).unwrap();
        let done = create(&mut conn, &mut clock, PROFILE, "Done", TODAY).unwrap();
        toggle_done(&mut conn, &mut clock, PROFILE, &done.id).unwrap();

        let moved = roll_over_pending(&mut conn, &mut clock, PROFILE, TOMORROW).unwrap();

        assert_eq!(moved, 1);
        assert_eq!(fetch(&conn, &open.id).unwrap().plan_date, TOMORROW);
        assert_eq!(fetch(&conn, &done.id).unwrap().plan_date, TODAY);
    }

    #[test]
    fn delete_removes_the_row_and_logs_a_tombstone() {
        let mut conn = setup_conn();
        let mut clock = test_clock();
        let item = create(&mut conn, &mut clock, PROFILE, "Task", TODAY).unwrap();
        delete(&mut conn, &mut clock, PROFILE, &item.id).unwrap();
        assert!(list(&conn, TODAY).unwrap().is_empty());

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
