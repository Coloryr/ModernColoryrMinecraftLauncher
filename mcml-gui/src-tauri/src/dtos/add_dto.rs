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

/// 压缩包检测结果（packType 为压缩包类型 ID，name 为推荐实例名）
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedPackDto {
    pub pack_type: String,
    pub name: String,
}

/// 整合包搜索结果条目（id 为项目 ID，来源内唯一）
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackItemDto {
    pub id: String,
    pub name: String,
    pub desc: String,
    pub icon: String,
    pub author: String,
    pub downloads: u64,
}

/// 整合包搜索结果分页
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackSearchDto {
    pub items: Vec<ModpackItemDto>,
    /// 当前页（从 0 开始）
    pub page: u32,
    /// 总条数
    pub total: u64,
}

/// 整合包可安装版本（id 为文件/版本 ID，安装时回传）
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackFileDto {
    pub id: String,
    /// 版本名
    pub name: String,
    /// 版本号 / 文件名
    pub file_name: String,
    pub date: String,
    /// 文件大小（字节，未知为 0）
    pub size: u64,
}

/// 整合包安装进度（state：downloadPack / readInfo / getInfo / downloadFile / extract / done）
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackProgressDto {
    pub state: String,
    pub now: u32,
    pub total: u32,
    pub sub_text: Option<String>,
    pub sub_now: u32,
    pub sub_total: u32,
}
