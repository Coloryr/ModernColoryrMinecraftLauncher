//! 添加资源窗口的列表查询：文件列表（某项目的版本）与项目列表（搜索 / 分类翻页）
//!
//! 从 `add_resource/mod.rs` 拆出来的两个大函数（135 / 160 行）。列表页顺手把项目与文件
//! 数据写进 `CURSEFOGRE_*` / `MODRINTH_*` 缓存，详情与下载都从缓存里取。

use mml_game::launcher::{FileType, ModPackType};
use mml_game::loader::LoaderType;
use mml_game::{curseforge, modrinth};
use mml_names::i18_items::error_type::ErrorType;
use mml_net::curseforge_api::{self, CurseFogreArg, CurseForgeSortType};
use mml_net::modrinth_api::{self, ModrinthSearchArg, ModrinthSortType};
use uuid::Uuid;

use crate::collect_utils;
use crate::dtos::add_resource_dto::{FileListDto, FileListItemDto, ProjectDto, ProjectItemDto};

use super::{CURSEFOGRE_FILE, CURSEFOGRE_INFO, DOWNLOAD_NOW, MODRINTH_FILE, MODRINTH_INFO};

/// 获取文件列表
///
/// 一页50个项目
/// CurseForge换页需要查询
/// Modrinth没有换页，一次获取所有
#[gui_macros::ipc_group("add_resource")]
#[tauri::command]
pub async fn add_resource_file(
    game: String,
    source: String,
    pid: String,
    file_type: String,
    page: u32,
    version: Option<String>,
    loader: Option<String>,
) -> Result<FileListDto, String> {
    let uuid = Uuid::parse_str(&game);
    if uuid.is_err() {
        return Err(String::from("err.uuid"));
    }
    let uuid = uuid.unwrap();
    let game = mml_game::get_instance(&uuid);
    if game.is_none() {
        return Err(String::from("err.gameNotFound"));
    }
    let game = game.unwrap();
    let info = game.read().unwrap().read_online_info();
    let source = ModPackType::from_string(&source);
    let file_type = FileType::from_string(&file_type);
    if file_type.is_none() {
        return Err(String::from("err.fileTypeNotFound"));
    }
    let file_type = file_type.unwrap();
    // loader 只对模组有意义，其余类型一律 Normal（缺省 / 非法值也视为 normal）
    let mod_loader = if matches!(file_type, FileType::Mod) {
        LoaderType::from_string(&loader.clone().unwrap_or_default()).unwrap_or(LoaderType::Normal)
    } else {
        LoaderType::Normal
    };

    match source {
        ModPackType::CurseForge => {
            let data = curseforge_api::get_mod_info(&pid)
                .await
                .map_err(|err| err.to_string())?;
            let list = curseforge_api::get_files_page(CurseFogreArg {
                id: Some(pid.clone()),
                version,
                page: Some(page),
                loader: curseforge::to_loader_id(&mod_loader),
                ..Default::default()
            })
            .await
            .map_err(|err| err.to_string())?;

            let mut list1 = Vec::new();

            {
                let mut lock = CURSEFOGRE_FILE.write().await;
                let list2 = lock.entry(pid.clone()).or_default();

                for item in list.data {
                    let info = info.get(&item.mod_id.to_string());
                    let download = match info {
                        Some(data) => data.fileid == item.id.to_string(),
                        None => false,
                    };

                    list1.push(FileListItemDto::new_curseforge(
                        &item,
                        file_type,
                        download,
                        check_version_download_now(&uuid, &pid, &item.id.to_string()),
                    ));
                    list2.insert(item.id.to_string(), item);
                }
            }

            Ok(FileListDto {
                list: list1,
                count: list.pagination.total_count,
                name: data.data.name,
                max_page: list.pagination.total_count / 50,
            })
        }
        ModPackType::Modrinth => {
            let data = modrinth_api::get_project(&pid)
                .await
                .map_err(|err| err.to_string())?;

            let list = modrinth_api::get_file_versions(
                &pid,
                match version.as_ref() {
                    Some(data) => Some(data),
                    None => None,
                },
                match loader.as_ref() {
                    Some(data) => Some(data),
                    None => None,
                },
            )
            .await
            .map_err(|err| err.to_string())?;

            let mut list1 = Vec::new();

            let len = list.len() as u64;

            {
                let mut lock = MODRINTH_FILE.write().await;
                let list2 = lock.entry(pid.clone()).or_default();

                for item in list {
                    let info = info.get(&item.project_id);
                    let download = match info {
                        Some(data) => data.fileid == item.id,
                        None => false,
                    };

                    // 没有文件可下的版本直接跳过（见 new_modrinth 的注释）
                    if let Some(dto) = FileListItemDto::new_modrinth(
                        &item,
                        file_type,
                        download,
                        check_version_download_now(&uuid, &pid, &item.id),
                    ) {
                        list1.push(dto);
                    }
                    list2.insert(item.id.to_string(), item);
                }
            }

            Ok(FileListDto {
                list: list1,
                count: len,
                name: data.title,
                max_page: len / 50,
            })
        }
        _ => Err(String::from("err.sourceType")),
    }
}

