//! 窗口模块
//!
//! 每个窗口一个 rs 文件，包含：
//! - 窗口规格（模型）：标签 / 标题 / 尺寸常量
//! - 窗口专属数据模型与方法（账户 / 新闻 / 游戏事件等）
//! - 窗口创建操作：`open(app)` 创建（或聚焦）对应窗口
//! - 窗口按钮调用的方法（IPC 命令，如 list_dir）
//! 通用数据模型见 `../models/`。
//!
//! 窗口的创建 / 聚焦 / 关闭统一由 `../window_manager.rs` 处理，
//! 各窗口的 `open` 只负责传入自己的规格（标签 / 标题 / 尺寸）。
//!
//! 所有窗口的创建、聚焦、关闭统一在这里处理：
//! - 主窗口在 `setup` 阶段通过 [`show_main_window`] 创建（并恢复上次几何）
//! - 功能窗口通过 [`create_window`] 创建或聚焦；`window_open_window` / `window_close_window`
//!   命令供前端调用（多窗口模式），参数为窗口 kind（与前端 `registry.ts` 对应）
//! - 窗口几何状态（`window_save.json`）与 GUI 配置（`gui_config.json`）也在此维护

pub mod account;
pub mod add;
pub mod add_modpack;
pub mod add_resource;
pub mod collect;
pub mod download;
pub mod help;
pub mod main;
pub mod resource;
pub mod settings;
pub mod skin;
pub mod stats;

use std::{
    any::Any,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex, OnceLock, RwLock},
};

use mcml_base::serialize_tools;
use mcml_config::config_save;
use mcml_names::{names, uuids};
use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Error::WindowNotFound, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::listens;
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

/// 主窗口固定 uuid（其余窗口从 2 号起按顺序分配）
const MAIN_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
const ACCOUNT_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000002");

/// 添加实例窗口固定 uuid
const ADD_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000008");

/// 下载窗口固定 uuid
pub const DOWNLOAD_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000009");

/// 下载整合包窗口固定 uuid
const ADD_MODPACK_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-00000000000a");

/// 添加资源窗口固定 uuid
const ADD_RESOURCE_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-00000000000b");

/// 收藏窗口固定 uuid
const COLLECT_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-00000000000c");

/// 窗口注册表条目
struct WindowEntry {
    label: &'static str,
    min_width: f64,
    min_height: f64,
}

/// 通用窗口最小尺寸（= 无历史几何时的默认尺寸）
const MIN_WIDTH: f64 = 640.0;
const MIN_HEIGHT: f64 = 480.0;

/// 主窗口最小尺寸
const MAIN_MIN_WIDTH: f64 = 920.0;
const MAIN_MIN_HEIGHT: f64 = 600.0;

/// 账户窗口最小尺寸
const ACCOUNT_MIN_WIDTH: f64 = 770.0;
const ACCOUNT_MIN_HEIGHT: f64 = 480.0;

/// 添加实例窗口最小尺寸
const ADD_MIN_WIDTH: f64 = 700.0;
const ADD_MIN_HEIGHT: f64 = 585.0;

/// 下载窗口最小尺寸
const DOWNLOAD_MIN_WIDTH: f64 = 670.0;
const DOWNLOAD_MIN_HEIGHT: f64 = 470.0;

/// 下载整合包窗口最小尺寸
const ADD_MODPACK_MIN_WIDTH: f64 = 920.0;
const ADD_MODPACK_MIN_HEIGHT: f64 = 600.0;

