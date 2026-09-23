use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FabricMetaLoaderObj {
    pub version: String,
    pub stable: bool,
}

impl Default for FabricMetaLoaderObj {
    fn default() -> Self {
        Self {
            version: Default::default(),
            stable: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct FabricMetaObj {
    pub loader: Vec<FabricMetaLoaderObj>,
}

impl Default for FabricMetaObj {
    fn default() -> Self {
        Self {
            loader: Default::default(),
        }
    }
}
