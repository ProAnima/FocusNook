use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tauri::{Manager, PhysicalPosition, PhysicalSize};

const WINDOW_STATE_FILENAME: &str = "window-state.json";
const MIN_WIDTH: u32 = 648;
const MIN_HEIGHT: u32 = 392;
const MAX_WIDTH: u32 = 900;
const MAX_HEIGHT: u32 = 1200;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct MainWindowState {
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

fn load_state(data_dir: &Path) -> Option<MainWindowState> {
    let raw = fs::read_to_string(state_path(data_dir)).ok()?;
    serde_json::from_str(&raw).ok()
}

fn write_state(data_dir: &Path, state: &MainWindowState) -> Result<(), String> {
    let (width, height) = clamp_size(state.width, state.height);
    let normalized = MainWindowState {
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

    let (width, height) = clamp_size(state.width, state.height);
    let _ = window.set_size(PhysicalSize::new(width, height));
    if let (Some(x), Some(y)) = (state.x, state.y) {
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
}

pub fn save(window: &tauri::WebviewWindow, data_dir: &Path) -> Result<(), String> {
    if window.label() != "main" {
        return Ok(());
    }
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let position = window.outer_position().ok();
    write_state(
        data_dir,
        &MainWindowState {
            width: size.width,
            height: size.height,
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
    fn writes_and_reads_local_window_state() -> Result<(), String> {
        let dir =
            std::env::temp_dir().join(format!("focusnook-window-state-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        write_state(
            &dir,
            &MainWindowState {
                width: 420,
                height: 640,
                x: Some(24),
                y: Some(48),
            },
        )?;

        assert_eq!(
            load_state(&dir),
            Some(MainWindowState {
                width: 648,
                height: 640,
                x: Some(24),
                y: Some(48),
            })
        );

        fs::remove_dir_all(&dir).map_err(|e| e.to_string())
    }
}
