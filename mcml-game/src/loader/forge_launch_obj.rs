use serde::{Deserialize, Serialize};

use crate::mojang::game_arg_obj::ArtifactObj;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ForgeDownloadsObj {
    pub artifact: ArtifactObj,
}

impl Default for ForgeDownloadsObj {
    fn default() -> Self {
        Self {
            artifact: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ForgeLibrariesObj {
    pub name: String,
    pub downloads: ForgeDownloadsObj,
}

impl Default for ForgeLibrariesObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            downloads: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ForgeArgumentsObj {
    pub game: Vec<String>,
    pub jvm: Option<Vec<String>>,
}

impl Default for ForgeArgumentsObj {
    fn default() -> Self {
        Self {
            game: Default::default(),
            jvm: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ForgeLaunchObj {
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: String,
    pub arguments: ForgeArgumentsObj,
    pub libraries: Vec<ForgeLibrariesObj>,
}

impl Default for ForgeLaunchObj {
    fn default() -> Self {
        Self {
            main_class: Default::default(),
            minecraft_arguments: Default::default(),
            arguments: Default::default(),
            libraries: Default::default(),
        }
    }
}
