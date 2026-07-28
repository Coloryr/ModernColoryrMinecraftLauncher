use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeVersionObj {
    pub data: Vec<CurseForgeVersionDataObj>,
}

impl Default for CurseForgeVersionObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeVersionDataObj {
    #[serde(rename = "type")]
    pub verion_type: u32,
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeVersionTypeObj {
    pub data: Vec<CurseForgeVersionTypeDataObj>,
}

impl Default for CurseForgeVersionTypeObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeVersionTypeDataObj {
    pub id: String,
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
