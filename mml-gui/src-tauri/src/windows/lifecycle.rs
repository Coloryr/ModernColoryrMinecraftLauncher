//! 窗口生命周期：句柄表 / 模型表 / 建窗 / 开关窗 / 窗口事件
//!
//! 从 `windows/mod.rs` 拆出来的。这里管"窗口从生到死"：建窗（几何恢复 + 装饰时机 +
//! 最小尺寸校正）、句柄与模型的登记清理、关闭保护，以及 `on_window_event` 里的
//! 关闭 / 销毁 / 缩放事件。
//!
//! **命令（`window_*`）留在 `mod.rs`**：`bindings.ts` 的组键取"来源 .rs 模块最后一段"，
//! 命令搬文件会让组键从 `windows` 变成别的名字，前端会断（见 AGENTS.md §4）。

use std::{
    any::Any,
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, LazyLock, Mutex, RwLock},
};

use tauri::{
    AppHandle, Emitter, Error::WindowNotFound, Manager, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

use crate::listens;
use mml_names::names;
use uuid::Uuid;

use super::geometry::{
    min_inner_physical, reconcile_min_size, remember_decorated_inset, save_window_state,
    window_state_for,
};
use super::registry::{
    ADD_MODPACK_WINDOW_UUID, ADD_WINDOW_UUID, BLOCK_WINDOW_UUID, DOWNLOAD_WINDOW_UUID,
    MAIN_WINDOW_UUID, WINDOWS_INFO, uuid_for_kind,
};
use super::{add_modpack, block};

/// 已打开窗口的句柄表（uuid → 句柄）
pub(super) static OPEN_WINDOWS: LazyLock<RwLock<HashMap<Uuid, WebviewWindow<tauri::Wry>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 窗口模型表（uuid → 模型）
///
/// 窗口模型跟随窗口生命周期：开窗时创建、关窗时销毁，不走全局 app.manage。
/// Tauri 的 `manage` / `State` 底层是同一个全局 StateManager，无法按窗口隔离，
/// 因此模型由本模块持有；命令通过窗口 label 解析 uuid 后取用。
pub(super) static WINDOW_MODELS: LazyLock<RwLock<HashMap<Uuid, Arc<dyn Any + Send + Sync>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 确保指定窗口的模型已创建（尚无则初始化）
///
/// 当前仅主窗口有模型（`MainWindowModel`），其余窗口无状态；
/// 新增窗口模型时在此按 uuid 分派。
pub(super) fn ensure_window_model(uuid: &Uuid) {
    let mut models = WINDOW_MODELS.write().unwrap();
    if *uuid == MAIN_WINDOW_UUID {
        models
            .entry(*uuid)
            .or_insert_with(|| Arc::new(Mutex::new(crate::windows::main::MainWindowModel::new())));
    } else if *uuid == ADD_WINDOW_UUID || *uuid == ADD_MODPACK_WINDOW_UUID {
        models
            .entry(*uuid)
            .or_insert_with(|| Arc::new(Mutex::new(crate::windows::add::AddWindowModel::new())));
    }
}

/// 取窗口模型（按调用方窗口的 label 解析 uuid）
///
/// 模型不存在（窗口未开）时返回 None，命令侧自行决定降级行为。
/// 兜底：`Destroyed` 事件若丢失（异常关闭路径），模型会残留——命令进来时
/// 发现窗口已不在，顺带清掉，保证模型严格跟随窗口生命周期。
pub(crate) fn window_model<T: Send + Sync + 'static>(window: &WebviewWindow) -> Option<Arc<T>> {
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
pub(crate) fn model_for_kind<T: Send + Sync + 'static>(kind: &str) -> Option<Arc<T>> {
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
pub(crate) fn ensure_model_for_kind(kind: &str) -> Result<(), String> {
    let uuid = uuid_for_kind(kind).ok_or_else(|| format!("unknown window kind: {kind}"))?;
    ensure_window_model(&uuid);
    Ok(())
}

/// 释放某个 kind 的模型（单窗口模式下页面切走 / 关闭时由前端调用）
pub(crate) fn drop_model_for_kind(kind: &str) -> Result<(), String> {
    let uuid = uuid_for_kind(kind).ok_or_else(|| format!("unknown window kind: {kind}"))?;
    // 该 kind 有真实窗口开着时不释放：多窗口模式下页面还在，模型得留着
    if let Some(label) = WINDOWS_INFO.get(&uuid).map(|e| e.label)
        && app_has_window(label)
    {
        return Ok(());
    }
    remove_window_model(&uuid);
    Ok(())
}

/// 是否存在某个 label 的真实窗口（不经过 AppHandle，查句柄表）
pub(super) fn app_has_window(label: &str) -> bool {
    OPEN_WINDOWS
        .read()
        .unwrap()
        .values()
        .any(|w| w.label() == label)
}

/// 移除窗口模型（窗口销毁 / 逻辑页关闭时调用）
pub(super) fn remove_window_model(uuid: &Uuid) {
    WINDOW_MODELS.write().unwrap().remove(uuid);
}

/// 窗口是否拒绝本次关闭
///
/// - 下载窗口 / 主窗口：仍有下载任务时拒绝（前端弹确认框，确认后停止下载再关窗 / 退出）
/// - 添加实例 / 下载整合包窗口：模型侧关闭保护（查询数据期间）
///
/// 其余窗口不保护。
pub(super) fn close_guarded(uuid: &Uuid) -> bool {
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
pub(crate) fn emit_close_blocked(window: &tauri::Window<tauri::Wry>) {
    let _ = window.emit_to(window.label(), listens::CLOSE_BLOCKED, ());
}

/// 创建（或聚焦）一个窗口
///
/// - `url_path`：webview 加载的路径（相对应用源，通常为 `index.html`；需要带
///   参数时传 `index.html?...`——tauri 对恰好 `index.html` 的路径走裸 app URL
///   分支，其余字符串经 `Url::join` 拼接，query 可随真实 URL 带给前端）
///
/// 注意：必须由 async 命令调用（同步命令在 Windows 主线程阻塞创建会冻结应用）。
pub(super) fn create_window(
    app: &AppHandle,
    label: &str,
    uuid: &Uuid,
    url_path: &str,
) -> Result<WebviewWindow, String> {
    // 已存在则聚焦，避免重复窗口
    if let Some(win) = app.get_webview_window(label) {
        win.set_focus().map_err(|err| err.to_string())?;
        // 句柄表可能因异常退出丢失，补记一份
        OPEN_WINDOWS.write().unwrap().insert(*uuid, win.clone());
        ensure_window_model(uuid);
        return Ok(win);
    }

    let geom = window_state_for(uuid);
    // 上次是最大化：按记录里的「最大化之前」几何开窗，开完再最大化（见 save_window_state）
    let restore_maximized = geom.as_ref().map(|g| g.maximized).unwrap_or(false);

    // 最小尺寸随注册表条目走（各窗口内容布局不同，可压缩程度不同）；
    // 无历史几何时直接以最小尺寸居中打开（注册表即窗口尺寸的唯一来源）
    let binding = &*WINDOWS_INFO;
    let (min_w, min_h) = binding
        .get(uuid)
        .map(|e| (e.min_width, e.min_height))
        .unwrap_or((600.0, 400.0));

    // 所有窗口都走插件装饰：必须以 decorations(true) 建出来，插件才能在原生 frame 上
    // 激活自绘装饰；激活前先藏起来，由前端挂载后调 window_activate_decoration 显示。
    //
    // 这样做的收益是 **Win11 贴靠布局**（悬停最大化按钮弹出窗口位置选择面板）。
    // 它靠的是原生 WM_NCHITTEST → HTMAXBUTTON，而插件用一个原生小子窗口盖在
    // 最大化按钮的位置上回答这个消息；矩形由前端量出来交给它
    // （见 mml-vue/src/lib/decoration.ts）。所以按钮外观仍是 M²L 自己的 WindowControls。

    // webview 铺不透明暗色底，避免加载首帧透出桌面（与暗色主题 --bg 一致）
    let builder = WebviewWindowBuilder::new(app, label, WebviewUrl::App(url_path.into()))
        .title(names::MML)
        .min_inner_size(min_w, min_h)
        // 原生 frame 起步 + 隐藏：装饰激活成功后再显示（失败则退回原生 frame 显示）
        .decorations(true)
        .visible(false);

    // WebView2 用户数据目录（EBWebView）搬到缓存目录下
    //
    // 不指定的话 Tauri 在 Windows 上会**强制**落到 `%LOCALAPPDATA%\<identifier>\`
    // （见 tauri 的 manager/webview.rs：`we need to force a data_directory`），
    // 于是界面的 localStorage / cookie / 缓存全散在系统盘用户目录里，
    // 与"启动器自带一份便携数据"的预期不符。这里显式指到 `<cache>/webview`：
    // 用户数据跟运行目录走，卸载 / 拷贝整个目录即可带走。
    //
    // 时机是安全的：`cache/` 由 `mml_downloader::init`（经 `mml_core::init`）
    // 在 main 里创建，而窗口一律在 `run()` 的 setup 之后才建 —— 此时路径已就绪，
    // 这里再兜一次 create_dir_all 防手删。
    let builder = match webview_data_dir() {
        Some(dir) => {
            if let Err(err) = std::fs::create_dir_all(&dir) {
                // 建不出来不致命：退回 Tauri 的默认目录总比开不了窗好
                mml_log::error(format!(
                    "[window] 创建 webview 数据目录失败，退回默认位置：{} ({err})",
                    dir.display()
                ));
                builder
            } else {
                builder.data_directory(dir)
            }
        }
        None => builder,
    };

    let builder = builder.background_color(tauri::window::Color(0x14, 0x16, 0x1a, 0xff));
    let win = match geom {
        Some(g) => {
            // 恢复端（builder.position / inner_size）用逻辑像素，保存端是物理像素：
            // 按目标显示器的缩放系数换算。否则带缩放（125%/150%）的屏幕上每开一次窗
            // 都乘一遍缩放系数，窗口逐次变大、位置漂移。
            // monitor_from_point 吃物理坐标，与保存的值同坐标系。
            let scale = app
                .monitor_from_point(g.x as f64, g.y as f64)
                .ok()
                .flatten()
                .or_else(|| app.primary_monitor().ok().flatten())
                .map(|m| m.scale_factor())
                .unwrap_or(1.0);

            // 恢复前先夹一次最小客户区：历史几何可能小于当前最小尺寸（旧版本存的，或最小尺寸
            // 后来调大过），直接创建会得到一个比最小值还小的窗口，而且这个非法值又会被原样
            // 写回文件、一直循环。这里按客户区口径夹（不含边框带）。
            //
            // 口径：save_window_state 存的是 `inner_size()`（客户区），与这里 `.inner_size()`
            // 同源；建窗这一刻是 decorations(true)，与保存时若无装饰则含义一致（都是客户区）。
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

    // 记下「带装饰」状态下外框 − 客户区的差值，供存盘时折算用。
    //
    // 必须在**这一刻**量：此时窗口还是 decorations(true)，量到的差值包含原生标题栏
    // （实测 16×39）；插件激活调 set_decorations(false) 之后标题栏并进客户区、
    // 差值只剩边框带（实测 16×9），那时就再也量不到标题栏有多高了 ——
    // 而关窗保存正发生在那个状态。
    remember_decorated_inset(uuid, &win);

    // 无边框 + 系统阴影时，客户区比外框小一圈：那是 Windows 的边框 / 非客户区带
    // （96 DPI 下左右下各 8px、顶部 1px；投影画在外框之外，不在这一圈里）。
    // tao 建窗和 set_inner_size 都会补偿这个带，但 WM_GETMINMAXINFO 不会
    // （无边框时 adjust_size 不加减任何边距）—— 于是 min_inner_size 实际约束的是
    // 「外框最小尺寸」，客户区会少掉一圈，界面被压得比注册表里设定的更小。
    reconcile_min_size(&win, min_w, min_h);

    OPEN_WINDOWS.write().unwrap().insert(*uuid, win.clone());
    ensure_window_model(uuid);
    // 开窗即记录初始几何（文件始终反映当前所有窗口；后续缩放/移动会持续更新）。
    // 带上 restore_maximized：下面 maximize() 触发的 Resized 会被跳过（最大化时不记几何），
    // 否则文件里的最大化标志会被这次初始保存清掉。
    let _ = save_window_state(uuid, &win, restore_maximized);
    if restore_maximized {
        let _ = win.maximize();
    }
    Ok(win)
}

/// WebView2 用户数据目录（EBWebView 的落点）
///
/// `<cache>/webview`，其中 `<cache>` 就是下载器的临时目录
/// （`mml_downloader::get_cache_path()` = `<运行目录>/cache`，见 names::CACHE_DIR）。
/// 缓存路径的唯一来源是下载器，这里**不**自己拼运行目录。
///
/// 返回 `Option` 只是沿用调用方"拿不到就退回 Tauri 默认目录"的那条分支：
/// 实际上 `mml_downloader::init` 由 `mml_core::init` 在 `main` 里先跑完
/// （窗口一律在 `run()` 的 setup 之后才建），这里**总是** `Some`。
/// 注意 `mml_downloader::get_cache_path()` 自己在未初始化时是 `unwrap()` **panic**
/// 的（见 mml-downloader/src/lib.rs），所以那条兜底分支真到了也接不住 ——
/// 别把它当保险。
pub(super) fn webview_data_dir() -> Option<PathBuf> {
    let cache = mml_downloader::get_cache_path();
    Some(cache.join(names::WEBVIEW_DIR))
}

/// 关闭指定 uuid 的窗口：先保存几何，再真正关闭窗口
pub(crate) fn close_window_from_uuid(app: &AppHandle, uuid: &Uuid) -> Result<(), String> {
    let binding = &*WINDOWS_INFO;
    let Some(entry) = binding.get(uuid) else {
        return Err(WindowNotFound.to_string());
    };
    let label = entry.label;

    // 句柄优先取句柄表，缺失时按标签现查（覆盖外部创建的窗口）。
    //
    // 这里**只克隆、不提前摘表**：`close()` 会先发 `CloseRequested`（tauri 2.11 的文档
    // 明确写了"It emits WindowEvent::CloseRequested first ... so you can intercept it"），
    // 关闭保护完全可能拦下这次关闭 —— 摘了表而窗口还活着，句柄表就与真实窗口脱节，
    // `app_has_window` 会对一个开着的窗口答"没有"。真正的清理交给 `Destroyed`。
    let window = OPEN_WINDOWS
        .read()
        .unwrap()
        .get(uuid)
        .cloned()
        .or_else(|| app.get_webview_window(label));
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
pub(crate) fn open_window_from_uuid(
    app: &AppHandle,
    uuid: &Uuid,
    url_path: &str,
) -> Result<(), String> {
    let binding = &*WINDOWS_INFO;
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
    let binding = &*WINDOWS_INFO;
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
                let others: Vec<(Uuid, WebviewWindow<tauri::Wry>)> = OPEN_WINDOWS
                    .write()
                    .unwrap()
                    .iter()
                    .filter(|(u, _)| **u != MAIN_WINDOW_UUID)
                    .map(|(u, w)| (*u, w.clone()))
                    .collect();
                for (other_uuid, win) in others {
                    // 主窗口退出是最终退出：destroy 强制关闭，绕过各窗口的关闭保护。
                    // 但 destroy **不发 CloseRequested**，那条链路里的收尾不会执行 ——
                    // 这里手动补上：取消还在跑的整合包搜索、记下最大化标志与几何，
                    // 否则下次开窗既不是最大化、几何也停在旧值。
                    add_modpack::cancel_search(win.label());
                    let maximized =
                        win.is_maximized().unwrap_or(false) || win.is_fullscreen().unwrap_or(false);
                    let _ = save_window_state(&other_uuid, &win, maximized);
                    let _ = win.destroy();
                }
            }
        }
        // 窗口已销毁：清理句柄与跟随窗口的模型（下次开窗重新创建）
        tauri::WindowEvent::Destroyed => {
            // 关窗时把还在等应答的重名确认对话框按"拒绝"收尾：用户没答就关窗的话，
            // 安装 future 会永久 pending（任务卡在"进行中"），发送端也一直留在表里
            if let Some(kind) = label.strip_prefix("mml-")
                && let Some(model) =
                    model_for_kind::<Mutex<crate::windows::add::AddWindowModel>>(kind)
            {
                model.lock().unwrap().reject_all_dialogs();
            }
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
