//! 窗口管理器
//!
//! 所有窗口的创建、聚焦、关闭统一在这里处理：
//! - 主窗口在 `setup` 阶段通过 [`create_main`] 创建（并恢复上次几何）
//! - 功能窗口通过 [`create_window`] 创建或聚焦；`open_window` / `close_window`
//!   命令供前端调用（多窗口模式）
//! - 窗口几何状态（`window_save.json`）与 GUI 配置（`gui_config.json`）也在此维护，

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    str::FromStr,
    sync::{LazyLock, OnceLock, RwLock},
};

use mcml_base::serialize_tools;
use mcml_config::config_save;
use mcml_names::{names, uuids};
use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Error::WindowNotFound, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};
use uuid::{Uuid, uuid};

use crate::dtos::GuiConfigDto;

/// 窗口几何状态（window_save.json）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}

pub enum WindowType {
    MainWindow(String),
    AccountWindow(String),
}

const MAIN_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");

const WINDOWS_INFO: LazyLock<HashMap<Uuid, WindowType>> = LazyLock::new(|| {
    let mut map = HashMap::new();

    map.insert(
        MAIN_WINDOW_UUID,
        WindowType::MainWindow(String::from("mcml-main")),
    );
    map.insert(
        uuid!("00000000-0000-0000-0000-000000000002"),
        WindowType::AccountWindow(String::from("mcml-account")),
    );

    map
});

const WINDOW_MIN_WIDTH: f64 = 900.0;
const WINDOW_MIN_HEIGHT: f64 = 600.0;
const WINDOW_DEFAULT_WIDHT: f64 = 1100.0;
const WINDOW_DEFAULT_HEIGHT: f64 = 720.0;

/// 窗口几何状态（uuid → 几何），内存中的唯一数据源
static WINDOWS_STATE: LazyLock<RwLock<HashMap<Uuid, WindowState>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 窗口状态文件路径（window_save.json）
static STATE_FILE: OnceLock<PathBuf> = OnceLock::new();

/// 主窗口句柄（创建后记录，供关闭 / 判断使用）
static MAIN_WINDOW: RwLock<Option<WebviewWindow<tauri::Wry>>> = RwLock::new(None);
static ACCOUNT_WINDOW: RwLock<Option<WebviewWindow<tauri::Wry>>> = RwLock::new(None);

/// 读取窗口状态文件（启动时调用，位于运行路径下）
pub fn init<P: AsRef<Path>>(path: P) {
    let file = STATE_FILE.get_or_init(|| path.as_ref().join(names::WINDOW_SAVE_FILE));

    if file.exists() && file.is_file() {
        if let Ok(data) = serialize_tools::json_from_file::<HashMap<Uuid, WindowState>>(file) {
            WINDOWS_STATE.write().unwrap().extend(data);
        }
    }
}

/// 保存窗口状态
pub fn save() {
    let Some(file) = STATE_FILE.get() else {
        return;
    };
    let data = WINDOWS_STATE.read().unwrap();
    config_save::save(uuids::WINDOW_FILE_UUID, &*data, file);
}

/// 读取指定 uuid 的窗口几何（用于启动时恢复位置大小）
pub fn window_state_for(uuid: &Uuid) -> Option<WindowState> {
    WINDOWS_STATE.read().unwrap().get(uuid).cloned()
}

/// 设置窗口状态
pub fn window_state_set(uuid: &Uuid, state: WindowState) {
    WINDOWS_STATE.write().unwrap().insert(uuid.clone(), state);
    save();
}

/// 主窗口句柄（若已创建）
pub fn main_window() -> Option<WebviewWindow<tauri::Wry>> {
    MAIN_WINDOW.read().unwrap().clone()
}

/// 创建（或聚焦）一个窗口
///
/// 注意：必须由 async 命令调用（同步命令在 Windows 主线程阻塞创建会冻结应用）。
pub fn create_window(app: &AppHandle, label: &str, uuid: &Uuid) -> Result<WebviewWindow, String> {
    // 已存在则聚焦，避免重复窗口
    if let Some(win) = app.get_webview_window(label) {
        win.set_focus().map_err(|err| err.to_string())?;
        return Ok(win);
    }

    let geom = window_state_for(uuid);

    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(names::MCML)
        .min_inner_size(WINDOW_MIN_WIDTH, WINDOW_MIN_HEIGHT);
    let win = match geom {
        Some(g) => builder
            .inner_size(g.width as f64, g.height as f64)
            .position(g.x as f64, g.y as f64)
            .build(),
        None => builder
            .inner_size(WINDOW_DEFAULT_WIDHT, WINDOW_DEFAULT_HEIGHT)
            .center()
            .build(),
    };
    Ok(win.map_err(|e| e.to_string())?)
}

