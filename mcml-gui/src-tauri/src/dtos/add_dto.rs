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

/// 实例重名确认（kind：overwrite 覆盖 / rename 自动改名，前端弹窗询问）
#[derive(Clone, Serialize)]
pub struct NameConflictDto {
    pub id: u32,
    pub kind: String,
    pub name: String,
}
