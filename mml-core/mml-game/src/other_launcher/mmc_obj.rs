use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct MMCObj {
    pub components: Vec<ComponentsObj>,
}

impl Default for MMCObj {
    fn default() -> Self {
        Self {
            components: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ComponentsObj {
    #[serde(rename = "cachedVersion")]
    pub cached_version: String,
    pub uid: String,
    pub version: String,
}

impl Default for ComponentsObj {
    fn default() -> Self {
        Self {
            cached_version: Default::default(),
            uid: Default::default(),
            version: Default::default(),
        }
    }
}