/// 窗口注册表：uuid → 窗口信息
const WINDOWS_INFO: LazyLock<HashMap<Uuid, WindowEntry>> = LazyLock::new(|| {
    HashMap::from([
        (
            MAIN_WINDOW_UUID,
            WindowEntry {
                label: "mcml-main",
                min_width: MAIN_MIN_WIDTH,
                min_height: MAIN_MIN_HEIGHT,
            },
        ),
        (
            ACCOUNT_WINDOW_UUID,
            WindowEntry {
                label: "mcml-account",
                min_width: ACCOUNT_MIN_WIDTH,
                min_height: ACCOUNT_MIN_HEIGHT,
            },
        ),
        (
            uuid!("00000000-0000-0000-0000-000000000003"),
            WindowEntry {
                label: "mcml-settings",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            uuid!("00000000-0000-0000-0000-000000000004"),
            WindowEntry {
                label: "mcml-stats",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            uuid!("00000000-0000-0000-0000-000000000005"),
            WindowEntry {
                label: "mcml-skin",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            uuid!("00000000-0000-0000-0000-000000000006"),
            WindowEntry {
                label: "mcml-help",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            uuid!("00000000-0000-0000-0000-000000000007"),
            WindowEntry {
                label: "mcml-resource",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            ADD_WINDOW_UUID,
            WindowEntry {
                label: "mcml-add",
                min_width: ADD_MIN_WIDTH,
                min_height: ADD_MIN_HEIGHT,
            },
        ),
        (
            DOWNLOAD_WINDOW_UUID,
            WindowEntry {
                label: "mcml-download",
                min_width: DOWNLOAD_MIN_WIDTH,
                min_height: DOWNLOAD_MIN_HEIGHT,
            },
        ),
        (
            ADD_MODPACK_WINDOW_UUID,
            WindowEntry {
                label: "mcml-add_modpack",
                min_width: ADD_MODPACK_MIN_WIDTH,
                min_height: ADD_MODPACK_MIN_HEIGHT,
            },
        ),
        (
            ADD_RESOURCE_WINDOW_UUID,
            WindowEntry {
                label: "mcml-add_resource",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            COLLECT_WINDOW_UUID,
            WindowEntry {
                label: "mcml-collect",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
    ])
});

/// 前端窗口 kind → uuid（标签去 `mcml-` 前缀匹配）
fn uuid_for_kind(kind: &str) -> Option<Uuid> {
    WINDOWS_INFO
        .iter()
        .find(|(_, e)| e.label.strip_prefix("mcml-") == Some(kind))
        .map(|(uuid, _)| *uuid)
}

/// 窗口几何状态（uuid → 几何），内存中的唯一数据源
static WINDOWS_STATE: LazyLock<RwLock<HashMap<Uuid, WindowState>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 窗口状态文件路径（window_save.json）
static STATE_FILE: OnceLock<PathBuf> = OnceLock::new();

/// 已打开窗口的句柄表（uuid → 句柄）
static OPEN_WINDOWS: LazyLock<RwLock<HashMap<Uuid, WebviewWindow<tauri::Wry>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 窗口模型表（uuid → 模型）
///
/// 窗口模型跟随窗口生命周期：开窗时创建、关窗时销毁，不走全局 app.manage。
/// Tauri 的 `manage` / `State` 底层是同一个全局 StateManager，无法按窗口隔离，
/// 因此模型由本模块持有；命令通过窗口 label 解析 uuid 后取用。
static WINDOW_MODELS: LazyLock<RwLock<HashMap<Uuid, Arc<dyn Any + Send + Sync>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 确保指定窗口的模型已创建（尚无则初始化）
///
/// 当前仅主窗口有模型（`MainWindowModel`），其余窗口无状态；
/// 新增窗口模型时在此按 uuid 分派。
fn ensure_window_model(app: &AppHandle, uuid: &Uuid) {
    let mut models = WINDOW_MODELS.write().unwrap();
    if *uuid == MAIN_WINDOW_UUID {
        models
            .entry(uuid.clone())
            .or_insert_with(|| Arc::new(Mutex::new(crate::windows::main::MainWindowModel::new())));
    } else if *uuid == ADD_WINDOW_UUID || *uuid == ADD_MODPACK_WINDOW_UUID {
        models
            .entry(uuid.clone())
            .or_insert_with(|| Arc::new(Mutex::new(crate::windows::add::AddWindowModel::new())));
    }
}

/// 取窗口模型（按调用方窗口的 label 解析 uuid）
///
/// 模型不存在（窗口未开）时返回 None，命令侧自行决定降级行为。
/// 兜底：`Destroyed` 事件若丢失（异常关闭路径），模型会残留——命令进来时
/// 发现窗口已不在，顺带清掉，保证模型严格跟随窗口生命周期。
pub fn window_model<T: Send + Sync + 'static>(window: &WebviewWindow) -> Option<Arc<T>> {
    let label = window.label().to_string();
    let kind = label.strip_prefix("mcml-")?;
    let uuid = uuid_for_kind(kind)?;
    let model = WINDOW_MODELS.read().unwrap().get(&uuid)?.clone();
    if window.get_webview_window(&label).is_none() {
        remove_window_model(&uuid);
        return None;
    }
    model.downcast::<T>().ok()
}

/// 移除窗口模型（窗口销毁时调用）
fn remove_window_model(uuid: &Uuid) {
    WINDOW_MODELS.write().unwrap().remove(uuid);
}

/// 窗口是否拒绝本次关闭
///
/// - 下载窗口：仍有下载任务时拒绝（前端弹确认框，确认后停止下载再关窗）
/// - 添加实例 / 下载整合包窗口：模型侧关闭保护（查询数据期间）
/// 其余窗口不保护。
fn close_guarded(uuid: &Uuid) -> bool {
    // 下载窗口：任务未清空时不让直接关，避免后台下载被静默中断
    if *uuid == DOWNLOAD_WINDOW_UUID {
        return !mcml_downloader::get_tasks().is_empty();
    }

    let binding = WINDOW_MODELS.read().unwrap();
    let Some(model) = binding.get(uuid) else {
        return false;
    };
    let model = model.clone();
    drop(binding);
    let Ok(model) = model.downcast::<Mutex<crate::windows::add::AddWindowModel>>() else {
        return false;
    };
    model.lock().unwrap().close_guard()
}

/// 关闭被拒绝事件（窗口处于关闭保护时前端弹提示说明原因）
#[gui_macros::emit]
pub fn emit_close_blocked(window: &tauri::Window<tauri::Wry>) {
    let _ = window.emit_to(window.label(), listens::CLOSE_BLOCKED, ());
}

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

/// 创建（或聚焦）一个窗口
///
/// 注意：必须由 async 命令调用（同步命令在 Windows 主线程阻塞创建会冻结应用）。
fn create_window(app: &AppHandle, label: &str, uuid: &Uuid) -> Result<WebviewWindow, String> {
    // 已存在则聚焦，避免重复窗口
    if let Some(win) = app.get_webview_window(label) {
        win.set_focus().map_err(|err| err.to_string())?;
        // 句柄表可能因异常退出丢失，补记一份
        OPEN_WINDOWS
            .write()
            .unwrap()
            .insert(uuid.clone(), win.clone());
        ensure_window_model(app, uuid);
        return Ok(win);
    }

    let geom = window_state_for(uuid);

    // 最小尺寸随注册表条目走（各窗口内容布局不同，可压缩程度不同）；
    // 无历史几何时直接以最小尺寸居中打开（注册表即窗口尺寸的唯一来源）
    let binding = WINDOWS_INFO;
    let (min_w, min_h) = binding
        .get(uuid)
        .map(|e| (e.min_width, e.min_height))
        .unwrap_or((600.0, 400.0));

    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App("index.html".into()))
        .title(names::MCML)
        .min_inner_size(min_w, min_h)
        // 自绘标题栏：关掉系统装饰，最小化 / 最大化 / 关闭由前端 WindowControls 调下面的命令实现
        .decorations(false)
        // 原生窗口背景铺暗色底，避免 webview 加载首帧白屏（与暗色主题 --bg 一致）
        .background_color(tauri::window::Color(0x14, 0x16, 0x1a, 0xff));
    let win = match geom {
        Some(g) => builder
            .inner_size(g.width as f64, g.height as f64)
            .position(g.x as f64, g.y as f64)
            .build(),
        None => builder.inner_size(min_w, min_h).center().build(),
    };
    let win = win.map_err(|e| e.to_string())?;
    OPEN_WINDOWS
        .write()
        .unwrap()
        .insert(uuid.clone(), win.clone());
    ensure_window_model(app, uuid);
    // 开窗即记录初始几何（文件始终反映当前所有窗口；后续缩放/移动会持续更新）
    let _ = save_window_state(uuid, &win);
    Ok(win)
}

/// 保存窗口几何到状态表
///
/// 坐标系须与恢复端（`WebviewWindowBuilder`）一致：
/// `.position()` 设置的是外框位置 → 存 `outer_position()`；
/// `.inner_size()` 设置的是客户区尺寸 → 存 `inner_size()`。
/// 混用会导致每次开窗位置漂移、窗口逐次变大。
fn save_window_state(uuid: &Uuid, window: &WebviewWindow) -> Result<(), String> {
    let pos = window.outer_position().map_err(|err| err.to_string())?;
    let size = window.inner_size().map_err(|err| err.to_string())?;

    let mut geom = match window_state_for(uuid) {
        Some(geom) => geom,
        None => WindowState::default(),
    };

    geom.x = pos.x;
    geom.y = pos.y;
    geom.width = size.width;
    geom.height = size.height;
    window_state_set(uuid, geom);

    Ok(())
}

/// 关闭指定 uuid 的窗口：先保存几何，再真正关闭窗口
pub fn close_window_from_uuid(app: &AppHandle, uuid: &Uuid) -> Result<(), String> {
    let binding = WINDOWS_INFO;
    let Some(entry) = binding.get(uuid) else {
        return Err(WindowNotFound.to_string());
    };
    let label = entry.label;

    // 句柄优先取句柄表，缺失时按标签现查（覆盖外部创建的窗口）
    let window = match OPEN_WINDOWS.write().unwrap().remove(uuid) {
        Some(win) => Some(win),
        None => app.get_webview_window(label),
    };
    let Some(window) = window else {
        return Err(WindowNotFound.to_string());
    };

    // 句柄可能已失效（窗口被外部关闭）：标签已不存在则清理句柄表并视为已关闭
    if app.get_webview_window(label).is_none() {
        OPEN_WINDOWS.write().unwrap().remove(uuid);
        return Ok(());
    }

    save_window_state(uuid, &window)?;
    window.close().map_err(|err| err.to_string())?;
    Ok(())
}

/// 打开（或聚焦）指定 uuid 的窗口
pub fn open_window_from_uuid(app: &AppHandle, uuid: &Uuid) -> Result<(), String> {
    let binding = WINDOWS_INFO;
    let Some(entry) = binding.get(uuid) else {
        return Err(WindowNotFound.to_string());
    };
    let label = entry.label;

    create_window(app, label, uuid)?;
    Ok(())
}

/// 创建 / 聚焦主窗口（setup 阶段调用）
pub fn show_main_window(app: &AppHandle) -> Result<(), String> {
    open_window_from_uuid(app, &MAIN_WINDOW_UUID)
}

/// 窗口事件处理（注册于 `Builder::on_window_event`）
///
/// 原生标题栏 X、JS API 直接 `close()` 都不经过 `window_close_window` 命令，
/// 在 `CloseRequested` 阶段统一保存几何（窗口仍存活，位置有效）；
/// `Destroyed` 阶段清理句柄表并销毁跟随窗口的模型。
pub fn on_window_event(window: &tauri::Window<tauri::Wry>, event: &tauri::WindowEvent) {
    let label = window.label().to_string();
    let binding = WINDOWS_INFO;
    let Some((uuid, _)) = binding.iter().find(|(_, e)| e.label == label) else {
        return;
    };
    match event {
        tauri::WindowEvent::CloseRequested { api, .. } => {
            // 窗口模型开启关闭保护时（如正在查询数据）拒绝本次关闭请求，并通知前端提示
            if close_guarded(uuid) {
                api.prevent_close();
                emit_close_blocked(window);
                return;
            }
            // 关闭已放行：取消该窗口还在跑的整合包搜索
            add_modpack::cancel_search(&label);
            if let Some(win) = window.app_handle().get_webview_window(&label) {
                let _ = save_window_state(uuid, &win);
            }
            if *uuid == MAIN_WINDOW_UUID {
                mcml_core::stop();
                let others: Vec<WebviewWindow<tauri::Wry>> = OPEN_WINDOWS
                    .write()
                    .unwrap()
                    .iter()
                    .filter(|(u, _)| **u != MAIN_WINDOW_UUID)
                    .map(|(_, w)| w.clone())
                    .collect();
                for win in others {
                    // 主窗口退出是最终退出：destroy 强制关闭，绕过各窗口的关闭保护
                    let _ = win.destroy();
                }
            }
        }
        // 窗口已销毁：清理句柄与跟随窗口的模型（下次开窗重新创建）
        tauri::WindowEvent::Destroyed => {
            OPEN_WINDOWS.write().unwrap().remove(uuid);
            remove_window_model(uuid);
        }
        // 缩放 / 移动：实时记录几何（不必等关窗，配置保存按文件去重合并写入）
        // 最大化 / 全屏时不记（否则还原后会以最大化尺寸打开）
        tauri::WindowEvent::Resized(_) | tauri::WindowEvent::Moved(_) => {
            if let Some(win) = window.app_handle().get_webview_window(&label) {
                if win.is_maximized().unwrap_or(false) || win.is_fullscreen().unwrap_or(false) {
                    return;
                }
                let _ = save_window_state(uuid, &win);
            }
        }
        _ => {}
    }
}

/// 打开一个功能窗口（多窗口模式，窗口按钮调用）
///
/// 注意：必须保持 async：同步命令在 Windows 上跑在主线程，而窗口创建会阻塞
/// 等待主线程，导致整个应用冻结（新窗口白屏、无法点击）。
#[tauri::command]
pub async fn window_open_window(app: AppHandle, kind: String) -> Result<(), String> {
    let Some(uuid) = uuid_for_kind(&kind) else {
        return Err(format!("unknown window kind: {kind}"));
    };
    open_window_from_uuid(&app, &uuid)
}

/// 关闭一个窗口（多窗口模式，窗口关闭按钮调用）
#[tauri::command]
pub fn window_close_window(app: AppHandle, kind: String) -> Result<(), String> {
    let Some(uuid) = uuid_for_kind(&kind) else {
        return Err(format!("unknown window kind: {kind}"));
    };
    close_window_from_uuid(&app, &uuid)
}

// ---------------- 自绘标题栏的窗口控制 ----------------
//
// 关掉系统装饰后，这三个动作由前端标题栏调用。写在应用自己的命令里（而非 JS 侧
// `getCurrentWindow()`），这样不必往 capabilities/default.json 加窗口权限。
// 关闭不走这里——见 `window_close_window`，那条链路才会跑关闭保护与几何保存。

/// 开始拖动窗口（标题栏按下时调用，之后由系统接管）
#[tauri::command]
pub fn window_start_dragging(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|err| err.to_string())
}

/// 最小化窗口
#[tauri::command]
pub fn window_minimize(window: WebviewWindow) -> Result<(), String> {
    window.minimize().map_err(|err| err.to_string())
}

/// 最大化 / 还原（取反），返回切换后的状态
#[tauri::command]
pub fn window_toggle_maximize(window: WebviewWindow) -> Result<bool, String> {
    let maximized = window.is_maximized().map_err(|err| err.to_string())?;
    if maximized {
        window.unmaximize().map_err(|err| err.to_string())?;
    } else {
        window.maximize().map_err(|err| err.to_string())?;
    }
    Ok(!maximized)
}

/// 当前是否最大化（标题栏换图标用；`Win+↑`、系统贴靠等外部操作也会改它）
#[tauri::command]
pub fn window_is_maximized(window: WebviewWindow) -> bool {
    window.is_maximized().unwrap_or(false)
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
