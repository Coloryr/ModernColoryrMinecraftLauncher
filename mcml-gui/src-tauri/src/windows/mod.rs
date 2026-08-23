//! 窗口模块
//!
//! 每个窗口一个 rs 文件，包含：
//! - 窗口规格（模型）：标签 / 标题 / 尺寸常量
//! - 窗口专属数据模型与方法（账户 / 新闻 / 游戏事件等）
//! - 窗口创建操作：`open(app)` 创建（或聚焦）对应窗口
//! - 窗口按钮调用的方法（IPC 命令，如 list_dir）
//! 通用数据模型见 `../models/`。

pub mod account;
pub mod add;
pub mod help;
pub mod main;
pub mod resource;
pub mod settings;
pub mod skin;
pub mod state;
pub mod stats;

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// 创建窗口的通用操作：已存在则聚焦，否则新建（加载主页面，前端按标签渲染对应页面）
///
/// 注意：必须由 async 命令调用（同步命令在 Windows 主线程阻塞创建会冻结应用）。
pub fn create(app: &AppHandle, label: &str, title: &str, width: f64, height: f64) -> Result<(), String> {
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

/// 打开一个功能窗口（多窗口模式，窗口按钮调用）
///
/// 注意：当前前端改用官方 JS API `new WebviewWindow()` 创建窗口
/// （见 mcml-vue/src/windows/windowManager.ts），本命令保留作备用。
/// 必须保持 async：同步命令在 Windows 上跑在主线程，而窗口创建会阻塞
/// 等待主线程，导致整个应用冻结（新窗口白屏、无法点击）。
///
/// 窗口规格（标题 / 尺寸）与创建操作分别定义在 `windows/<kind>.rs`。
#[tauri::command]
pub async fn open_window(app: AppHandle, kind: String) -> Result<(), String> {
    println!("[open_window] 打开窗口 kind={kind}");
    match kind.as_str() {
        "main" => main::open(&app),
        "settings" => settings::open(&app),
        "stats" => stats::open(&app),
        "skin" => skin::open(&app),
        "help" => help::open(&app),
        "resource" => resource::open(&app),
        "account" => account::open(&app),
        "add" => add::open(&app),
        _ => Err(format!("未知窗口类型: {kind}")),
    }
}
