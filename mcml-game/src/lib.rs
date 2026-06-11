use std::collections::HashMap;

use crate::launcher::{
    custom_game_arg_obj::CustomGameArgObj, game_setting_obj::GameSettingObj,
    game_time_obj::GameTimeObj, mod_info_obj::FileOnlineInfoObj,
};

pub mod game_arg;
pub mod game_launch;
pub mod launcher;
pub mod launcher_path;
pub mod loader;
pub mod game_download;
pub mod mojang;

pub struct GameInstanceObj {
    pub setting: GameSettingObj,
    pub time: GameTimeObj,
    pub files: HashMap<String, FileOnlineInfoObj>,
    pub custom: HashMap<String, CustomGameArgObj>,
}

impl GameInstanceObj {}
