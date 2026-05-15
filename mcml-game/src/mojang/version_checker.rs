use crate::{
    launch_path::version_path,
    launcher::game_setting_obj::GameSettingObj,
    mojang::{game_arg_obj::GameArgObj, version_parse::parse_game_version},
};

/// 比较两个 Minecraft 版本号
/// 返回 true 如果 version1 > version2
pub fn is_game_version_greater(v1: &str, v2: &str) -> bool {
    let parts1 = parse_game_version(v1);
    let parts2 = parse_game_version(v2);

    match (parts1, parts2) {
        (Some(p1), Some(p2)) => p1 > p2,
        _ => false,
    }
}

impl GameSettingObj {
    /// 是否为V2版本
    pub fn is_game_version_v2(&self) -> bool {
        let version = version_path::get_version(&self.version);
        match version {
            None => false,
            Some(data) => data.is_game_version_v2(),
        }
    }
}

impl GameArgObj {
    /// 是否为V2版本
    pub fn is_game_version_v2(&self) -> bool {
        self.minimum_launcher_version > 18
    }
}

/// 判断是否是 1.17 以上版本
/// - `version`: 版本号字符串
pub fn is_game_version_117(version: &String) -> bool {
    is_game_version_greater(version, "1.17") || version == "1.17"
}

/// 判断是否是 1.20.2 以上版本
/// - `version`: 版本号字符串
pub fn is_game_version_120(version: &String) -> bool {
    is_game_version_greater(version, "1.20") || version == "1.20"
}

/// 判断是否是 1.20.2 以上版本
/// - `version`: 版本号字符串
pub fn is_game_version_1202(version: &String) -> bool {
    is_game_version_greater(version, "1.20.2") || version == "1.20.2"
}
