use serde::{Deserialize, Serialize};

use crate::loader::LibrariesObj;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct QuiltArgumentsObj {
    pub game: Vec<String>,
}

impl Default for QuiltArgumentsObj {
    fn default() -> Self {
        Self {
            game: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct QuiltLoaderObj {
    pub id: String,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    pub arguments: QuiltArgumentsObj,
    pub libraries: Vec<LibrariesObj>,
}

impl Default for QuiltLoaderObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            main_class: Default::default(),
            arguments: Default::default(),
            libraries: Default::default(),
        }
    }
}
