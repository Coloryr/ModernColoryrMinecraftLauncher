//! CurseForge 版本类型 DTO

use serde::{Deserialize, Serialize};

/// CurseForge 版本类型列表
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeVersionObj {
    /// 版本类型数据列表
    pub data: Vec<CurseForgeVersionDataObj>,
}

impl Default for CurseForgeVersionObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

/// 单个版本类型分组
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeVersionDataObj {
    /// 类型标记
    #[serde(rename = "type")]
    pub verion_type: u32,
    /// 该类型下的游戏版本列表
    pub versions: Vec<String>,
}

impl Default for CurseForgeVersionDataObj {
    fn default() -> Self {
        Self {
            verion_type: Default::default(),
            versions: Default::default(),
        }
    }
}

/// CurseForge 版本类型 ID 列表
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeVersionTypeObj {
    /// 版本类型列表
    pub data: Vec<CurseForgeVersionTypeDataObj>,
}

impl Default for CurseForgeVersionTypeObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

/// 单个版本类型
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeVersionTypeDataObj {
    /// 类型 ID
    pub id: u32,
    /// 类型名称
    pub name: String,
}

impl Default for CurseForgeVersionTypeDataObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: Default::default(),
        }
    }
}
