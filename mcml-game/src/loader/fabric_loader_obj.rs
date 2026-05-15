use serde::{Deserialize, Serialize};

use crate::loader::LibrariesObj;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FabricArgumentsObj {
    pub game: Vec<String>,
    pub jvm: Vec<String>,
}

impl Default for FabricArgumentsObj {
    fn default() -> Self {
        Self {
            game: Default::default(),
            jvm: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FabricLoaderObj {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: FabricArgumentsObj,
    pub libraries: Vec<LibrariesObj>,
}

impl Default for FabricLoaderObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            main_class: Default::default(),
            arguments: Default::default(),
            libraries: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FabricLoaderVersionItemObj {
    pub version: String,
}

impl Default for FabricLoaderVersionItemObj {
    fn default() -> Self {
        Self {
            version: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FabricLoaderVersionObj {
    pub loader: FabricLoaderVersionItemObj,
}

impl Default for FabricLoaderVersionObj {
    fn default() -> Self {
        Self {
            loader: Default::default(),
        }
    }
}
