use mcml_base::{Os, get_system_info};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::mojang::game_arg_obj::GameRulesObj;

pub mod assets_obj;
pub mod game_arg_obj;
pub mod version_checker;
pub mod version_obj;
pub mod version_parse;

/// 游戏版本类型
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GameType {
    /// 发布版
    Release,
    Snapshot,
    Other,
    All,
}

impl Default for GameType {
    fn default() -> Self {
        GameType::Release
    }
}

/// 检查规则是否适用
/// - `list`: 规则列表
pub fn check_allow(list: &Vec<GameRulesObj>) -> bool {
    let mut allow = true;
    let sys = get_system_info();
    for item in list.iter() {
        if item.action == "allow" {
            if let Some(os) = &item.os {
                if os.name == "osx" && sys.os == Os::MacOS {
                    allow = true;
                } else if os.name == "windows" && sys.os == Os::Windows {
                    allow = true;
                } else if os.name == "linux" && sys.os == Os::Linux {
                    allow = true;
                } else {
                    allow = false;
                }

                if os.arch == "x86" && !sys.is_arm {
                    allow = true;
                }
            } else {
                allow = true;
            }
        } else if item.action == "disallow" {
            if let Some(os) = &item.os {
                if os.name == "osx" && sys.os == Os::MacOS {
                    allow = false;
                } else if os.name == "windows" && sys.os == Os::Windows {
                    allow = false;
                } else if os.name == "linux" && sys.os == Os::Linux {
                    allow = false;
                } else {
                    allow = true;
                }

                if os.arch == "x86" && !sys.is_arm {
                    allow = false;
                }
            } else {
                allow = false;
            }
        }
    }

    allow
}
