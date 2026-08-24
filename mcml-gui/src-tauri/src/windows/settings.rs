//! 设置窗口：规格 + 创建操作
use tauri::AppHandle;

use crate::window_manager::create_window;

pub const LABEL: &str = "mcml-settings";
pub const TITLE: &str = "启动器设置";
pub const WIDTH: f64 = 760.0;
pub const HEIGHT: f64 = 600.0;

/// 打开设置窗口
pub fn open(app: &AppHandle) -> Result<(), String> {
    create_window(app, LABEL, TITLE, WIDTH, HEIGHT)
}
