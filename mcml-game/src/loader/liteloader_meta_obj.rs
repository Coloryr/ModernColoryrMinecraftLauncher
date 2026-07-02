use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ReopObj {
    pub stream: String,
    pub url: String,
}

impl Default for ReopObj {
    fn default() -> Self {
        Self {
            stream: Default::default(),
            url: Default::default(),
        }
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LoaderObj {
    #[serde(rename = "tweakClass")]
    pub tweak_class: String,
    pub libraries: Vec<LibrariesObj>,
    // pub stream: String,
    pub file: String,
    pub version: String,
    // pub build: String,
    pub md5: String,
    // pub timestamp: String,
    // #[serde(rename = "lastSuccessfulBuild")]
    // pub last_successful_build: i32
}

impl Default for LoaderObj {
    fn default() -> Self {
        Self {
            tweak_class: Default::default(),
            libraries: Default::default(),
            file: Default::default(),
            version: Default::default(),
            md5: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SnapshotsObj {
    pub libraries: Vec<LibrariesObj>,
    #[serde(rename = "com.mumfrey:liteloader")]
    pub loader: HashMap<String, LoaderObj>,
}

impl Default for SnapshotsObj {
    fn default() -> Self {
        Self {
            libraries: Default::default(),
            loader: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LiteloaderVersionObj {
    pub repo: ReopObj,
    pub snapshots: SnapshotsObj,
    pub artefacts: SnapshotsObj,
}

impl Default for LiteloaderVersionObj {
    fn default() -> Self {
        Self {
            repo: Default::default(),
            snapshots: Default::default(),
            artefacts: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LiteloaderMetaObj {
    pub versions: HashMap<String, LiteloaderVersionObj>,
}

impl Default for LiteloaderMetaObj {
    fn default() -> Self {
        Self {
            versions: Default::default(),
        }
    }
}
