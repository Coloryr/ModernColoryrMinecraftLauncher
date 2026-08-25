//! 窗口管理器
//!
//! 所有窗口的创建、聚焦、关闭统一在这里处理：
//! - 主窗口在 `setup` 阶段通过 [`create_main`] 创建（并恢复上次几何）
//! - 功能窗口通过 [`create_window`] 创建或聚焦；`open_window` / `close_window`
//!   命令供前端调用（多窗口模式）
//! - 窗口几何状态（`window_save.json`）与 GUI 配置（`gui_config.json`）也在此维护，
//!   主窗口使用固定 [`MAIN_WINDOW_UUID`]。

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{LazyLock, OnceLock, RwLock},
};

use mcml_base::serialize_tools;
use mcml_config::config_save;
use mcml_names::{names, uuids};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder};
use uuid::{Uuid, uuid};

use crate::gui_config::GuiConfig;

/// 窗口几何状态（window_save.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowState {
    pub uuid: String,
    pub label: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            uuid: String::new(),
            label: String::new(),
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

/// 主窗口固定 UUID
pub const MAIN_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
/// 主窗口标签
pub const MAIN_WINDOW_LABEL: &str = "main";

/// 窗口几何状态（uuid → 几何），内存中的唯一数据源
static WINDOWS: LazyLock<RwLock<HashMap<Uuid, WindowState>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 窗口状态文件路径（window_save.json）
static FILE: OnceLock<PathBuf> = OnceLock::new();

/// 主窗口句柄（创建后记录，供关闭 / 判断使用）
static MAIN_WINDOW: OnceLock<WebviewWindow<tauri::Wry>> = OnceLock::new();

/// 读取窗口状态文件（启动时调用，位于运行路径下）
pub fn init<P: AsRef<Path>>(path: P) {
    let file = FILE.get_or_init(|| path.as_ref().join(names::WINDOW_SAVE_FILE));

    if file.exists() && file.is_file() {
        if let Ok(data) = serialize_tools::json_from_file::<HashMap<Uuid, WindowState>>(file) {
            WINDOWS.write().unwrap().extend(data);
        }
    }
}

/// 保存窗口几何（异步写入 window_save.json）
pub fn save() {
    let Some(file) = FILE.get() else {
        return;
    };
    let data = WINDOWS.read().unwrap();
    config_save::save(uuids::WINDOW_FILE_UUID, &*data, file);
}

/// 读取指定 uuid 的窗口几何（用于启动时恢复位置大小）
pub fn window_state_for(uuid: Uuid) -> Option<WindowState> {
    WINDOWS.read().unwrap().get(&uuid).cloned()
}

/// 主窗口句柄（若已创建）
pub fn main_window() -> Option<&'static WebviewWindow<tauri::Wry>> {
    MAIN_WINDOW.get()
}

/// 创建（或聚焦）一个窗口
///
/// 注意：必须由 async 命令调用（同步命令在 Windows 主线程阻塞创建会冻结应用）。
pub fn create_window(
    app: &AppHandle,
    label: &str,
    title: &str,
    width: f64,
    height: f64,
) -> Result<(), String> {
    // 已存在则聚焦，避免重复窗口
    if let Some(win) = app.get_webview_window(label) {
        let _ = win.set_focus();
        return Ok(());
    }
    WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(title)
        .inner_size(width, height)
        .build()
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 创建主窗口（有上次几何则恢复位置/大小，否则按默认居中显示）；已存在则聚焦
pub fn create_main(app: &AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let _ = win.set_focus();
        return Ok(());
    }
    let geom = window_state_for(MAIN_WINDOW_UUID);
    let builder =
        WebviewWindowBuilder::new(app, MAIN_WINDOW_LABEL, WebviewUrl::App("index.html".into()))
            .title("MCML 启动器")
            .min_inner_size(900.0, 600.0);
    let win = match geom {
        Some(g) => builder
            .inner_size(g.width as f64, g.height as f64)
            .position(g.x as f64, g.y as f64)
            .build(),
        None => builder.inner_size(1100.0, 720.0).center().build(),
    };
    let win = win.map_err(|e| e.to_string())?;
    let _ = MAIN_WINDOW.set(win);
    Ok(())
}

/// 按标签关闭窗口
pub fn close_label(app: &AppHandle, label: &str) -> Result<(), String> {
    if let Some(win) = app.get_webview_window(label) {
        win.close().map_err(|e| e.to_string())?;
    }
    Ok(())
}

// ================= IPC 命令 =================

/// 打开一个功能窗口（多窗口模式，窗口按钮调用）
///
/// 注意：必须保持 async：同步命令在 Windows 上跑在主线程，而窗口创建会阻塞
/// 等待主线程，导致整个应用冻结（新窗口白屏、无法点击）。
#[tauri::command]
pub async fn window_open_window(app: AppHandle, kind: String) -> Result<(), String> {
    println!("[window_manager] 打开窗口 kind={kind}");
    match kind.as_str() {
        "main" => crate::windows::main::open(&app),
        "settings" => crate::windows::settings::open(&app),
        "stats" => crate::windows::stats::open(&app),
        "skin" => crate::windows::skin::open(&app),
        "help" => crate::windows::help::open(&app),
        "resource" => crate::windows::resource::open(&app),
        "account" => crate::windows::account::open(&app),
        "add" => crate::windows::add::open(&app),
        _ => Err(format!("未知窗口类型: {kind}")),
    }
}

/// 关闭窗口（kind → 标签；main 关闭主窗口）
#[tauri::command]
pub fn window_close_window(app: AppHandle, kind: String) -> Result<(), String> {
    let label = match kind.as_str() {
        "main" => MAIN_WINDOW_LABEL.to_string(),
        _ => format!("mcml-{kind}"),
    };
    close_label(&app, &label)
}

/// 获取 GUI 状态（无文件时返回默认值）
#[tauri::command]
pub fn window_get_gui_config() -> GuiConfig {
    crate::gui_config::get()
}

/// 保存 GUI 状态到 gui_config.json
#[tauri::command]
pub fn window_save_gui_config(config: GuiConfig) -> Result<(), String> {
    crate::gui_config::set(config);
    Ok(())
}

/// 获取全部窗口几何状态
#[tauri::command]
pub fn window_get_window_states() -> Vec<WindowState> {
    WINDOWS.read().unwrap().values().cloned().collect()
}

/// 保存（或更新）某个窗口的几何状态，按 uuid 去重
#[tauri::command]
pub fn window_save_window_state(state: WindowState) -> Result<(), String> {
    let uuid = Uuid::parse_str(&state.uuid).map_err(|e| e.to_string())?;
    WINDOWS.write().unwrap().insert(uuid, state);
    save();
    Ok(())
}
