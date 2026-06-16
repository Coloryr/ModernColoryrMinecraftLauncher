use mcml_base::{checker::check_is_not_number, file_item::FileItemObj};

use crate::{
    launcher::SourceType,
    launcher_path::{assets_path, version_path},
    mojang::game_arg_obj::LoggingObj,
};

/// 检测下载源
/// - `pid`: 项目号
/// - `fid`: 文件号
pub fn test_source(pid: &String, fid: &String) -> SourceType {
    if check_is_not_number(&pid) || check_is_not_number(&fid) {
        SourceType::Modrinth
    } else {
        SourceType::CurseForge
    }
}

/// 安全Log4j文件
/// - `obj`: 游戏数据
pub fn build_log4j_item(obj: &LoggingObj) -> FileItemObj {
    FileItemObj {
        name: String::from("log4j2-xml"),
        local: version_path::get_dir().join("log4j2").join("log4j2.xml"),
        url: obj.client.file.url.clone(),
        sha1: obj.client.file.sha1.clone(),
    }
}

/// 游戏资源文件
/// - `name`: 名字
/// - `hash`: 校验值
pub fn build_assets_item(name: String, hash: String) -> FileItemObj {
    FileItemObj {
        name,
        local: assets_path::get_obj_dir()
            .join(hash.chars().take(2).collect())
            .join(hash),
        url: (),
        sha1: (),
    }
}
