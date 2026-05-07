use serde::{Deserialize, Serialize};

use crate::mojang::game_arg_obj::{GameArgObj, GameLibrariesObj};

/// 自定义游戏启动配置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CustomGameArgObj {
    #[serde(flatten)]
    pub base: GameArgObj,
    #[serde(rename = "compatibleJavaMajors")]
    pub compatible_java_majors: Option<Vec<i32>>,
    pub name: String,
    pub order: i32,
    pub uid: String,
    #[serde(rename = "+tweakers")]
    pub add_tweakers: Option<Vec<String>>,
    #[serde(rename = "+jvmArgs")]
    pub add_jvm_args: Option<Vec<String>>,
    pub version: String,
    #[serde(rename = "+mainJar")]
    pub main_jar: GameLibrariesObj,
    #[serde(rename = "_minecraftVersion")]
    pub minecraft_version: Option<String>,
}

impl Default for CustomGameArgObj {
    fn default() -> Self {
        Self {
            base: Default::default(),
            compatible_java_majors: Default::default(),
            name: Default::default(),
            order: Default::default(),
            uid: Default::default(),
            add_tweakers: Default::default(),
            add_jvm_args: Default::default(),
            version: Default::default(),
            main_jar: Default::default(),
            minecraft_version: Default::default(),
        }
    }
}
