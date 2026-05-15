use serde::{Deserialize, Serialize};

pub mod fabric_api;
pub mod fabric_loader_obj;
pub mod forge_install_obj;
pub mod forge_launch_obj;
pub mod optifine_obj;
pub mod quilt_loader_obj;

#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct LoaderKey {
    pub mc: String,
    pub version: String,
}

impl LoaderKey {
    pub fn new(mc: String, version: String) -> Self {
        LoaderKey { mc, version }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LibrariesObj {
    pub name: String,
    pub url: String,
}

impl Default for LibrariesObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            url: Default::default(),
        }
    }
}
