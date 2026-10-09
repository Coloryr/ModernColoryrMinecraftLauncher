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
pub mod colormc;
pub mod custom_home;
pub mod download;
pub mod export;
pub mod java_download;
pub mod log;
pub mod main;
pub mod resource;
pub mod settings;
pub mod stats;

// 内部子模块：**只放实现细节**，IPC 命令一律留在本文件
// （`bindings.ts` 的命令分组键取"来源 .rs 模块最后一段"，命令一搬组键就变，前端会断）
//
// 注意：`ipc-gen` 是**裸文本搜索**命令属性（见 ipc-gen/src/scan.rs 的 commands_in），
// 注释里写出那个属性字面量会让它把**紧随其后的函数**当成命令 —— 别在注释里写它。
mod geometry;
mod lifecycle;
mod modpack_task;
mod registry;

use tauri::{AppHandle, Emitter, WebviewWindow};

pub use self::geometry::init;
pub use self::lifecycle::{on_window_event, show_main_window};
// `add.rs` 用 `windows::model_for_kind` 取窗口模型，按原路径再导出
pub(crate) use self::lifecycle::{model_for_kind, window_model};

use self::lifecycle::{
    close_window_from_uuid, drop_model_for_kind, ensure_model_for_kind, open_window_from_uuid,
};

use self::geometry::reconcile_min_size;
use self::registry::{WINDOWS_INFO, min_size_of_label, uuid_for_kind};

use crate::listens;
use uuid::Uuid;

use crate::dtos::{GuiConfigDto, LogFocusDto, WindowSizeDto};

/// 解析前端传来的分组 uuid（空白 / 非法一律按"默认分组"处理）
///
/// 分组以 uuid 为身份（见 mml-game 的 `game_group`），但 IPC 参数照例是字符串，
/// 所以各处统一在这里转一次。宁可落到默认分组，也不要因为一个过期 id 让整条命令失败。
pub fn parse_group_id(group: Option<String>) -> Option<Uuid> {
    group
        .filter(|g| !g.trim().is_empty())
        .and_then(|g| Uuid::parse_str(&g).ok())
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
        .values()
        .map(|e| WindowSizeDto {
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
/// - `query`: 新窗口 URL 上的 query（**不含 `?`**）
///
/// `query` 由前端 `windowManager` 组装 —— 目标窗口的实例与"要打开哪个项目"
/// （`uuid` / `psource` / `ppid` … ）都只能靠 URL 送达，而参数名在前端
/// （`PROJ_PARAMS`，读那边也用它）只有一份，所以不在 Rust 侧另立一套。
/// 传空 / 不传时退回下面按 kind 拼的老路子（只有 log / export 带 uuid）。
#[tauri::command]
pub async fn window_open_window(
    app: AppHandle,
    kind: String,
    instance: Option<String>,
    query: Option<String>,
) -> Result<(), String> {
    kind_parse_check(&kind, &instance)?;
    let Some(uuid) = uuid_for_kind(&kind) else {
        return Err(format!("unknown window kind: {kind}"));
    };
    // 新窗口的目标参数只能靠 URL 送达：前端组装好了就用它（带 uuid / 项目参数），
    // 没给就退回老路子 —— 只有 log / export 把 uuid 拼进去
    let url_path = match query.as_deref().filter(|q| !q.is_empty()) {
        Some(query) => {
            // 前端那份 query 是 URLSearchParams 编出来的（不会出现裸 # / 控制字符）；
            // 这里只做一次兜底，别把畸形串拼进 WebviewUrl
            if query.contains('#') || query.chars().any(char::is_control) {
                return Err(String::from("err.windowQuery"));
            }
            format!("index.html?{query}")
        }
        None => match (&kind[..], &instance) {
            ("log", Some(instance)) | ("export", Some(instance)) => {
                format!("index.html?window={kind}&uuid={instance}")
            }
            _ => String::from("index.html"),
        },
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
// 关掉系统装饰后，本段的动作（拖动 / 激活装饰 / 最小化 / 最大化切换 / 查最大化 /
// 设标题）由前端标题栏调用。写在应用自己的命令里（而非 JS 侧 `getCurrentWindow()`），
// 这样不必往 capabilities/default.json 加窗口权限。
// 关闭不走这里——见 `window_close_window`，那条链路才会跑关闭保护与几何保存。

/// 开始拖动窗口（标题栏按下时调用，之后由系统接管）
#[tauri::command]
pub fn window_start_dragging(window: WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|err| err.to_string())
}

/// 激活插件装饰并显示窗口（所有窗口都走插件装饰，见 [`create_window`]）
///
/// 窗口建出来时是 `decorations: true` + 隐藏的（见 `create_window`），
/// 这里在原生 frame 上激活自绘装饰、再显示。激活成功即由**插件的控件**接管标题栏右侧，
/// Windows 11 的贴靠布局 flyout 挂在它的最大化按钮上 —— 这正是本命令存在的理由。
///
/// 失败时退回原生 frame 并照样显示窗口：宁可露出系统标题栏，也不能让主窗口打不开。
///
/// # 返回值
///
/// 成功返回 `"custom"`（自绘装饰已接管）。激活失败时**已经把窗口退回原生 frame 并显示**
/// 了（宁可露出系统标题栏也不能让窗口打不开），那条路径返回 `Err` 并带上原因。
#[tauri::command]
pub async fn window_activate_decoration(window: WebviewWindow) -> Result<&'static str, String> {
    use tauri_plugin_decoration::WebviewWindowExt;

    if let Err(error) = window.activate_decoration().await {
        // 回退：先确保原生 frame 还在，再显示，最后把原因带回前端
        let _ = window.restore_decoration().await;
        let _ = window.show();
        return Err(format!("decoration activate failed: {error}"));
    }

    // 装饰已摘掉，此刻量到的「外框 − 客户区」才是那圈 Windows 边框带
    // （建窗那一刻量到的是边框带 + 标题栏，见 reconcile_min_size 的文档）。
    // 建窗那次调用**保留**：激活失败退回原生 frame 时，那个口径才是对的。
    if let Some((min_w, min_h)) = min_size_of_label(window.label()) {
        reconcile_min_size(&window, min_w, min_h);
    }

    window.show().map_err(|err| err.to_string())?;
    Ok("custom")
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
pub fn window_ensure_model(kind: String) -> Result<(), String> {
    ensure_model_for_kind(&kind)
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
