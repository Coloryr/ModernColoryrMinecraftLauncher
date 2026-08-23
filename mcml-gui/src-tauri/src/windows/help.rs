//! 帮助窗口：规格 + 创建操作
use tauri::AppHandle;

use super::create;

pub const LABEL: &str = "mcml-help";
pub const TITLE: &str = "帮助手册";
pub const WIDTH: f64 = 760.0;
pub const HEIGHT: f64 = 600.0;

/// 打开帮助窗口
pub fn open(app: &AppHandle) -> Result<(), String> {
    create(app, LABEL, TITLE, WIDTH, HEIGHT)
}
