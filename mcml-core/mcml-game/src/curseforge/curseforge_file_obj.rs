use serde::{Deserialize, Serialize};

use crate::curseforge::curseforge_list_obj::CurseForgeListPaginationObj;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseFogreMutFileObj {
    pub data: Vec<CurseForgeFileDataObj>,
    pub pagination: CurseForgeListPaginationObj,
}

impl Default for CurseFogreMutFileObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
            pagination: Default::default(),
        }
    }
}

/// 模组信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeFileObj {
    pub data: CurseForgeFileDataObj,
}

impl Default for CurseForgeFileObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeFileDataObj {
    pub id: u64,
    #[serde(rename = "modId")]
    pub mod_id: u64,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "hashes")]
    pub hashes: Vec<HashesObj>,
    #[serde(rename = "fileDate")]
    pub file_date: String,
    #[serde(rename = "fileLength")]
    pub file_length: u64,
    #[serde(rename = "downloadCount")]
    pub download_count: u64,
    #[serde(rename = "downloadUrl")]
    pub download_url: Option<String>,
    pub dependencies: Option<Vec<DependenciesObj>>,
}

impl Default for CurseForgeFileDataObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            mod_id: Default::default(),
            display_name: Default::default(),
            file_name: Default::default(),
            hashes: Default::default(),
            file_date: Default::default(),
            file_length: Default::default(),
            download_count: Default::default(),
            download_url: Default::default(),
            dependencies: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HashesObj {
    pub value: String,
    pub algo: i32,
}

impl Default for HashesObj {
    fn default() -> Self {
        Self {
            value: Default::default(),
            algo: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct DependenciesObj {
    #[serde(rename = "modId")]
    pub mod_id: u64,
    #[serde(rename = "relationType")]
    pub relation_type: i32,
}

impl Default for DependenciesObj {
    fn default() -> Self {
        Self {
            mod_id: Default::default(),
            relation_type: Default::default(),
        }
    }
}
