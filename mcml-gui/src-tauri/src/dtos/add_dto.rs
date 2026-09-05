//! 添加实例窗口 DTO：目录项（`add_list_dir` 返回，前端 wire）

use serde::Serialize;

/// 目录项（list_dir 返回）
#[derive(Serialize)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

/// 加载器支持列表查询进度（step / total，前端弹窗显示进度条）
#[derive(Clone, Serialize)]
pub struct LoaderProgressDto {
    pub step: u32,
    pub total: u32,
}
