use std::path::PathBuf;

use mcml_base::file_item::FileHash;

use crate::launcher::{
    file_online_info_obj::FileOnlineInfoObj, instance_setting_obj::InstanceSettingObj,
};

pub enum ExportPackType {
    ColorMC,
    CurseForge,
    Modrinth,
    Zip,
}

pub struct ModExport {
    pub file: PathBuf,
    pub size: usize,
    pub pack: ExportPackType,
    pub url: Option<String>,
    pub hash: FileHash,
    pub online: Option<FileOnlineInfoObj>,
}

/// 导出需要的参数
pub struct ExportArg {
    pub file: PathBuf,
    pub pack: ExportPackType,
    pub mods: Vec<ModExport>,
}

impl InstanceSettingObj {
    pub async fn export(&self, data: ExportArg) {}
}
