use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct QuiltMetaLoaderObj {
    pub version: String,
}

impl Default for QuiltMetaLoaderObj {
    fn default() -> Self {
        Self {
            version: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct QuiltMetaObj {
    pub loader: Vec<QuiltMetaLoaderObj>,
}

impl Default for QuiltMetaObj {
    fn default() -> Self {
        Self {
            loader: Default::default(),
        }
    }
}
