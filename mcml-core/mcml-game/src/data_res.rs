use std::{collections::HashMap, path::PathBuf};

use mcml_base::file_item::FileItemObj;
use mcml_names::names;

use crate::launcher::{
    FileType, file_online_info_obj::FileOnlineInfoObj, instance_setting_obj::InstanceSettingObj,
};

/// 获取下载项目信息
pub struct ItemPathRes {
    pub file_path: PathBuf,
    pub path: String,
    pub file_type: FileType,
}

/// 创建一些下载项目
pub struct DownloadItemRes {
    pub list: Vec<FileItemObj>,
    pub online: HashMap<String, FileOnlineInfoObj>,
}

impl ItemPathRes {
    pub fn change_to_resourcepacks(&mut self, game: &InstanceSettingObj) {
        self.file_path = game.get_resourcepacks_path();
        self.path = names::GAME_RESOURCEPACKS_DIR.to_string();
        self.file_type = FileType::Resourcepack;
    }

    pub fn change_to_shaderpacks(&mut self, game: &InstanceSettingObj) {
        self.file_path = game.get_shaderpacks_path();
        self.path = names::GAME_SHADERPACKS_DIR.to_string();
        self.file_type = FileType::Shaderpack;
    }

    pub fn change_to_saves(&mut self, game: &InstanceSettingObj) {
        self.file_path = game.get_saves_path();
        self.path = names::GAME_SAVES_DIR.to_string();
        self.file_type = FileType::Save;
    }

    pub fn change_to_openloader_datapack(&mut self, game: &InstanceSettingObj) {
        self.file_path = game
            .get_config_path()
            .join(names::OPEN_LOADER_DIR)
            .join(names::DATA_DIR);
        self.path = format!(
            "{}/{}/{}",
            names::GAME_CONFIG_DIR,
            names::OPEN_LOADER_DIR,
            names::DATA_DIR
        );
        self.file_type = FileType::OpenLoaderDataPack;
    }
}
