//! 窗口模块
//!
//! 每个窗口一个 rs 文件，包含：
//! - 窗口规格（模型）：标签 / 标题 / 尺寸常量
//! - 窗口专属数据模型与方法（账户 / 新闻 / 游戏事件等）
//! - 窗口按钮调用的方法（IPC 命令，如 list_dir）
//!
//! 所有窗口的创建、聚焦、关闭统一在本模块处理：
//! - 主窗口在 `setup` 阶段通过 [`show_main_window`] 创建（并恢复上次几何）
//! - 功能窗口通过 [`create_window`] 创建或聚焦；`window_open_window` / `window_close_window`
//!   命令供前端调用（多窗口模式），参数为窗口 kind（与前端 `registry.ts` 对应）
//! - 窗口几何状态（`window_save.json`）与 GUI 配置（`gui_config.json`）也在此维护

pub mod account;
pub mod add;
pub mod add_modpack;
pub mod add_resource;
pub mod block;
pub mod collect;
pub mod custom_home;
pub mod download;
pub mod export;
pub mod help;
pub mod java_download;
pub mod log;
pub mod main;
pub mod resource;
pub mod settings;
pub mod stats;

use std::{
    any::Any,
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, Mutex, OnceLock, RwLock},
};

use mml_base::serialize_tools;
use mml_config::config_save;
use mml_names::{names, uuids};
use serde::{Deserialize, Serialize};
use tauri::{
    AppHandle, Emitter, Error::WindowNotFound, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::listens;
use uuid::{Uuid, uuid};

use crate::dtos::{GuiConfigDto, LogFocusDto, WindowSizeDto};

/// 窗口几何状态（window_save.json）
///
/// 全仓统一这一套口径：
/// - `x / y`：**外框**位置，物理像素（与 `outer_position()` 同源）
/// - `width / height`：**客户区**尺寸，物理像素（与 `inner_size()` 同源），且不小于注册表里的最小客户区
///
/// 注册表 `WINDOWS_INFO` 里的最小 / 默认尺寸则是**客户区 + 逻辑像素**，两者之间只差显示器的
/// 缩放系数一次换算（见 [`min_inner_physical`]）；只有下发给 Windows 的外框最小尺寸
/// 才需要再加上那圈边框带（见 [`reconcile_min_size`]）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowState {
    /// 外框 X（`outer_position()`）
    pub x: i32,
    /// 外框 Y（`outer_position()`）
    pub y: i32,
    /// 客户区宽（`inner_size()`）
    pub width: u32,
    /// 客户区高（`inner_size()`）
    pub height: u32,
    /// 上次退出时是否处于最大化（全屏也算）
    ///
    /// 为 true 时 `x / y / width / height` 保留的是**最大化之前**的几何：开窗先按它开、
    /// 再最大化，这样既回到原来那块屏，也能恢复最大化状态。
    pub maximized: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            maximized: false,
        }
    }
}

/// 主窗口固定 uuid（其余窗口从 2 号起按顺序分配）
const MAIN_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");

/// 账户窗口
const ACCOUNT_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000002");

/// 设置窗口
const SETTINGS_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000003");

/// 统计窗口
const STATES_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000004");

/// 帮助窗口
const HELP_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000006");

/// 资源管理窗口
const RESOURCE_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000007");

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

/// 方块列表窗口固定 uuid
const BLOCK_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-00000000000d");

/// Java 下载窗口固定 uuid
const JAVA_DOWNLOAD_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-00000000000e");

/// 游戏日志窗口固定 uuid
const LOG_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-00000000000f");

/// 实例导出窗口固定 uuid
const EXPORT_WINDOW_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000010");

/// 窗口注册表条目
struct WindowEntry {
    /// 窗口标签（`mml-<kind>`，与前端 kind 对应）
    label: &'static str,
    /// 最小**客户区**宽度（逻辑像素；= 无历史几何时的默认宽度）
    min_width: f64,
    /// 最小**客户区**高度（逻辑像素；= 无历史几何时的默认高度）
    min_height: f64,
}

/// 通用窗口最小尺寸（= 无历史几何时的默认尺寸）
const MIN_WIDTH: f64 = 640.0;
/// 通用窗口最小高度
const MIN_HEIGHT: f64 = 480.0;

/// 主窗口最小宽度
const MAIN_MIN_WIDTH: f64 = 920.0;
/// 主窗口最小高度
const MAIN_MIN_HEIGHT: f64 = 600.0;

