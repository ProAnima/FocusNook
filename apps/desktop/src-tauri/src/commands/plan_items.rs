use super::trigger_server_sync;
use crate::{db, plan_items, profiles, sync_log};

#[tauri::command]
pub(crate) fn list_plan_items(
    db: tauri::State<db::Db>,
    plan_date: String,
) -> Result<Vec<plan_items::PlanItemDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    plan_items::list(&conn, &plan_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn list_plan_items_range(
    db: tauri::State<db::Db>,
    start_date: String,
    end_date: String,
) -> Result<Vec<plan_items::PlanItemDto>, String> {
    let conn = db.0.lock().map_err(|e| e.to_string())?;
    plan_items::list_range(&conn, &start_date, &end_date).map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn create_plan_item(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    title: String,
    plan_date: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::create(&mut conn, &mut clock, &profile_id, &title, &plan_date)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
pub(crate) fn toggle_plan_item_done(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::toggle_done(&mut conn, &mut clock, &profile_id, &id)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
pub(crate) fn cycle_plan_item_progress(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::cycle_progress(&mut conn, &mut clock, &profile_id, &id)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
pub(crate) fn toggle_plan_item_deferred(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::toggle_deferred(&mut conn, &mut clock, &profile_id, &id)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
pub(crate) fn toggle_plan_item_long_running(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::toggle_long_running(&mut conn, &mut clock, &profile_id, &id)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
pub(crate) fn move_plan_item_to_date(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
    plan_date: String,
) -> Result<plan_items::PlanItemDto, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let item = plan_items::move_to_date(&mut conn, &mut clock, &profile_id, &id, &plan_date)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(item)
}

#[tauri::command]
pub(crate) fn roll_over_pending_plan_items(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    target_date: String,
) -> Result<usize, String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    let moved = plan_items::roll_over_pending(&mut conn, &mut clock, &profile_id, &target_date)
        .map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    if moved > 0 {
        trigger_server_sync(&app);
    }
    Ok(moved)
}

#[tauri::command]
pub(crate) fn delete_plan_item(
    app: tauri::AppHandle,
    db: tauri::State<db::Db>,
    hlc_state: tauri::State<sync_log::HlcClockState>,
    profiles_state: tauri::State<profiles::ProfilesState>,
    id: String,
) -> Result<(), String> {
    let mut conn = db.0.lock().map_err(|e| e.to_string())?;
    let mut clock = hlc_state.0.lock().map_err(|e| e.to_string())?;
    let profile_id = profiles::active_profile_id(&profiles_state)?;
    plan_items::delete(&mut conn, &mut clock, &profile_id, &id).map_err(|e| e.to_string())?;
    drop(clock);
    drop(conn);
    trigger_server_sync(&app);
    Ok(())
}
