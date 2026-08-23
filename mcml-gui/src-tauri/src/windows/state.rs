//! 窗口状态 / 应用配置
//!
//! - `gui_config.json`：GUI 状态（主题 / 语言 / 窗口模式 / 侧栏位置等），由 Rust 提供读写
//! - `windows.json`：每个窗口的 uuid → 位置 / 大小（几何状态）
//! 主窗口使用固定 uuid，其余窗口首次保存时分配。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

fn data_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {e}"))?;
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn gui_config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("gui_config.json"))
}

fn windows_state_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(data_dir(app)?.join("windows.json"))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &PathBuf) -> Option<T> {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok())
}

fn write_json<T: Serialize>(path: &PathBuf, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

/// 获取 GUI 状态（无文件时返回默认值）
#[tauri::command]
pub fn get_gui_config(app: AppHandle) -> GuiConfig {
    gui_config_path(&app)
        .ok()
        .and_then(|p| read_json(&p))
        .unwrap_or_default()
}

/// 保存 GUI 状态到 gui_config.json
#[tauri::command]
pub fn save_gui_config(app: AppHandle, config: GuiConfig) -> Result<(), String> {
    let path = gui_config_path(&app)?;
    write_json(&path, &config)
}

/// 获取全部窗口几何状态
#[tauri::command]
pub fn get_window_states(app: AppHandle) -> Vec<WindowState> {
    windows_state_path(&app)
        .ok()
        .and_then(|p| read_json(&p))
        .unwrap_or_default()
}

/// 保存（或更新）某个窗口的几何状态，按 uuid 去重
#[tauri::command]
pub fn save_window_state(app: AppHandle, state: WindowState) -> Result<(), String> {
    let path = windows_state_path(&app)?;
    let mut list: Vec<WindowState> = read_json(&path).unwrap_or_default();
    if let Some(existing) = list.iter_mut().find(|w| w.uuid == state.uuid) {
        *existing = state;
    } else {
        list.push(state);
    }
    write_json(&path, &list)
}

/// 主窗口固定 uuid
#[tauri::command]
pub fn get_main_window_uuid() -> String {
    MAIN_WINDOW_UUID.to_string()
}

/// 读取指定 uuid 的窗口几何（用于启动时恢复位置大小）
pub fn window_state_for(app: &AppHandle, uuid: &str) -> Option<WindowState> {
    windows_state_path(app)
        .ok()
        .and_then(|p| read_json(&p))
        .and_then(|list: Vec<WindowState>| list.into_iter().find(|w| w.uuid == uuid))
}
