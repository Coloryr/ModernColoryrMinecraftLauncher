//! MCML 启动器 Tauri 壳
//!
//! 当前阶段：前端使用模拟数据（见 mcml-vue/src/lib/api.ts），
//! 后端提供窗口管理命令，支持真实多窗口模式。
//! 接入核心时在此恢复核心初始化、实例管理、游戏启动等命令。

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

/// 获取应用信息（占位命令，用于验证 IPC 通路）
#[tauri::command]
fn app_info() -> String {
    String::from("MCML Launcher UI")
}

/// 打开一个功能窗口（多窗口模式，备用命令）
///
/// 注意：当前前端改用官方 JS API `new WebviewWindow()` 创建窗口
/// （见 mcml-vue/src/windows/windowManager.ts），本命令保留作备用。
/// 必须保持 async：同步命令在 Windows 上跑在主线程，而窗口创建会阻塞
/// 等待主线程，导致整个应用冻结（新窗口白屏、无法点击）。
/// 异步命令跑在 tokio 工作线程，阻塞的是工作线程，主线程不受影响。
///
/// 每个窗口加载主页面 `index.html`，前端按窗口标签（mcml-<kind>）渲染对应页面。
/// 同一窗口已存在时聚焦，不重复创建。
#[tauri::command]
async fn open_window(app: AppHandle, kind: String) -> Result<(), String> {
    let (title, width, height) = match kind.as_str() {
        "settings" => ("启动器设置", 760.0, 600.0),
        "stats" => ("游戏统计", 760.0, 600.0),
        "skin" => ("皮肤查看", 760.0, 600.0),
        "help" => ("帮助手册", 760.0, 600.0),
        "resource" => ("资源管理", 900.0, 640.0),
        "account" => ("账户管理", 920.0, 640.0),
        _ => ("MCML 启动器", 1100.0, 720.0),
    };

    let label = format!("mcml-{kind}");
    println!("[open_window] 打开窗口 kind={kind} label={label}");

    // 已存在则聚焦，避免重复窗口
    if let Some(win) = app.get_webview_window(&label) {
        println!("[open_window] 窗口已存在，聚焦");
        let _ = win.set_focus();
        return Ok(());
    }

    // 加载主页面；窗口类型由窗口标签（mcml-<kind>）识别，不依赖 URL 参数
    let url = WebviewUrl::App("index.html".into());
    println!("[open_window] 创建窗口 url={url:?}");

    WebviewWindowBuilder::new(&app, label, url)
        .title(title)
        .inner_size(width, height)
        .build()
        .map_err(|e| {
            println!("[open_window] 创建失败: {e}");
            e.to_string()
        })?;

    println!("[open_window] 创建成功");
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![app_info, open_window])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
