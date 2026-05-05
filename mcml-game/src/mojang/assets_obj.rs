use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ObjectsObj {
    pub hash: String,
    pub size: i64,
}

impl Default for ObjectsObj {
    fn default() -> Self {
        Self {
            hash: Default::default(),
            size: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AssetsObj {
    pub objects: HashMap<String, ObjectsObj>,
}

impl Default for AssetsObj {
    fn default() -> Self {
        Self {
            objects: Default::default(),
        }
    }
}
