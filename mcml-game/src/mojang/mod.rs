use mcml_base::{Os, file_item::{FileHash, FileItemObj}, get_system_info};
use mcml_net::url_helper;
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::{launcher_path::{assets_path, libraies_path, version_path}, mojang::game_arg_obj::{GameRulesObj, LoggingObj}};

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

/// 安全Log4j文件
/// - `obj`: 游戏数据
pub fn build_log4j_item(obj: &LoggingObj) -> FileItemObj {
    FileItemObj {
        name: String::from("log4j2-xml"),
        file: version_path::get_dir().join("log4j2").join("log4j2.xml"),
        url: obj.client.file.url.clone(),
        hash: FileHash::Sha1(obj.client.file.sha1.clone()),
        later: Default::default(),
    }
}

/// 创建游戏资源下载项目
/// - `name`: 名字
/// - `hash`: 校验值
pub fn build_assets_item(name: &str, hash: &str) -> FileItemObj {
    let dir: String = hash.chars().take(2).collect();
    FileItemObj {
        name: String::from(name),
        file: assets_path::get_obj_dir().join(dir).join(hash),
        url: url_helper::get_download_assets(hash),
        hash: FileHash::Sha1(String::from(hash)),
        later: Default::default(),
    }
}

/// 创建游戏本体下载项目
/// - `version`: 游戏版本号
pub fn build_game_item(version: &str) -> FileItemObj {
    let game = version_path::get_version(version).unwrap();
    let file = libraies_path::get_game_file(version);

    FileItemObj {
        name: format!("minecraft-clinet-{version}.jar"),
        file,
        url: url_helper::get_minecraft_client(&game.downloads.client.url, version),
        hash: FileHash::Sha1(game.downloads.client.sha1.clone()),
        later: Default::default(),
    }
}