/// 账户窗口最小宽度
const ACCOUNT_MIN_WIDTH: f64 = 800.0;
/// 账户窗口最小高度
const ACCOUNT_MIN_HEIGHT: f64 = 650.0;

/// 添加实例窗口最小宽度
const ADD_MIN_WIDTH: f64 = 700.0;
/// 添加实例窗口最小高度
const ADD_MIN_HEIGHT: f64 = 585.0;

/// 下载窗口最小宽度
const DOWNLOAD_MIN_WIDTH: f64 = 670.0;
/// 下载窗口最小高度
const DOWNLOAD_MIN_HEIGHT: f64 = 470.0;

/// 下载整合包窗口最小宽度
const ADD_MODPACK_MIN_WIDTH: f64 = 920.0;
/// 下载整合包窗口最小高度
const ADD_MODPACK_MIN_HEIGHT: f64 = 600.0;

/// 设置窗口最小宽度（左侧标签导航布局需要的宽度）
const SETTINGS_MIN_WIDTH: f64 = 850.0;
/// 设置窗口最小高度
const SETTINGS_MIN_HEIGHT: f64 = 600.0;

/// Java 下载窗口最小宽度（四行下拉 + 下载按钮的小表单窗）
const JAVA_DOWNLOAD_MIN_WIDTH: f64 = 520.0;
/// Java 下载窗口最小高度
const JAVA_DOWNLOAD_MIN_HEIGHT: f64 = 420.0;

/// 游戏日志窗口最小宽度（日志控制台 + 实例选择条）
const LOG_MIN_WIDTH: f64 = 720.0;
/// 游戏日志窗口最小高度
const LOG_MIN_HEIGHT: f64 = 480.0;

/// 实例导出窗口最小宽度（元数据表单 + 导出设置）
const EXPORT_MIN_WIDTH: f64 = 560.0;
/// 实例导出窗口最小高度
const EXPORT_MIN_HEIGHT: f64 = 500.0;

/// 方块列表窗口最小宽度（分类栏 + 网格；工具条上还有搜索框、图标尺寸档与两个按钮）
const BLOCK_MIN_WIDTH: f64 = 770.0;
/// 方块列表窗口最小高度
const BLOCK_MIN_HEIGHT: f64 = 560.0;

