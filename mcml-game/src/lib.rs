use std::collections::HashMap;

use crate::launcher::{
    custom_game_arg_obj::CustomGameArgObj, game_setting_obj::GameSettingObj,
    game_time_obj::GameTimeObj, mod_info_obj::FileOnlineInfoObj,
};

pub mod game_arg;
pub mod launcher;
pub mod mojang;
pub mod launch_path;
pub mod loader;

pub struct GameInstanceObj {
    pub setting: GameSettingObj,
    pub time: GameTimeObj,
    pub files: HashMap<String, FileOnlineInfoObj>,
    pub custom: HashMap<String, CustomGameArgObj>,
}

impl GameInstanceObj {}
