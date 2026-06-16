use std::{collections::HashMap, fmt::format};

use mcml_base::{
    checker::check_is_not_number,
    file_item::{FileHash, FileItemObj},
};
use mcml_names::{names, urls};
use mcml_net::url_helper;

use crate::{
    launcher::SourceType,
    launcher_path::{assets_path, libraies_path, version_path},
    loader::{forge_install_obj::ForgeInstallObj, forge_launch_obj::{ForgeLaunchObj, ForgeLibrariesObj}},
    mojang::{game_arg_obj::LoggingObj, version_checker},
};

/// 检测下载源
/// - `pid`: 项目号
/// - `fid`: 文件号
pub fn test_source(pid: &str, fid: &str) -> SourceType {
    if check_is_not_number(pid) || check_is_not_number(fid) {
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
        file: version_path::get_dir().join("log4j2").join("log4j2.xml"),
        url: obj.client.file.url.clone(),
        hash: FileHash::Sha1(obj.client.file.sha1.clone()),
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
    }
}

/// 创建Forge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `jar`: 类型
fn build_forge_item(mc: &str, version: &str, jar: &str) -> FileItemObj {
    let mut version = String::from(version);
    version.push_str(&url_helper::forge_url_fix(&mc));

    FileItemObj {
        name: format!("net.minecraftforge:{mc}-{version}-{jar}"),
        file: libraies_path::get_base_dir()
            .join("net")
            .join("minecraftforge")
            .join("forge")
            .join(format!("{mc}-{version}"))
            .join(format!("forge-{mc}-{version}-{jar}.jar")),
        url: url_helper::get_forge_jar(mc, &version),
        hash: FileHash::None,
    }
}

/// 创建Forge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `jar`: 类型
fn build_neoforge_item(mc: &str, version: &str, jar: &str) -> FileItemObj {
    let v2222 = version_checker::is_game_version_1202(mc);
    let name = if v2222 {
        format!("neoforge-{version}-{jar}")
    } else {
        format!("forge-{mc}-{version}-{jar}")
    };

    let base_url = url_helper::get_neoforge_jar(v2222, mc, version);
    let base_path = libraies_path::get_base_dir().join("net").join("neoforged");

    FileItemObj {
        name: format!("net.neoforged:{name}"),
        file: if v2222 {
            base_path
                .join("neoforge")
                .join(version)
                .join(format!("{name}.jar"))
        } else {
            base_path
                .join("forge")
                .join(format!("{mc}-{version}"))
                .join(format!("{name}.jar"))
        },
        url: format!("{base_url}{name}.jar"),
        hash: FileHash::None,
    }
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_forge_installer(mc: &str, version: &str) -> FileItemObj {
    build_forge_item(mc, version, names::FILE_INSTALLER)
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_forge_universal(mc: &str, version: &str) -> FileItemObj {
    build_forge_item(mc, version, names::FILE_UNIVERSAL)
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_forge_client(mc: &str, version: &str) -> FileItemObj {
    build_forge_item(mc, version, names::FILE_CLIENT)
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_forge_launcher(mc: &str, version: &str) -> FileItemObj {
    let mut item = build_forge_item(mc, version, names::FILE_LAUNCHER);
    item.url = format!(
        "{}net/minecraftforge/forge/{mc}-{version}/forge-{mc}-{version}-{}.jar",
        urls::FORGE,
        names::FILE_LAUNCHER
    );
    item
}

/// 创建NeoForge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_neoforge_installer(mc: &str, version: &str) -> FileItemObj {
    build_neoforge_item(mc, version, names::FILE_INSTALLER)
}

/// 创建NeoForge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_neoforge_universal(mc: &str, version: &str) -> FileItemObj {
    build_neoforge_item(mc, version, names::FILE_UNIVERSAL)
}

/// 创建NeoForge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_neoforge_client(mc: &str, version: &str) -> FileItemObj {
    build_neoforge_item(mc, version, names::FILE_CLIENT)
}

/// 构建Forge运行库下载项目列表
/// - `info`: 运行库列表
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `neo`: 是否为NeoForge
/// - `v2`: 是否为1.13以上
/// - `install`: 是否为安装器
pub fn build_forge_libs(
    info: &Vec<ForgeLibrariesObj>,
    mc: &str,
    version: &str,
    neo: bool,
    v2: bool,
    install: bool,
) -> Vec<FileItemObj> {
    let mut list = HashMap::<String, FileItemObj>::new();

    let mut universal = false;
    let mut installer = false;
    let mut launcher = false;

    for item in info {
        if item.downloads.artifact.path.is_empty() {
            continue;
        }

        let obj = if item.downloads.artifact.url.is_empty() {
            FileItemObj {
                name: item.name.clone(),
                file: libraies_path::get_base_dir().join(&item.downloads.artifact.path),
                url: Default::default(),
                hash: FileHash::Sha1(item.downloads.artifact.sha1.clone()),
            }
        } else {
            FileItemObj {
                name: item.name.clone(),
                file: libraies_path::get_base_dir().join(&item.downloads.artifact.path),
                url: item.downloads.artifact.url.clone(),
                hash: FileHash::Sha1(item.downloads.artifact.sha1.clone()),
            }
        };

        list.insert(item.name.clone(), obj);

        if item.name.ends_with(names::FILE_UNIVERSAL) {
            universal = true;
        } else if item.name.ends_with(names::FILE_INSTALLER) {
            installer = true;
        } else if item.name.ends_with(names::FILE_LAUNCHER) {
            launcher = true;
        }
    }

    if !installer && install {
        list.insert(
            String::from(names::FILE_INSTALLER),
            if neo {
                build_neoforge_installer(mc, version)
            } else {
                build_forge_installer(mc, version)
            },
        );
    }

    if !universal {
        if !neo || version_checker::is_game_version_1202(mc) {
            list.insert(
                String::from(names::FILE_UNIVERSAL),
                if neo {
                    build_neoforge_universal(mc, version)
                } else {
                    build_forge_universal(mc, version)
                },
            );
        }
    }

    if v2 && !version_checker::is_game_version_117(mc) && !launcher {
        list.insert(
            String::from(names::FILE_LAUNCHER),
            build_forge_launcher(mc, version),
        );
    }

    let mut list1 = Vec::<FileItemObj>::new();

    for (_, item) in list.drain() {
        list1.push(item);
    }

    list1
}

/// 从启动信息构建Forge运行库下载项目列表
/// - `info`: 运行库列表
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `neo`: 是否为NeoForge
/// - `v2`: 是否为1.13以上
/// - `install`: 是否为安装器
pub fn build_forge_libs_from_info(info: &ForgeLaunchObj,
    mc: &str,
    version: &str,
    neo: bool,
    v2: bool,
    install: bool,) -> Vec<FileItemObj> {
    build_forge_libs(&info.libraries, mc, version, neo, v2, install)
}

/// 从安装信息构建Forge运行库下载项目列表
/// - `info`: 运行库列表
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `neo`: 是否为NeoForge
/// - `v2`: 是否为1.13以上
/// - `install`: 是否为安装器
pub fn build_forge_libs_from_install(info: &ForgeInstallObj,
    mc: &str,
    version: &str,
    neo: bool,
    v2: bool,
    install: bool,) -> Vec<FileItemObj> {
    build_forge_libs(&info.libraries, mc, version, neo, v2, install)
}