/// 窗口注册表：uuid → 窗口信息
const WINDOWS_INFO: LazyLock<HashMap<Uuid, WindowEntry>> = LazyLock::new(|| {
    HashMap::from([
        (
            MAIN_WINDOW_UUID,
            WindowEntry {
                label: "mml-main",
                min_width: MAIN_MIN_WIDTH,
                min_height: MAIN_MIN_HEIGHT,
            },
        ),
        (
            ACCOUNT_WINDOW_UUID,
            WindowEntry {
                label: "mml-account",
                min_width: ACCOUNT_MIN_WIDTH,
                min_height: ACCOUNT_MIN_HEIGHT,
            },
        ),
        (
            SETTINGS_WINDOW_UUID,
            WindowEntry {
                label: "mml-settings",
                min_width: SETTINGS_MIN_WIDTH,
                min_height: SETTINGS_MIN_HEIGHT,
            },
        ),
        (
            STATES_WINDOW_UUID,
            WindowEntry {
                label: "mml-stats",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            HELP_WINDOW_UUID,
            WindowEntry {
                label: "mml-help",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            RESOURCE_WINDOW_UUID,
            WindowEntry {
                label: "mml-resource",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            ADD_WINDOW_UUID,
            WindowEntry {
                label: "mml-add",
                min_width: ADD_MIN_WIDTH,
                min_height: ADD_MIN_HEIGHT,
            },
        ),
        (
            DOWNLOAD_WINDOW_UUID,
            WindowEntry {
                label: "mml-download",
                min_width: DOWNLOAD_MIN_WIDTH,
                min_height: DOWNLOAD_MIN_HEIGHT,
            },
        ),
        (
            ADD_MODPACK_WINDOW_UUID,
            WindowEntry {
                label: "mml-add_modpack",
                min_width: ADD_MODPACK_MIN_WIDTH,
                min_height: ADD_MODPACK_MIN_HEIGHT,
            },
        ),
        (
            ADD_RESOURCE_WINDOW_UUID,
            WindowEntry {
                label: "mml-add_resource",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            COLLECT_WINDOW_UUID,
            WindowEntry {
                label: "mml-collect",
                min_width: MIN_WIDTH,
                min_height: MIN_HEIGHT,
            },
        ),
        (
            BLOCK_WINDOW_UUID,
            WindowEntry {
                label: "mml-block",
                min_width: BLOCK_MIN_WIDTH,
                min_height: BLOCK_MIN_HEIGHT,
            },
        ),
        (
            JAVA_DOWNLOAD_WINDOW_UUID,
            WindowEntry {
                label: "mml-java_download",
                min_width: JAVA_DOWNLOAD_MIN_WIDTH,
                min_height: JAVA_DOWNLOAD_MIN_HEIGHT,
            },
        ),
        (
            LOG_WINDOW_UUID,
            WindowEntry {
                label: "mml-log",
                min_width: LOG_MIN_WIDTH,
                min_height: LOG_MIN_HEIGHT,
            },
        ),
        (
            EXPORT_WINDOW_UUID,
            WindowEntry {
                label: "mml-export",
                min_width: EXPORT_MIN_WIDTH,
                min_height: EXPORT_MIN_HEIGHT,
            },
        ),
    ])
});

/// 前端窗口 kind → uuid（标签去 `mml-` 前缀匹配）
fn uuid_for_kind(kind: &str) -> Option<Uuid> {
    WINDOWS_INFO
        .iter()
        .find(|(_, e)| e.label.strip_prefix("mml-") == Some(kind))
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
    let kind = label.strip_prefix("mml-")?;
    let uuid = uuid_for_kind(kind)?;
    let model = WINDOW_MODELS.read().unwrap().get(&uuid)?.clone();
    if window.get_webview_window(&label).is_none() {
        remove_window_model(&uuid);
        return None;
    }
    model.downcast::<T>().ok()
}

/// 按窗口 kind 取模型（单窗口模式下页面命令用）
///
/// 单窗口模式只有一个真实主窗口，"添加实例""下载整合包"都只是应用内的页面，
/// 它们发命令时调用方窗口一律是主窗口 —— 用 [`window_model`] 会解析出主窗口的 uuid、
/// 拿到 `MainWindowModel`，页面的命令便取不到自己的模型（表现为 `err.modelMissing`）。
/// 这里由命令显式声明自己属于哪个 kind。
///
/// **不创建模型**：模型仍由"该 kind 的窗口被打开"来创建（多窗口模式走 [`create_window`]，
/// 单窗口模式走前端的 [`window_ensure_model`] 命令），页面关掉时也会被释放，
/// 不会因为这里取一次就常驻下来。
pub fn model_for_kind<T: Send + Sync + 'static>(kind: &str) -> Option<Arc<T>> {
    let uuid = uuid_for_kind(kind)?;
    WINDOW_MODELS
        .read()
        .unwrap()
        .get(&uuid)?
        .clone()
        .downcast::<T>()
        .ok()
}

/// 确保某个 kind 的模型存在（单窗口模式下页面打开时由前端调用）
///
/// 单窗口模式没有真实窗口可挂，模型的生命周期改由前端页面的启停驱动：
/// 页面挂载时建、切走/关闭时释放（见 [`window_drop_model`]）。不这么做的话，
/// "添加实例"这类页面的模型要么取不到（`err.modelMissing`），要么只能常驻占内存。
pub fn ensure_model_for_kind(app: &AppHandle, kind: &str) -> Result<(), String> {
    let uuid = uuid_for_kind(kind).ok_or_else(|| format!("unknown window kind: {kind}"))?;
    ensure_window_model(app, &uuid);
    Ok(())
}

/// 释放某个 kind 的模型（单窗口模式下页面切走 / 关闭时由前端调用）
pub fn drop_model_for_kind(kind: &str) -> Result<(), String> {
    let uuid = uuid_for_kind(kind).ok_or_else(|| format!("unknown window kind: {kind}"))?;
    // 该 kind 有真实窗口开着时不释放：多窗口模式下页面还在，模型得留着
    if let Some(label) = WINDOWS_INFO.get(&uuid).map(|e| e.label) {
        if app_has_window(label) {
            return Ok(());
        }
    }
    remove_window_model(&uuid);
    Ok(())
}

/// 是否存在某个 label 的真实窗口（不经过 AppHandle，查句柄表）
fn app_has_window(label: &str) -> bool {
    OPEN_WINDOWS
        .read()
        .unwrap()
        .values()
        .any(|w| w.label() == label)
}

/// 移除窗口模型（窗口销毁 / 逻辑页关闭时调用）
fn remove_window_model(uuid: &Uuid) {
    WINDOW_MODELS.write().unwrap().remove(uuid);
}

/// 窗口是否拒绝本次关闭
///
/// - 下载窗口 / 主窗口：仍有下载任务时拒绝（前端弹确认框，确认后停止下载再关窗 / 退出）
/// - 添加实例 / 下载整合包窗口：模型侧关闭保护（查询数据期间）
/// 其余窗口不保护。
fn close_guarded(uuid: &Uuid) -> bool {
    // 下载窗口：任务未清空时不让直接关，避免后台下载被静默中断
    if *uuid == DOWNLOAD_WINDOW_UUID {
        return !mml_downloader::get_tasks().is_empty();
    }

    // 主窗口：关它就是退出应用。单窗口模式下主窗口是唯一的真实窗口，任何页面的 ✕
    // 最后都落到这里；多窗口模式下关主窗口同样是退出。两种情况都会连带停掉下载，
    // 所以与下载窗口同一套处理：先拒绝、由前端问一句，确认后停下载再关
    if *uuid == MAIN_WINDOW_UUID {
        return !mml_downloader::get_tasks().is_empty();
    }

    // 方块窗口：渲染进行中拒绝关闭（渲染任务虽独立于窗口存活，
    // 但关窗后进度无从展示，用户也会误以为渲染被中断）
    if *uuid == BLOCK_WINDOW_UUID {
        return block::render_running();
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
/// - `url_path`：webview 加载的路径（相对应用源，通常为 `index.html`；需要带
///   参数时传 `index.html?...`——tauri 对恰好 `index.html` 的路径走裸 app URL
///   分支，其余字符串经 `Url::join` 拼接，query 可随真实 URL 带给前端）
///
/// 注意：必须由 async 命令调用（同步命令在 Windows 主线程阻塞创建会冻结应用）。
fn create_window(
    app: &AppHandle,
    label: &str,
    uuid: &Uuid,
    url_path: &str,
) -> Result<WebviewWindow, String> {
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
    // 上次是最大化：按记录里的「最大化之前」几何开窗，开完再最大化（见 save_window_state）
    let restore_maximized = geom.as_ref().map(|g| g.maximized).unwrap_or(false);

    // 最小尺寸随注册表条目走（各窗口内容布局不同，可压缩程度不同）；
    // 无历史几何时直接以最小尺寸居中打开（注册表即窗口尺寸的唯一来源）
    let binding = WINDOWS_INFO;
    let (min_w, min_h) = binding
        .get(uuid)
        .map(|e| (e.min_width, e.min_height))
        .unwrap_or((600.0, 400.0));

    // webview 铺不透明暗色底，避免加载首帧透出桌面（与暗色主题 --bg 一致）
    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url_path.into()))
        .title(names::MML)
        .min_inner_size(min_w, min_h)
        // 自绘标题栏：关掉系统装饰，最小化 / 最大化 / 关闭由前端 WindowControls 调下面的命令实现
        .decorations(false)
        // 保留系统阴影（tao 的默认值）：Win11 下窗口投影与圆角都由 DWM 画。
        // 代价是客户区被缩进一圈：这是 Windows 的**边框 / 非客户区带**
        // （SM_CXSIZEFRAME + SM_CXPADDEDBORDER，96 DPI 下每边 8px；投影本身画在外框之外），
        // 顶部按 tao 对 Win11 的经验值是 1px。只有 Windows 有这个带，也只有开着无边框阴影时才有。
        // 这一圈归 DWM 画边框 / 圆角，webview 不再铺满外框；拖边缘调整大小走系统非客户区。
        // 若在关掉「透明效果」的机器上这一圈又变成黑边，改回 `.shadow(false)` 即可。
        .shadow(true)
        .background_color(tauri::window::Color(0x14, 0x16, 0x1a, 0xff));
    let win = match geom {
        Some(g) => {
            // 恢复端（builder.position / inner_size）用逻辑像素，保存端（outer_position /
            // inner_size()）是物理像素：按目标显示器的缩放系数换算。否则带缩放（125%/150%）
            // 的屏幕上每开一次窗都乘一遍缩放系数，窗口逐次变大、位置漂移。
            // monitor_from_point 吃物理坐标，与保存的值同坐标系
            let scale = app
                .monitor_from_point(g.x as f64, g.y as f64)
                .ok()
                .flatten()
                .or_else(|| app.primary_monitor().ok().flatten())
                .map(|m| m.scale_factor())
                .unwrap_or(1.0);
            // 恢复前先夹一次最小客户区：历史几何可能小于当前最小尺寸（旧版本存的，或最小尺寸
            // 后来调大过），直接创建会得到一个比最小值还小的窗口，而且这个非法值又会被原样写回
            // 文件、一直循环。这里按客户区口径夹（不含边框带）。
            let (min_pw, min_ph) = min_inner_physical((min_w, min_h), scale);
            builder
                .inner_size(
                    g.width.max(min_pw) as f64 / scale,
                    g.height.max(min_ph) as f64 / scale,
                )
                .position(g.x as f64 / scale, g.y as f64 / scale)
                .build()
        }
        None => builder.inner_size(min_w, min_h).center().build(),
    };
    let win = win.map_err(|e| e.to_string())?;

    // 无边框 + 系统阴影时，客户区比外框小一圈：那是 Windows 的边框 / 非客户区带
    // （96 DPI 下左右下各 8px、顶部 1px；投影画在外框之外，不在这一圈里）。
    // tao 建窗和 set_inner_size 都会补偿这个带，但 WM_GETMINMAXINFO 不会
    // （无边框时 adjust_size 不加减任何边距）—— 于是 min_inner_size 实际约束的是
    // 「外框最小尺寸」，客户区会少掉一圈，界面被压得比注册表里设定的更小。
    reconcile_min_size(&win, min_w, min_h);

    OPEN_WINDOWS
        .write()
        .unwrap()
        .insert(uuid.clone(), win.clone());
    ensure_window_model(app, uuid);
    // 开窗即记录初始几何（文件始终反映当前所有窗口；后续缩放/移动会持续更新）。
    // 带上 restore_maximized：下面 maximize() 触发的 Resized 会被跳过（最大化时不记几何），
    // 否则文件里的最大化标志会被这次初始保存清掉。
    let _ = save_window_state(uuid, &win, restore_maximized);
    if restore_maximized {
        let _ = win.maximize();
    }
    Ok(win)
}

/// 取某窗口的最小客户区尺寸（注册表口径：逻辑像素）
fn min_size_of(uuid: &Uuid) -> Option<(f64, f64)> {
    WINDOWS_INFO.get(uuid).map(|e| (e.min_width, e.min_height))
}

/// 注册表里的最小客户区尺寸（逻辑像素）→ 物理像素
///
/// 存盘与恢复都按客户区口径，夹取时用它。**不要在这里加边框带**：那圈只影响下发给 Windows 的
/// **外框**最小尺寸（见 [`reconcile_min_size`]），客户区本身不含它。
fn min_inner_physical(min: (f64, f64), scale: f64) -> (u32, u32) {
    let scale = if scale > 0.0 { scale } else { 1.0 };
    (
        (min.0 * scale).round().max(1.0) as u32,
        (min.1 * scale).round().max(1.0) as u32,
    )
}

/// 把注册表里的最小尺寸（客户区口径）换算成外框口径，再重设一次
///
/// Windows 的最小尺寸走 `WM_GETMINMAXINFO`，它约束的是**外框**；而我们关心的其实是客户区
/// （webview 能拿到多少排版空间）。无边框 + 系统阴影时「外框 − 客户区」就是那圈 Windows
/// 边框带（96 DPI 下左右下各 8px、顶部 1px，且只有 Windows 有）。
/// 这里量出真实差值（物理像素，随 DPI 自动变化、不用自己算 SM_CXSIZEFRAME ——
/// 那双指标也不是按 DPI 倍数缩的：96→8、120→9、144→11）加到最小尺寸上。
/// 没有这个带时差值为 0，等于什么都没做（关掉阴影或换到别的平台都安全）。
fn reconcile_min_size(win: &WebviewWindow, min_w: f64, min_h: f64) {
    let (Ok(inner), Ok(outer)) = (win.inner_size(), win.outer_size()) else {
        return;
    };
    let dx = outer.width.saturating_sub(inner.width);
    let dy = outer.height.saturating_sub(inner.height);
    if dx == 0 && dy == 0 {
        return;
    }
    // 量到的差值是物理像素，换算回逻辑像素后与 min_w / min_h 一起设回去。
    //
    // 必须用**逻辑单位**：tauri/tao 把它存成 `PixelUnit::Logical`，换到别的 DPI 显示器时
    // 会跟着缩放，与建窗时 `min_inner_size(min_w, min_h)` 的口径一致；
    // 用物理像素（`PhysicalSize`）则不会缩放，高 DPI 屏上"最小 770"就不再成立。
    let scale = win.scale_factor().unwrap_or(1.0);
    if scale <= 0.0 {
        return;
    }
    let width = min_w + dx as f64 / scale;
    let height = min_h + dy as f64 / scale;
    let _ = win.set_min_size(Some(tauri::LogicalSize::new(width, height)));
}

/// 保存窗口几何到状态表
///
/// 坐标系须与恢复端（`WebviewWindowBuilder`）一致：
/// `.position()` 设置的是外框位置 → 存 `outer_position()`；
/// `.inner_size()` 设置的是客户区尺寸 → 存 `inner_size()`。
/// 混用会导致每次开窗位置漂移、窗口逐次变大。
///
/// 两条统一规则：
/// - 尺寸夹到不小于注册表里的最小客户区（见 [`min_inner_physical`]），保证文件里的值能直接开窗；
/// - `maximized`（最大化 / 全屏）为真时**不覆盖几何**，只记下这个标志。最大化窗口的
///   `outer_position()` 带着框外偏移（常见是 −8），当成普通位置存下来，下次开窗
///   `monitor_from_point` 可能解析到**另一块显示器**，而且开出来还不是最大化。
fn save_window_state(uuid: &Uuid, window: &WebviewWindow, maximized: bool) -> Result<(), String> {
    let pos = window.outer_position().map_err(|err| err.to_string())?;
    let size = window.inner_size().map_err(|err| err.to_string())?;

    let mut geom = match window_state_for(uuid) {
        Some(geom) => geom,
        None => WindowState::default(),
    };

    let scale = window.scale_factor().unwrap_or(1.0);
    // 夹到不小于注册表里的最小客户区：文件里出现比最小值还小的尺寸，下次开窗就会以非法尺寸
    // 创建，而且会被反复写回、一直不收敛（真实例子：窗口停在「外框 = 最小尺寸」上时，
    // 客户区比最小值小了一圈带宽，主窗口 920×600 → 904×600）。
    let (min_w, min_h) = min_inner_physical(min_size_of(uuid).unwrap_or((0.0, 0.0)), scale);

    if !maximized {
        geom.x = pos.x;
        geom.y = pos.y;
        geom.width = size.width.max(min_w);
        geom.height = size.height.max(min_h);
    }
    geom.maximized = maximized;
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

    // 最大化 / 全屏时只记标志，几何保留最大化之前的值（见 save_window_state）
    let maximized =
        window.is_maximized().unwrap_or(false) || window.is_fullscreen().unwrap_or(false);
    save_window_state(uuid, &window, maximized)?;
    window.close().map_err(|err| err.to_string())?;
    Ok(())
}

/// 打开（或聚焦）指定 uuid 的窗口
///
/// - `url_path`：传给 [`create_window`] 的 webview 路径
pub fn open_window_from_uuid(app: &AppHandle, uuid: &Uuid, url_path: &str) -> Result<(), String> {
    let binding = WINDOWS_INFO;
    let Some(entry) = binding.get(uuid) else {
        return Err(WindowNotFound.to_string());
    };
    let label = entry.label;

    create_window(app, label, uuid, url_path)?;
    Ok(())
}

/// 创建 / 聚焦主窗口（setup 阶段调用）
pub fn show_main_window(app: &AppHandle) -> Result<(), String> {
    open_window_from_uuid(app, &MAIN_WINDOW_UUID, "index.html")
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
                // 最大化 / 全屏：只记标志、不覆盖几何（否则下次会开在另一块屏、而且不最大化）
                let maximized =
                    win.is_maximized().unwrap_or(false) || win.is_fullscreen().unwrap_or(false);
                let _ = save_window_state(uuid, &win, maximized);
            }
            if *uuid == MAIN_WINDOW_UUID {
                mml_core::stop();
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
                // 这里已确定不是最大化 / 全屏；取消最大化时也会走到这，顺手把标志清掉
                let _ = save_window_state(uuid, &win, false);
            }
        }
        _ => {}
    }
}

