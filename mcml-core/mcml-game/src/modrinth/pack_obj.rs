use std::collections::HashMap;

use mcml_net::modrinth_api::version_obj::HasheObj;
use serde::{Deserialize, Serialize};

use crate::launcher::project_save_obj::McmlProjectSaveObj;

/// Modrinth整合包数据
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthPackObj {
    #[serde(rename = "formatVersion")]
    pub format_version: i32,
    #[serde(rename = "versionId")]
    pub version_id: String,
    pub name: String,
    pub summary: String,
    pub files: Vec<ModrinthPackFileObj>,
    pub dependencies: HashMap<String, String>,
}

impl Default for ModrinthPackObj {
    fn default() -> Self {
        Self {
            format_version: Default::default(),
            version_id: Default::default(),
            name: Default::default(),
            summary: Default::default(),
            files: Default::default(),
            dependencies: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthPackFileObj {
    pub path: String,
    pub hashes: HasheObj,
    pub downloads: Vec<String>,
    #[serde(rename = "fileSize")]
    pub file_size: u64,
    #[serde(rename = "_private_data", alias = "_colormc")]
    pub project: Option<McmlProjectSaveObj>,
}

impl Default for ModrinthPackFileObj {
    fn default() -> Self {
        Self {
            path: Default::default(),
            hashes: Default::default(),
            downloads: Default::default(),
            file_size: Default::default(),
            project: Default::default(),
        }
    }
}