pub fn save_window_state(uuid: &Uuid, window: WebviewWindow) -> Result<(), String> {
    let pos = window.inner_position().map_err(|err| err.to_string())?;
    let size = window.outer_size().map_err(|err| err.to_string())?;

    let mut geom = match window_state_for(&uuid) {
        Some(geom) => geom,
        None => WindowState::default(),
    };

    geom.x = pos.x;
    geom.y = pos.y;
    geom.width = size.width;
    geom.height = size.height;
    window_state_set(&uuid, geom);

    Ok(())
}

pub fn close_window_from_uuid(_app: &AppHandle, uuid: &str, _sub_uuid: &str) -> Result<(), String> {
    let uuid = Uuid::from_str(uuid).map_err(|err| err.to_string())?;
    let binding = WINDOWS_INFO;
    let info = binding.get(&uuid);

    match info {
        Some(window_type) => match window_type {
            WindowType::MainWindow(_label) => {
                let window = MAIN_WINDOW.read().unwrap().clone();
                match window {
                    Some(window) => {
                        save_window_state(&uuid, window);
                        *MAIN_WINDOW.write().unwrap() = None;
                        Ok(())
                    }
                    None => Err(WindowNotFound.to_string()),
                }
            }
            WindowType::AccountWindow(_label) => {
                let window = ACCOUNT_WINDOW.read().unwrap().clone();
                match window {
                    Some(window) => {
                        save_window_state(&uuid, window);
                        *ACCOUNT_WINDOW.write().unwrap() = None;
                        Ok(())
                    }
                    None => Err(WindowNotFound.to_string()),
                }
            }
        },
        None => Err(WindowNotFound.to_string()),
    }
}

pub fn open_window_from_uuid(app: &AppHandle, uuid: &Uuid, _sub_uuid: &str) -> Result<(), String> {
    let binding = WINDOWS_INFO;
    let info = binding.get(&uuid);

    match info {
        Some(window_type) => match window_type {
            WindowType::MainWindow(label) => {
                let window = create_window(app, label, uuid)?;
                *MAIN_WINDOW.write().unwrap() = Some(window);
                Ok(())
            }
            WindowType::AccountWindow(label) => {
                let window = create_window(app, label, uuid)?;
                *ACCOUNT_WINDOW.write().unwrap() = Some(window);
                Ok(())
            }
        },
        None => Err(WindowNotFound.to_string()),
    }
}

pub fn show_main_window(app: &AppHandle) -> Result<(), String> {
    open_window_from_uuid(app, &MAIN_WINDOW_UUID, "")
}

// ================= IPC 命令 =================

/// 打开一个功能窗口（多窗口模式，窗口按钮调用）
///
/// 注意：必须保持 async：同步命令在 Windows 上跑在主线程，而窗口创建会阻塞
/// 等待主线程，导致整个应用冻结（新窗口白屏、无法点击）。
#[tauri::command]
pub async fn window_open_window(
    app: AppHandle,
    uuid: String,
    sub_uuid: String,
) -> Result<(), String> {
    let uuid = Uuid::from_str(&uuid).map_err(|err| err.to_string())?;
    open_window_from_uuid(&app, &uuid, &sub_uuid)
}

/// 关闭窗口（kind → 标签；main 关闭主窗口）
#[tauri::command]
pub fn window_close_window(app: AppHandle, uuid: String, sub_uuid: String) -> Result<(), String> {
    close_window_from_uuid(&app, &uuid, &sub_uuid)
}

/// 获取 GUI 状态（无文件时返回默认值；前端 wire 为 DTO，TS 命名 camelCase）
#[tauri::command]
pub fn window_get_gui_config() -> GuiConfigDto {
    crate::gui_config::get().into()
}

/// 保存 GUI 状态到 gui_config.json（前端 DTO 转内部 GuiConfig）
#[tauri::command]
pub fn window_save_gui_config(config: GuiConfigDto) -> Result<(), String> {
    crate::gui_config::set(config.into());
    Ok(())
}
