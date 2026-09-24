//! Modrinth 项目详情 DTO

use serde::{Deserialize, Serialize};

/// Modrinth 项目详情
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthProjectObj {
    /// 项目 ID
    pub id: String,
    /// 项目类型（mod / modpack 等）
    pub project_type: String,
    /// 项目名称
    pub title: String,
    /// 简介
    pub description: String,
    /// 正文描述（Markdown）
    pub body: String,
    /// 最后更新时间
    pub updated: String,
    /// 下载量
    pub downloads: u64,
    /// 支持的加载器列表
    pub loaders: Vec<String>,
    /// 图标 URL
    pub icon_url: String,
    /// 分类标签
    pub categories: Vec<String>,
    /// 图库列表
    pub gallery: Vec<GalleryObj>,
}

impl Default for ModrinthProjectObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            project_type: Default::default(),
            title: Default::default(),
            description: Default::default(),
            body: Default::default(),
            updated: Default::default(),
            downloads: Default::default(),
            loaders: Default::default(),
            icon_url: Default::default(),
            categories: Default::default(),
            gallery: Default::default(),
        }
    }
}

/// 项目图库图片
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GalleryObj {
    /// 图片标题
    pub title: Option<String>,
    /// 原始图片 URL
    pub raw_url: String,
    /// 图片说明
    pub description: Option<String>,
    /// 排序序号
    pub ordering: i32,
}

impl Default for GalleryObj {
    fn default() -> Self {
        Self {
            title: Default::default(),
            raw_url: Default::default(),
            description: Default::default(),
            ordering: Default::default(),
        }
    }
}
