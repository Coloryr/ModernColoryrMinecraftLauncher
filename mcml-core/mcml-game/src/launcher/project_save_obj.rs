use serde::{Deserialize, Serialize};

use crate::launcher::SourceType;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct McmlProjectSaveObj {
    #[serde(rename = "type")]
    pub source_type: SourceType,
    pub pid: String,
    pub fid: String,
}

impl Default for McmlProjectSaveObj {
    fn default() -> Self {
        Self {
            source_type: Default::default(),
            pid: Default::default(),
            fid: Default::default(),
        }
    }
}
