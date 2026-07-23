use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::{LogicalSize, Manager, PhysicalPosition};

const WINDOW_STATE_FILENAME: &str = "window-state.json";
const LOGICAL_SIZE_STATE_VERSION: u8 = 2;
const MIN_WIDTH: u32 = 648;
const MIN_HEIGHT: u32 = 392;
const MAX_WIDTH: u32 = 900;
const MAX_HEIGHT: u32 = 1200;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct MainWindowState {
    #[serde(default)]
    version: u8,
    width: u32,
    height: u32,
    x: Option<i32>,
    y: Option<i32>,
}

fn state_path(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join(WINDOW_STATE_FILENAME)
}

fn clamp_size(width: u32, height: u32) -> (u32, u32) {
    (
        width.clamp(MIN_WIDTH, MAX_WIDTH),
        height.clamp(MIN_HEIGHT, MAX_HEIGHT),
    )
}

fn usable_scale_factor(scale_factor: f64) -> f64 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor
    } else {
        1.0
    }
}

fn restored_size(state: &MainWindowState, scale_factor: f64) -> (u32, u32) {
    if state.version >= LOGICAL_SIZE_STATE_VERSION {
        return clamp_size(state.width, state.height);
    }

    // v1 stored outer_size() verbatim, which is physical pixels. Reinterpret it
    // once using the current monitor scale so a 648 px window does not become
    // only 432 CSS px at 150% Windows scaling.
    let scale = usable_scale_factor(scale_factor);
    let width = (f64::from(state.width) / scale).round() as u32;
    let height = (f64::from(state.height) / scale).round() as u32;
    clamp_size(width, height)
}

fn load_state(data_dir: &Path) -> Option<MainWindowState> {
    let raw = fs::read_to_string(state_path(data_dir)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn apply_size(window: &tauri::WebviewWindow, state: &MainWindowState, scale_factor: f64) {
    let (width, height) = restored_size(state, scale_factor);
    // During a cross-monitor move the window dispatcher can briefly retain
    // the previous monitor's scale. Convert with the event's new scale here.
    let physical_size =
        LogicalSize::new(width, height).to_physical::<u32>(usable_scale_factor(scale_factor));
    let _ = window.set_size(physical_size);
}

fn write_state(data_dir: &Path, state: &MainWindowState) -> Result<(), String> {
    let (width, height) = clamp_size(state.width, state.height);
    let normalized = MainWindowState {
        version: state.version,
        width,
        height,
        x: state.x,
        y: state.y,
    };
    let raw = serde_json::to_string_pretty(&normalized).map_err(|e| e.to_string())?;
    fs::write(state_path(data_dir), raw).map_err(|e| e.to_string())
}

pub fn apply(app: &tauri::AppHandle, data_dir: &Path) {
    let Some(state) = load_state(data_dir) else {
        return;
    };
    let Some(window) = app.get_webview_window("main") else {
        return;
    };

    // Move first so Windows can switch the window to the target monitor's DPI
    // before the logical size is restored.
    if let (Some(x), Some(y)) = (state.x, state.y) {
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
    let scale_factor = window.scale_factor().unwrap_or(1.0);
    apply_size(&window, &state, scale_factor);
}

pub fn reapply_size_after_scale_change(
    window: &tauri::WebviewWindow,
    data_dir: &Path,
    scale_factor: f64,
) {
    let Some(state) = load_state(data_dir) else {
        return;
    };
    apply_size(window, &state, scale_factor);
}

pub fn save(window: &tauri::WebviewWindow, data_dir: &Path) -> Result<(), String> {
    if window.label() != "main" {
        return Ok(());
    }
    let physical_size = window.outer_size().map_err(|e| e.to_string())?;
    let scale_factor = window.scale_factor().map_err(|e| e.to_string())?;
    let size = physical_size.to_logical::<f64>(scale_factor);
    let position = window.outer_position().ok();
    write_state(
        data_dir,
        &MainWindowState {
            version: LOGICAL_SIZE_STATE_VERSION,
            width: size.width.round() as u32,
            height: size.height.round() as u32,
            x: position.map(|value| value.x),
            y: position.map(|value| value.y),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_tiny_or_huge_sizes() {
        assert_eq!(clamp_size(10, 10), (648, MIN_HEIGHT));
        assert_eq!(clamp_size(2000, 2000), (MAX_WIDTH, MAX_HEIGHT));
    }

    #[test]
    fn migrates_legacy_physical_size_at_scaled_dpi() {
        let state = MainWindowState {
            version: 0,
            width: 648,
            height: 809,
            x: None,
            y: None,
        };

        assert_eq!(restored_size(&state, 1.5), (MIN_WIDTH, 539));
    }

    #[test]
    fn keeps_current_logical_size_at_scaled_dpi() {
        let state = MainWindowState {
            version: LOGICAL_SIZE_STATE_VERSION,
            width: 720,
            height: 640,
            x: None,
            y: None,
        };

        assert_eq!(restored_size(&state, 2.0), (720, 640));
    }

    #[test]
    fn writes_and_reads_local_window_state() -> Result<(), String> {
        let dir =
            std::env::temp_dir().join(format!("focusnook-window-state-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        write_state(
            &dir,
            &MainWindowState {
                version: LOGICAL_SIZE_STATE_VERSION,
                width: 420,
                height: 640,
                x: Some(24),
                y: Some(48),
            },
        )?;

        assert_eq!(
            load_state(&dir),
            Some(MainWindowState {
                version: LOGICAL_SIZE_STATE_VERSION,
                width: 648,
                height: 640,
                x: Some(24),
                y: Some(48),
            })
        );

        fs::remove_dir_all(&dir).map_err(|e| e.to_string())
    }
}
