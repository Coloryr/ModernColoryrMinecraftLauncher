//! 资源管理窗口：规格 + 创建操作
use tauri::AppHandle;

use crate::window_manager::create_window;

pub const LABEL: &str = "mcml-resource";
pub const TITLE: &str = "资源管理";
pub const WIDTH: f64 = 900.0;
pub const HEIGHT: f64 = 640.0;

/// 打开资源管理窗口
pub fn open(app: &AppHandle) -> Result<(), String> {
    create_window(app, LABEL, TITLE, WIDTH, HEIGHT)
}