/// 打开一个功能窗口（多窗口模式，窗口按钮调用）
///
/// 注意：必须保持 async：同步命令在 Windows 上跑在主线程，而窗口创建会阻塞
/// 等待主线程，导致整个应用冻结（新窗口白屏、无法点击）。
/// 各窗口的默认宽高（无历史几何时创建窗口用的尺寸，与 WINDOWS_INFO 同源）。
///
/// 前端 JS 回退路径（`createViaJs`）从这里取尺寸，前端注册表不再自带宽高。
#[tauri::command]
pub fn window_get_window_sizes() -> Vec<WindowSizeDto> {
    WINDOWS_INFO
        .iter()
        .map(|(_, e)| WindowSizeDto {
            kind: e.label.strip_prefix("mml-").unwrap_or(e.label).to_string(),
            width: e.min_width,
            height: e.min_height,
        })
        .collect()
}

/// 打开一个功能窗口（多窗口模式，窗口按钮调用）
///
/// - `kind`: 窗口类型
/// - `instance`: 目标实例 UUID（游戏日志窗口定位要查看的实例；窗口已存在时
///   聚焦并推送 `log-focus` 事件让已开窗口切换实例）
#[tauri::command]
pub async fn window_open_window(
    app: AppHandle,
    kind: String,
    instance: Option<String>,
) -> Result<(), String> {
    kind_parse_check(&kind, &instance)?;
    let Some(uuid) = uuid_for_kind(&kind) else {
        return Err(format!("unknown window kind: {kind}"));
    };
    // log / export 窗口按 uuid 定位实例：新窗口靠 URL query 拿目标
    let url_path = match (&kind[..], &instance) {
        ("log", Some(instance)) | ("export", Some(instance)) => {
            format!("index.html?window={kind}&uuid={instance}")
        }
        _ => String::from("index.html"),
    };
    open_window_from_uuid(&app, &uuid, &url_path)?;
    // 新创建的窗口靠 URL query 拿目标；已存在的窗口 URL 不变，靠事件切换
    if let Some(instance) = &instance {
        if kind == "log" {
            log::emit_log_focus(
                &app,
                LogFocusDto {
                    uuid: instance.clone(),
                },
            );
        }
        if kind == "export" {
            export::emit_export_focus(
                &app,
                LogFocusDto {
                    uuid: instance.clone(),
                },
            );
        }
    }
    Ok(())
}

