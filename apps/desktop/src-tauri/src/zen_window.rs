//! Temporary desktop geometry; normal window persistence is suspended during Zen Mode.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use tauri::{LogicalSize, WebviewWindow};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static NORMAL_SIZE: Mutex<Option<LogicalSize<f64>>> = Mutex::new(None);

pub fn is_active() -> bool {
    ACTIVE.load(Ordering::SeqCst)
}

fn apply_geometry(window: &WebviewWindow, zen: bool, size: LogicalSize<f64>) -> tauri::Result<()> {
    let position = window.outer_position()?;
    window.set_max_size(None::<LogicalSize<f64>>)?;
    window.set_min_size(Some(LogicalSize::new(
        280.0,
        if zen { 72.0 } else { 260.0 },
    )))?;
    window.set_size(size)?;
    window.set_max_size(Some(LogicalSize::new(
        520.0,
        if zen { 180.0 } else { 1200.0 },
    )))?;
    window.set_position(position)?;
    Ok(())
}

pub fn set_mode(window: &WebviewWindow, enabled: bool) -> Result<(), String> {
    let mut normal = NORMAL_SIZE
        .lock()
        .map_err(|_| "window state unavailable".to_string())?;
    if enabled == is_active() {
        return Ok(());
    }
    if enabled {
        let size = window
            .inner_size()
            .map_err(|e| e.to_string())?
            .to_logical::<f64>(window.scale_factor().map_err(|e| e.to_string())?);
        ACTIVE.store(true, Ordering::SeqCst);
        if let Err(error) = apply_geometry(window, true, LogicalSize::new(size.width, 96.0)) {
            if apply_geometry(window, false, size).is_err() {
                log::warn!("failed to restore normal window geometry after Zen transition");
            }
            ACTIVE.store(false, Ordering::SeqCst);
            return Err(error.to_string());
        }
        *normal = Some(size);
    } else if let Some(size) = *normal {
        apply_geometry(window, false, size).map_err(|e| e.to_string())?;
        ACTIVE.store(false, Ordering::SeqCst);
        *normal = None;
    }
    Ok(())
}

pub fn resize_to_content(window: &WebviewWindow, height: u32) -> Result<(), String> {
    let _normal = NORMAL_SIZE
        .lock()
        .map_err(|_| "window state unavailable".to_string())?;
    if !is_active() {
        return Ok(());
    }
    let size = window
        .inner_size()
        .map_err(|e| e.to_string())?
        .to_logical::<f64>(window.scale_factor().map_err(|e| e.to_string())?);
    let position = window.outer_position().map_err(|e| e.to_string())?;
    window
        .set_size(LogicalSize::new(
            size.width,
            f64::from(height.clamp(72, 180)),
        ))
        .map_err(|e| e.to_string())?;
    window.set_position(position).map_err(|e| e.to_string())
}
