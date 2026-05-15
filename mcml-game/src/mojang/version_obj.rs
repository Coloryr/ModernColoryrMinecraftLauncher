use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LastestObj {
    pub release: String,
}

impl Default for LastestObj {
    fn default() -> Self {
        Self {
            release: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct VersionsObj {
    pub id: String,
    #[serde(rename = "type")]
    pub version_type: String,
    pub url: String,
    pub sha1: String,
}

impl Default for VersionsObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            version_type: Default::default(),
            url: Default::default(),
            sha1: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct VersionObj {
    pub latest: LastestObj,
    pub versions: Vec<VersionsObj>,
}

impl Default for VersionObj {
    fn default() -> Self {
        Self {
            latest: Default::default(),
            versions: Default::default(),
        }
    }
}