/// 打开日志 / 导出窗口时的实例参数校验（kind 与 instance 参数匹配性）
fn kind_parse_check(kind: &str, instance: &Option<String>) -> Result<(), String> {
    if kind != "log" && kind != "export" && instance.is_some() {
        return Err(format!("kind {kind} 不接受 instance 参数"));
    }
    if (kind == "log" || kind == "export")
        && let Some(instance) = instance
    {
        Uuid::parse_str(instance).map_err(|_| "err.uuid".to_string())?;
    }
    Ok(())
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

/// 设置调用方窗口的原生标题（任务栏 / Alt+Tab 显示）
///
/// 窗口创建时统一叫 `names::MML`；自绘标题栏文案由前端持有（已 i18n），
/// 因此由前端在标题变化时同步过来，语言切换后原生标题也能跟随
#[tauri::command]
pub fn window_set_title(window: WebviewWindow, title: String) -> Result<(), String> {
    window.set_title(&title).map_err(|err| err.to_string())
}

/// 确保某个窗口 kind 的模型存在（单窗口模式下前端页面挂载时调用）
///
/// 多窗口模式下模型本来就跟着真实窗口创建，这里重复调用无副作用。
#[tauri::command]
pub fn window_ensure_model(app: AppHandle, kind: String) -> Result<(), String> {
    ensure_model_for_kind(&app, &kind)
}

/// 释放某个窗口 kind 的模型（单窗口模式下前端页面切走 / 关闭时调用）
///
/// 单窗口模式没有"窗口关闭"这个事件可依赖 —— 页面切走时组件进 KeepAlive，
/// 后端收不到任何通知。不显式释放的话模型会一直挂着（常驻内存），
/// 而页面上的取消令牌 / 关闭保护 / 重名应答通道都是**就地失效**的：
/// 下次回到该页会重新 ensure 一份干净的。
#[tauri::command]
pub fn window_drop_model(kind: String) -> Result<(), String> {
    drop_model_for_kind(&kind)
}

/// 重启启动器（切换窗口模式这类"必须重启才干净生效"的设置后，由前端调用）
///
/// 顺序有讲究：
/// 1. **先落盘**：`mml_core::stop()` 触发停止事件，其中 `config_save::stop` 是同步 join
///    （返回即代表队列写完）。若放到拉起新进程之后，新进程可能读到旧配置、仍旧按旧模式打开；
/// 2. **再拉起一份自己**（`current_exe()` 就是当前可执行文件）；
/// 3. **最后退出本进程**：放在后台线程里延迟一下再退——开发模式下 `tauri dev` 的 vite 会随本进程
///    退出而关闭，新进程得趁它活着把页面拉起来；release 没有这个依赖，稍等让新窗口先出现即可。
///
/// 本命令一定结束当前进程，是否提示由调用方决定。
#[tauri::command]
pub fn window_restart_app(app: AppHandle) {
    mml_core::stop();

    std::thread::spawn(move || {
        match std::env::current_exe() {
            Ok(exe) => {
                if let Err(err) = std::process::Command::new(exe).spawn() {
                    mml_log::error(format!("restart launcher failed: {err}"));
                }
            }
            Err(err) => mml_log::error(format!("restart launcher: {err}")),
        }

        // debug（tauri dev）等新进程把页面从 vite 拉起来；release 下没有 dev server 这回事
        let wait = if cfg!(debug_assertions) { 2000 } else { 150 };
        std::thread::sleep(std::time::Duration::from_millis(wait));
        app.exit(0);
    });
}

/// 获取 GUI 状态（无文件时返回默认值；前端 wire 为 DTO，TS 命名 camelCase）
///
/// 首次启动（无配置文件）时默认主题跟随系统深浅色——WebView2 的窗口主题
/// 即系统的应用模式。此时不落盘，用户第一次改设置才会把当时的主题存进文件
#[tauri::command]
pub fn window_get_gui_config(window: WebviewWindow) -> GuiConfigDto {
    let mut config = crate::gui_config::get();
    if crate::gui_config::is_fresh() {
        config.theme = match window.theme() {
            Ok(tauri::Theme::Light) => crate::gui_config::Theme::Light,
            _ => crate::gui_config::Theme::Dark,
        };
    }
    config.into()
}

/// 保存 GUI 状态到 gui_config.json（前端 DTO 转内部 GuiConfig）
///
/// 皮肤显示模式 / 头像配置有变化时广播 `skin-config-change`：渲染缓存键里含这两项，
/// 各窗口收到后按新模式重取头像 / 皮肤图
#[tauri::command]
pub async fn window_save_gui_config(app: AppHandle, config: GuiConfigDto) -> Result<(), String> {
    let mut new_config: crate::gui_config::GuiConfig = config.into();
    let skin_changed = {
        let old = crate::gui_config::get();
        old.skin_display != new_config.skin_display || old.head != new_config.head
    };
    let client_changed = {
        let old = crate::gui_config::get();
        old.client != new_config.client
    };
    let new_client = new_config.client.clone();
    if client_changed {
        let uuid = new_client.lock_instance.clone();
        if !uuid.is_empty()
            && let Ok(uuid) = Uuid::parse_str(&uuid)
            && mml_game::have_instance_uuid(&uuid)
        {
            new_config.main_window.selected_instance = new_client.lock_instance.clone();
        }
    }
    crate::gui_config::set(new_config);
    if skin_changed {
        emit_skin_config_change(&app);
    }
    if client_changed {
        emit_client_config_change(&app, new_client.into());
    }
    Ok(())
}

/// 皮肤 / 头像显示配置变更事件（跨窗口同步重取渲染图）
#[gui_macros::emit]
fn emit_skin_config_change(app: &AppHandle) {
    let _ = app.emit(listens::SKIN_CONFIG_CHANGE, ());
}

/// 客户端设置变更事件（跨窗口同步，如主窗口 MOTD 卡片 / 登录锁定）
#[gui_macros::emit]
fn emit_client_config_change(app: &AppHandle, config: crate::dtos::ClientConfigDto) {
    let _ = app.emit(listens::CLIENT_CONFIG_CHANGE, config);
}
