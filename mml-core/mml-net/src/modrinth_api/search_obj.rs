//! Modrinth 搜索结果 DTO

use serde::{Deserialize, Serialize};

/// Modrinth 搜索返回
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthSearchObj {
    /// 命中的项目总数
    pub total_hits: u64,
    /// 命中条目列表（分页后的当前页）
    pub hits: Vec<HitObj>,
}

impl Default for ModrinthSearchObj {
    fn default() -> Self {
        Self {
            total_hits: Default::default(),
            hits: Default::default(),
        }
    }
}

/// 单条搜索结果
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HitObj {
    /// 项目 ID
    pub project_id: String,
    /// 作者
    pub author: String,
    /// 项目名称
    pub title: String,
    /// 简介
    pub description: String,
    /// 分类标签
    pub categories: Vec<String>,
    /// 图库图片 URL 列表
    pub gallery: Vec<String>,
    /// 下载量
    pub downloads: u64,
    /// 图标 URL
    pub icon_url: Option<String>,
    /// 最后更新时间
    pub date_modified: String,
}

impl Default for HitObj {
    fn default() -> Self {
        Self {
            project_id: Default::default(),
            author: Default::default(),
            title: Default::default(),
            description: Default::default(),
            categories: Default::default(),
            gallery: Default::default(),
            downloads: Default::default(),
            icon_url: Default::default(),
            date_modified: Default::default(),
        }
    }
}
