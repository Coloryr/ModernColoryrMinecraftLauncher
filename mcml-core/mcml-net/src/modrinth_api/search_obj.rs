use serde::{Deserialize, Serialize};

/// Modrinth搜索返回
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthSearchObj {
    pub total_hits: u32,
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HitObj {
    pub project_id: String,
    pub author: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub gallery: Vec<String>,
    pub downloads: u64,
    pub icon_url: String,
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
