//! 添加资源窗口：从 CurseForge / Modrinth 浏览并获取资源
//!
//! 与「下载整合包」窗口（`add_modpack`）分开：这里按资源类型
//! （模组 / 资源包 / 光影包等）走通用的项目与文件列表查询。

use std::{collections::HashMap, sync::LazyLock};

use mcml_game::{
    curseforge,
    launcher::{FileType, ModPackType},
    loader::LoaderType,
    modrinth,
};
use mcml_names::i18_items::error_type::ErrorType;
use mcml_net::{
    curseforge_api::{
        self, CurseFogreArg, CurseForgeSortType, file_obj::CurseForgeFileDataObj,
        list_obj::CurseForgeListDataObj,
    },
    modrinth_api::{
        self, ModrinthSearchArg, ModrinthSortType, search_obj::HitObj,
        version_obj::ModrinthVersionObj,
    },
};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::{collect_utils, dtos::add_resource_dto::{FileListDto, FileListItemDto, ProjectDto, ProjectItemDto}};

static CURSEFOGRE_INFO: LazyLock<RwLock<HashMap<String, CurseForgeListDataObj>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static MODRINTH_INFO: LazyLock<RwLock<HashMap<String, HitObj>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static CURSEFOGRE_FILE: LazyLock<RwLock<HashMap<String, HashMap<String, CurseForgeFileDataObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static MODRINTH_FILE: LazyLock<RwLock<HashMap<String, HashMap<String, ModrinthVersionObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static DOWNLOAD_NOW: LazyLock<RwLock<HashMap<Uuid, HashMap<SourceInfo, SourceDownloadInfo>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SourceInfo {
    pub pid: String,
    pub fid: String,
}

/// 资源下载进度
pub struct SourceDownloadInfo {
    /// 下载进度
    pub now: f32,
}

/// 获取下载源
#[tauri::command]
pub fn add_resource_source_type() -> Result<Vec<String>, String> {
    Ok(vec![
        ModPackType::Modrinth.to_string(),
        ModPackType::CurseForge.to_string(),
    ])
}

/// 获取分组
#[tauri::command]
pub async fn add_resource_categories(
    source: String,
    file_type: String,
) -> Result<HashMap<String, String>, String> {
    let source = ModPackType::from_string(&source);
    let file_type = FileType::from_string(&file_type);
    if file_type.is_none() {
        return Err(String::from("err.fileTypeNotFound"));
    }
    let file_type = file_type.unwrap();

    match source {
        ModPackType::CurseForge => curseforge::get_categories(file_type)
            .await
            .map_err(|err| err.to_string()),
        ModPackType::Modrinth => modrinth::get_categories(file_type)
            .await
            .map_err(|err| err.to_string()),
        _ => Err(String::from("err.sourceType")),
    }
}

/// 获取排序方式
#[tauri::command]
pub fn add_resource_sort_type(source: String) -> Result<Vec<String>, String> {
    let source = ModPackType::from_string(&source);

    match source {
        ModPackType::CurseForge => Ok(vec![
            CurseForgeSortType::Popularity.to_string(),
            CurseForgeSortType::Featured.to_string(),
            CurseForgeSortType::LastUpdated.to_string(),
            CurseForgeSortType::Name.to_string(),
            CurseForgeSortType::TotalDownloads.to_string(),
        ]),
        ModPackType::Modrinth => Ok(vec![
            ModrinthSortType::Relevance.to_string(),
            ModrinthSortType::Downloads.to_string(),
            ModrinthSortType::Follows.to_string(),
            ModrinthSortType::Newest.to_string(),
            ModrinthSortType::Updated.to_string(),
        ]),
        _ => Err(String::from("err.sourceType")),
    }
}

/// 获取支持的游戏版本列表
#[tauri::command]
pub async fn add_resource_game_versions(source: String) -> Result<Vec<String>, String> {
    let source = ModPackType::from_string(&source);

    match source {
        ModPackType::CurseForge => curseforge::get_game_versions()
            .await
            .map_err(|err| err.to_string()),
        ModPackType::Modrinth => modrinth::get_game_versions()
            .await
            .map_err(|err| err.to_string()),
        _ => Err("err.sourceType".to_string()),
    }
}

/// 获取文件列表
///
/// 一页50个项目
/// CurseForge换页需要查询
/// Modrinth没有换页，一次获取所有
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
    let game = mcml_game::get_instance(&uuid);
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
    let mod_loader = LoaderType::from_string(&loader.clone().unwrap_or_default());
    if mod_loader.is_none() {
        return Err(String::from("err.fileTypeNotFound"));
    }
    let mod_loader = if matches!(file_type, FileType::Mod) {
        mod_loader.unwrap()
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
                version: version,
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
                        check_version_download_now(&uuid, &pid, &item.id.to_string()).await,
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

                    list1.push(FileListItemDto::new_modrinth(
                        &item,
                        file_type,
                        download,
                        check_version_download_now(&uuid, &pid, &item.id).await,
                    ));
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
    let game = mcml_game::get_instance(&uuid);
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
    let mod_loader = LoaderType::from_string(&loader.clone().unwrap_or_default());
    if mod_loader.is_none() {
        return Err(String::from("err.fileTypeNotFound"));
    }
    let mod_loader = if matches!(file_type, FileType::Mod) {
        mod_loader.unwrap()
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

            let list = match file_type {
                FileType::Mod => {
                    curseforge_api::get_mod_list(CurseFogreArg {
                        version,
                        page: Some(page),
                        sort: Some(sort),
                        filter,
                        category,
                        loader: curseforge::to_loader_id(&mod_loader),
                        ..Default::default()
                    })
                    .await
                }
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
                    FileType::Modpack,
                    info.is_some(),
                    true,
                    collect_utils::is_star(&pid),
                    check_download_now(&uuid, &item.id.to_string()).await,
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

            let list = modrinth_api::get_modpack_list(ModrinthSearchArg {
                page: Some(page),
                category,
                query: filter,
                sort,
                version,
                loader: modrinth::to_loader_id(&mod_loader),
                ..Default::default()
            })
            .await
            .map_err(|err| err.to_string())?;

            let mut list1 = Vec::new();

            let mut map = MODRINTH_INFO.write().await;

            for item in list.hits {
                let pid = item.project_id.clone();
                let info = info.get(&pid);

                let temp = ProjectItemDto::new_modrinth(
                    &item,
                    FileType::Modpack,
                    info.is_some(),
                    true,
                    collect_utils::is_star(&pid),
                    check_download_now(&uuid, &pid).await,
                    None,
                )
                .await;

                list1.push(temp);
                map.insert(pid, item);
            }

            Ok(ProjectDto {
                items: list1,
                count: list.total_hits,
            })
        }
        _ => Err(String::from("err.sourceType")),
    }
}

async fn check_download_now(game: &Uuid, pid: &str) -> bool {
    let read = DOWNLOAD_NOW.read().await;
    let list = read.get(game);
    if list.is_none() {
        return false;
    }
    for (key, _) in list.unwrap().iter() {
        if key.fid == pid {
            return true;
        }
    }

    return false;
}

async fn check_version_download_now(game: &Uuid, pid: &str, fid: &str) -> bool {
    let read = DOWNLOAD_NOW.read().await;
    let list = read.get(game);
    if list.is_none() {
        return false;
    }
    for (key, _) in list.unwrap().iter() {
        if key.fid == fid && key.pid == pid {
            return true;
        }
    }

    return false;
}