/// 获取项目列表
///
/// 参数就是 IPC 契约（`bindings.ts` 由 Rust 源码生成，见 AGENTS.md §4），
/// 不能为了少几个参数把它们并成结构体 —— 那等于改前端调用方式。
#[allow(clippy::too_many_arguments)]
#[gui_macros::ipc_group("add_resource")]
#[tauri::command]
pub async fn add_resource_list(
    game: String,
    source: String,
    file_type: String,
    page: u32,
    sort: String,
    category: Option<String>,
    filter: Option<String>,
    version: Option<String>,
    loader: Option<String>,
) -> Result<ProjectDto, String> {
    let uuid = Uuid::parse_str(&game);
    if uuid.is_err() {
        return Err(String::from("err.uuid"));
    }
    let uuid = uuid.unwrap();
    let game = mml_game::get_instance(&uuid);
    if game.is_none() {
        return Err(String::from("err.gameNotFound"));
    }
    let game = game.unwrap();
    let info = game.read().unwrap().read_online_info();
    let source = ModPackType::from_string(&source);
    let file_type = FileType::from_string(&file_type);
    if file_type.is_none() {
        return Err(String::from("err.fileTypeNotFound"));
    }
    let file_type = file_type.unwrap();
    // loader 只对模组有意义，其余类型一律 Normal（缺省 / 非法值也视为 normal）
    let mod_loader = if matches!(file_type, FileType::Mod) {
        LoaderType::from_string(&loader.clone().unwrap_or_default()).unwrap_or(LoaderType::Normal)
    } else {
        LoaderType::Normal
    };

    match source {
        ModPackType::CurseForge => {
            let sort = CurseForgeSortType::from_string(&sort);
            if sort.is_none() {
                return Err(String::from("err.sortTypeNotFound"));
            }
            let sort = sort.unwrap();

            let arg = CurseFogreArg {
                version,
                page: Some(page),
                sort: Some(sort),
                filter,
                category,
                loader: curseforge::to_loader_id(&mod_loader),
                ..Default::default()
            };

            let list = match file_type {
                FileType::Mod => curseforge_api::get_mod_list(arg).await,
                FileType::Save => curseforge_api::get_save_list(arg).await,
                FileType::Resourcepack => curseforge_api::get_resourcepack_list(arg).await,
                FileType::Shaderpack => curseforge_api::get_shaders_list(arg).await,
                FileType::DataPacks => curseforge_api::get_datapacks_list(arg).await,
                _ => Err(ErrorType::InvalidOperation),
            }
            .map_err(|err| err.to_string())?;

            let mut list1 = Vec::new();

            let mut map = CURSEFOGRE_INFO.write().await;

            for item in list.data {
                let pid = item.id.to_string();
                let info = info.get(&pid);

                let temp = ProjectItemDto::new_curseforge(
                    &item,
                    file_type,
                    info.is_some(),
                    true,
                    collect_utils::is_star(&pid),
                    check_download_now(&uuid, &item.id.to_string()),
                    None,
                );

                list1.push(temp);
                map.insert(pid, item);
            }

            Ok(ProjectDto {
                items: list1,
                count: list.pagination.total_count,
            })
        }
        ModPackType::Modrinth => {
            let sort = ModrinthSortType::from_string(&sort);
            if sort.is_none() {
                return Err(String::from("err.sortTypeNotFound"));
            }
            let sort = sort.unwrap();

            // 有一项分类需要输入文本才能搜索
            // if matches!(sort, ModrinthSortType::Downloads)

            let arg = ModrinthSearchArg {
                page: Some(page),
                category,
                query: filter,
                sort,
                version,
                loader: modrinth::to_loader_id(&mod_loader),
                ..Default::default()
            };

            // Modrinth 没有存档（world）类型
            let list = match file_type {
                FileType::Mod => modrinth_api::get_mod_list(arg).await,
                FileType::Resourcepack => modrinth_api::get_resourcepack_list(arg).await,
                FileType::Shaderpack => modrinth_api::get_shaderpack_list(arg).await,
                FileType::DataPacks => modrinth_api::get_datapack_list(arg).await,
                _ => Err(ErrorType::InvalidOperation),
            }
            .map_err(|err| err.to_string())?;

            let mut list1 = Vec::new();
            // 缓存先攒着，**构建 DTO 期间不持锁**：new_modrinth 里会 await 查
            // 标签图标（首次可能真发网络请求），持全局写锁跨 await 会把其它
            // 窗口读同一份缓存的请求一起堵住。攒完最后一次性写入。
            let mut cached = Vec::new();

            for item in list.hits {
                let pid = item.project_id.clone();
                let info = info.get(&pid);

                let temp = ProjectItemDto::new_modrinth(
                    &item,
                    file_type,
                    info.is_some(),
                    true,
                    collect_utils::is_star(&pid),
                    check_download_now(&uuid, &pid),
                    None,
                )
                .await;

                list1.push(temp);
                cached.push((pid, item));
            }

            {
                let mut map = MODRINTH_INFO.write().await;
                for (pid, item) in cached {
                    map.insert(pid, item);
                }
            }

            Ok(ProjectDto {
                items: list1,
                count: list.total_hits,
            })
        }
        _ => Err(String::from("err.sourceType")),
    }
}

/// 项目是否正在下载（该实例任务表里有同 pid 的条目，列表角标用）
fn check_download_now(game: &Uuid, pid: &str) -> bool {
    let read = DOWNLOAD_NOW.read().unwrap();
    let list = read.get(game);
    if list.is_none() {
        return false;
    }
    for (key, _) in list.unwrap().iter() {
        if key.pid == pid {
            return true;
        }
    }

    false
}

/// 具体文件是否正在下载（该实例任务表里有同 pid + fid 的条目）
fn check_version_download_now(game: &Uuid, pid: &str, fid: &str) -> bool {
    let read = DOWNLOAD_NOW.read().unwrap();
    let list = read.get(game);
    if list.is_none() {
        return false;
    }
    for (key, _) in list.unwrap().iter() {
        if key.fid == fid && key.pid == pid {
            return true;
        }
    }

    false
}
