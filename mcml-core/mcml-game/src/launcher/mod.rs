use mcml_base::tools;
use serde_repr::{Deserialize_repr, Serialize_repr};

pub mod custom_game_arg_obj;
pub mod custom_loader_obj;
pub mod file_online_info_obj;
pub mod game_time_obj;
pub mod instance_setting_obj;
pub mod project_save_obj;

/// 整合包类型
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModPackType {
    CurseForge,
    Modrinth,
    McMod,
    ServerPack,
    None,
}

impl Default for ModPackType {
    fn default() -> Self {
        ModPackType::None
    }
}

impl ModPackType {
    /// 独立 ID（跨进程传输用，显示名由前端 i18n 翻译）
    pub fn id(&self) -> &'static str {
        match self {
            ModPackType::CurseForge => "curseforge",
            ModPackType::Modrinth => "modrinth",
            ModPackType::McMod => "mcmod",
            ModPackType::ServerPack => "serverpack",
            ModPackType::None => "none",
        }
    }

    /// 按 ID 解析整合包类型
    pub fn from_id(id: &str) -> Self {
        match id {
            "curseforge" => ModPackType::CurseForge,
            "modrinth" => ModPackType::Modrinth,
            "mcmod" => ModPackType::McMod,
            "serverpack" => ModPackType::ServerPack,
            _ => ModPackType::None,
        }
    }
}

/// 编码模式
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LogEncoding {
    UTF8,
    GBK,
}

impl Default for LogEncoding {
    fn default() -> Self {
        LogEncoding::UTF8
    }
}

/// 文件类型
pub enum FileType {
    Modpack,
    Mod,
    Save,
    Shaderpack,
    Resourcepack,
    DataPacks,
    Schematic,
    Java,
    Game,
    Config,
    AuthConfig,
    Pic,
    Optifine,
    Skin,
    Music,
    Text,
    GameIcon,
    Head,
    JavaZip,
    Loader,
    InputConfig,
    User,
    Cmd,
    Icon,
    StartIcon,
    File,
    OpenLoaderDataPack,
    Lang,
}

impl FileType {}

/// 检测下载源
/// - `pid`: 项目号
/// - `fid`: 文件号
pub fn get_source_type(pid: &str, fid: &str) -> ModPackType {
    if tools::check_is_not_number(pid) || tools::check_is_not_number(fid) {
        ModPackType::Modrinth
    } else {
        ModPackType::CurseForge
    }
}