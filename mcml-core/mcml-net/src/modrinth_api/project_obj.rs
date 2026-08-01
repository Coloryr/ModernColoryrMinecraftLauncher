use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthProjectObj {
    pub id: String,
    pub project_type: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub updated: String,
    pub downloads: u64,
    pub loaders: Vec<String>,
    pub icon_url: String,
    pub categories: Vec<String>,
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GalleryObj {
    pub title: String,
    pub raw_url: String,
    pub description: String,
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
