use serde::{Deserialize, Serialize};

use crate::loader::{LibrariesObj, forge_launch_obj::ForgeLibrariesObj};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ForgeInstallObj {
    pub profile: String,
    pub version: String,
    pub minecraft: String,
    pub libraries: Vec<ForgeLibrariesObj>,
}

impl Default for ForgeInstallObj {
    fn default() -> Self {
        Self {
            profile: Default::default(),
            version: Default::default(),
            minecraft: Default::default(),
            libraries: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct VersionInfoObj {
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: String,
    pub libraries: Vec<LibrariesObj>,
}

impl Default for VersionInfoObj {
    fn default() -> Self {
        Self {
            main_class: Default::default(),
            minecraft_arguments: Default::default(),
            libraries: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ForgeInstallOldObj {
    #[serde(rename = "versionInfo")]
    pub version_info: VersionInfoObj,
}

impl Default for ForgeInstallOldObj {
    fn default() -> Self {
        Self {
            version_info: Default::default(),
        }
    }
}
