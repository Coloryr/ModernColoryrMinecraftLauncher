//! 扫描游戏版本

use std::path::{Path, PathBuf};

use mcml_base::archives::BaseArchive;
use mcml_names::names;
use mcml_sys::path_helper;

use crate::{add_game::PackType, other_launcher};

/// 扫描文件夹下的游戏版本
///
/// 然后返回可以导入的游戏实例
pub fn scan_game_from_path<P: AsRef<Path>>(path: P) -> Vec<PathBuf> {
    let mut list = Vec::new();
    let mut dirs = path_helper::get_dirs(&path);
    dirs.insert(0, path.as_ref().to_path_buf());

    for item in dirs {
        if item.ends_with(names::VERSION_DIR) {
            let dir1 = path_helper::get_dirs(&item);
            for item in dir1 {
                if other_launcher::is_minecraft_version(&item) {
                    list.push(item.to_path_buf());
                }
            }

            if !list.is_empty() {
                return list;
            }
        }

        if item.ends_with(names::INSTANCE_DIR) {
            let dir1 = path_helper::get_dirs(&item);
            for item in dir1 {
                if other_launcher::is_mmc_version(&item) {
                    list.push(item.to_path_buf());
                }
            }

            if !list.is_empty() {
                return list;
            }
        }

        if other_launcher::is_minecraft_version(&item) {
            list.push(item.to_path_buf());
            continue;
        }

        if other_launcher::is_mmc_version(&item) {
            list.push(item.to_path_buf());
        }
    }

    list
}

/// 检测压缩包类型
pub fn test_archive_type<P: AsRef<Path>>(path: P) -> Option<PackType> {
    if let Some(ext) = path.as_ref().extension()
        && ext == names::MRPACK_EXT
    {
        return Some(PackType::Modrinth);
    }

    let arch = BaseArchive::open(path);
    if arch.is_err() {
        return None;
    }

    let arch = arch.unwrap();
    for item in arch.entries() {
        if item.is_dir {
            if item.name.starts_with(".minecraft/") || item.name.ends_with(".exe") {
                return Some(PackType::LauncherPack);
            }
        } else {
            if item.name == names::GAME_FILE {
                return Some(PackType::ArchivePack);
            } else if item.name == names::HMCLFILE {
                return Some(PackType::HMCL);
            } else if item.name == names::MMCCFG_FILE {
                return Some(PackType::MMC);
            } else if item.name == names::MANIFEST_FILE {
                return Some(PackType::CurseForge);
            } else if item.name == names::SERVER_MANIFEST_FILE {
                return Some(PackType::HMCLServer);
            } else if item.name == names::MODRINTH_FILE {
                return Some(PackType::Modrinth);
            }
        }
    }

    None
}
