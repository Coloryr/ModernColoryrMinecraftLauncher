use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HMCLObj {
    pub name: String,
    pub addons: Vec<AddonsObj>,
    #[serde(rename = "launchInfo")]
    pub launch_info: Option<LaunchInfoObj>,
}

impl Default for HMCLObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            addons: Default::default(),
            launch_info: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AddonsObj {
    pub id: String,
    pub version: String,
}

impl Default for AddonsObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            version: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LaunchInfoObj {
    #[serde(rename = "minMemory")]
    pub min_memory: Option<u32>,
    #[serde(rename = "maxMemory")]
    pub max_memory: Option<u32>,
    #[serde(rename = "width")]
    pub width: Option<u16>,
    #[serde(rename = "height")]
    pub height: Option<u16>,
    #[serde(rename = "fullscreen")]
    pub fullscreen: Option<bool>,
    #[serde(rename = "environmentVariables")]
    pub environment_variables: Option<HashMap<String, String>>,
    #[serde(rename = "launchArgument")]
    pub launch_argument: Option<Vec<String>>,
    #[serde(rename = "javaArgument")]
    pub java_argument: Option<Vec<String>>,
    #[serde(rename = "quickPlayOption")]
    pub quick_play_option: Option<QuickPlayOptionObj>,
    #[serde(rename = "preLaunchCommand")]
    pub pre_launch_command: Option<String>,
    #[serde(rename = "postExitCommand")]
    pub post_exit_command: Option<String>,
}

impl Default for LaunchInfoObj {
    fn default() -> Self {
        Self {
            min_memory: Default::default(),
            max_memory: Default::default(),
            width: Default::default(),
            height: Default::default(),
            fullscreen: Default::default(),
            launch_argument: Default::default(),
            java_argument: Default::default(),
            quick_play_option: Default::default(),
            pre_launch_command: Default::default(),
            post_exit_command: Default::default(),
            environment_variables: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct QuickPlayOptionObj {}

impl Default for QuickPlayOptionObj {
    fn default() -> Self {
        Self {}
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HMCLServerObj {
    pub name: String,
    pub author: String,
    pub version: String,
    pub description: String,
    pub file_api: String,
    pub files: Vec<HMCLServerFileObj>,
    pub addons: Vec<AddonsObj>,
}

impl Default for HMCLServerObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            author: Default::default(),
            version: Default::default(),
            description: Default::default(),
            file_api: Default::default(),
            files: Default::default(),
            addons: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HMCLServerFileObj {
    pub path: String,
    pub hash: String,
}

impl Default for HMCLServerFileObj {
    fn default() -> Self {
        Self {
            path: Default::default(),
            hash: Default::default(),
        }
    }
}
