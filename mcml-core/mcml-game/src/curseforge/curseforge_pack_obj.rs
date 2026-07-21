use serde::{Deserialize, Serialize};

/// CurseForge整合包数据
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgePackObj {
    pub minecraft: MinecraftObj,
    #[serde(rename = "manifestType")]
    pub manifest_type: String,
    #[serde(rename = "manifestVersion")]
    pub manifest_version: i32,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "version")]
    pub version: String,
    #[serde(rename = "author")]
    pub author: String,
    #[serde(rename = "files")]
    pub files: Vec<FilesObj>,
    #[serde(rename = "overrides")]
    pub overrides: String,
}

impl Default for CurseForgePackObj {
    fn default() -> Self {
        Self {
            minecraft: Default::default(),
            manifest_type: Default::default(),
            manifest_version: Default::default(),
            name: Default::default(),
            version: Default::default(),
            author: Default::default(),
            files: Default::default(),
            overrides: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct MinecraftObj {
    pub version: String,
    #[serde(rename = "modLoaders")]
    pub mod_loaders: Vec<ModLoadersObj>,
}

impl Default for MinecraftObj {
    fn default() -> Self {
        Self {
            version: Default::default(),
            mod_loaders: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModLoadersObj {
    pub id: String,
    pub primary: bool,
}

impl Default for ModLoadersObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            primary: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FilesObj {
    #[serde(rename = "projectID")]
    pub project_id: i64,
    #[serde(rename = "fileID")]
    pub file_id: i64,
    pub required: bool,
}

impl Default for FilesObj {
    fn default() -> Self {
        Self {
            project_id: Default::default(),
            file_id: Default::default(),
            required: Default::default(),
        }
    }
}
