//! 添加实例窗口 DTO：目录项（`add_list_dir` 返回，前端 wire）

use serde::Serialize;

/// 目录项（list_dir 返回）
#[derive(Serialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}
