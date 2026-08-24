//! 添加实例窗口：规格 + 创建操作 + 窗口按钮调用的方法（list_dir）
use serde::Serialize;
use tauri::AppHandle;

use crate::window_manager::create_window;

/// 目录项（list_dir 返回）
#[derive(Serialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

/// 列出目录的直接内容（目录优先，再按名称排序）；
/// 添加实例窗口选择文件夹时调用，用于预览文件夹内容树
#[tauri::command]
pub fn list_dir(path: String) -> Result<Vec<DirEntry>, String> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        entries.push(DirEntry { name, is_dir });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

/// 窗口规格（模型）
pub const LABEL: &str = "mcml-add";
pub const TITLE: &str = "添加实例";
pub const WIDTH: f64 = 900.0;
pub const HEIGHT: f64 = 660.0;

/// 打开添加实例窗口
pub fn open(app: &AppHandle) -> Result<(), String> {
    create_window(app, LABEL, TITLE, WIDTH, HEIGHT)
}
