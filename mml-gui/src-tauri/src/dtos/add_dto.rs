//! 添加实例窗口 DTO：目录项（`add_list_dir` 返回，前端 wire）

use serde::Serialize;

/// 目录项（list_dir 返回）
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirEntry {
    /// 目录 / 文件名
    pub name: String,
    /// 是否为目录
    pub is_dir: bool,
}

/// 加载器支持列表查询进度（前端弹窗显示进度条）
#[derive(Clone, Serialize)]
pub struct LoaderProgressDto {
    /// 当前步数
    pub step: u32,
    /// 总步数
    pub total: u32,
}

/// 实例重名确认（前端弹窗询问）
#[derive(Clone, Serialize)]
pub struct NameConflictDto {
    /// 冲突实例 ID
    pub id: u32,
    /// 处理方式：overwrite 覆盖 / rename 自动改名
    pub kind: String,
    /// 冲突的实例名
    pub name: String,
}

/// 压缩包检测结果
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedPackDto {
    /// 压缩包类型 ID
    pub pack_type: String,
    /// 推荐实例名
    pub name: String,
}

/// 整合包搜索结果条目（id 为项目 ID，来源内唯一）
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModpackItemDto {
    /// 项目 ID
    pub id: String,
    /// 项目名
    pub name: String,
    /// 简介
    pub desc: String,
    /// 图标地址
    pub icon: String,
    /// 作者
    pub author: String,
    /// 下载量
    pub downloads: u64,
}

/// 整合包安装进度（state：downloadPack / readInfo / getInfo / downloadFile / extract / done）
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackProgressDto {
    /// 当前安装阶段 ID
    pub state: String,
    /// 当前阶段进度
    pub now: u32,
    /// 当前阶段总量
    pub total: u32,
    /// 子进度说明文本
    pub sub_text: Option<String>,
    /// 子进度当前值
    pub sub_now: u32,
    /// 子进度总量
    pub sub_total: u32,
}
