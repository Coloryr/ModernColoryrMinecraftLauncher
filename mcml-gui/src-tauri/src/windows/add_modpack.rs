//! 下载整合包窗口：在线整合包的搜索 / 版本列表 / 安装
//!
//! 安装走 `mcml_game::add_game`（CurseForge / Modrinth 各自的流程），
//! 进度与重名确认复用添加实例窗口的回调（`super::add`），因为压缩包 / 网址
//! 导入走的是同一套安装流程。

use std::collections::HashMap;
use std::sync::LazyLock;

use mcml_game::add_game;
use mcml_game::gui_hook::AddModPackState;
use mcml_game::launcher::{FileType, ModPackType};
use mcml_net::curseforge_api::file_obj::CurseForgeFileDataObj;
use mcml_net::curseforge_api::list_obj::CurseForgeListDataObj;
use mcml_net::curseforge_api::{self, CurseFogreArg, CurseForgeSortType};
use mcml_net::modrinth_api::search_obj::HitObj;
use mcml_net::modrinth_api::version_obj::ModrinthVersionObj;
use mcml_net::modrinth_api::{self};
use mcml_net::modrinth_api::{ModrinthSearchArg, ModrinthSortType};
use tauri::{AppHandle, WebviewWindow};
use tokio::sync::RwLock;

use crate::collect_utils;
use crate::dtos::add_resource_dto::{FileListDto, FileListItemDto, ProjectDto, ProjectItemDto};
use crate::windows::add_resource::SourceInfo;

use super::add::{instance_gui, pack_gui, prepare};

static CURSEFOGRE_INFO: LazyLock<RwLock<HashMap<String, CurseForgeListDataObj>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static MODRINTH_INFO: LazyLock<RwLock<HashMap<String, HitObj>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static CURSEFOGRE_FILE: LazyLock<RwLock<HashMap<String, HashMap<String, CurseForgeFileDataObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static MODRINTH_FILE: LazyLock<RwLock<HashMap<String, HashMap<String, ModrinthVersionObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static DOWNLOAD_NOW: LazyLock<RwLock<HashMap<SourceInfo, ModPackInstallState>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 整合包安装进度
pub struct ModPackInstallState {
    pub state: AddModPackState,
    pub now: usize,
    pub all: Option<usize>,
    pub sub_text: String,
    pub sub_now: usize,
    pub sub_all: Option<usize>,
}

/// 安装在线整合包（下载压缩包后走对应类型的安装流程；name 取自整合包元数据）
#[tauri::command]
pub async fn add_modpack_install(
    window: WebviewWindow,
    app: AppHandle,
    source: String,
    project_id: String,
    file_id: String,
    group: Option<String>,
) -> Result<String, String> {
    let (_store, token) = prepare(&window)?;
    let gui = instance_gui(&window);
    let progress = pack_gui(&window);

    let uuid = tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(async {
            let res = match source.as_str() {
                "curseforge" => {
                    let fid: u64 = file_id.parse().map_err(|_| "err.badFileId".to_string())?;
                    let list = curseforge_api::get_files(vec![fid])
                        .await
                        .map_err(|e| e.to_string())?;
                    let mut data = list
                        .into_iter()
                        .next()
                        .ok_or_else(|| "err.fileNotFound".to_string())?;
                    add_game::install_curseforge(&mut data, group, None, gui, progress, None, token)
                        .await
                        .map_err(|e| e.to_string())
                }
                "modrinth" => {
                    let data = modrinth_api::get_version(&project_id, &file_id)
                        .await
                        .map_err(|e| e.to_string())?;
                    add_game::install_modrinth(&data, group, None, gui, progress, None, token)
                        .await
                        .map_err(|e| e.to_string())
                }
                _ => Err(String::from("err.unknownSource")),
            };
            res
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    crate::windows::main::emit_instance_change(&app, "add");
    Ok(uuid.to_string())
}

/// 获取项目列表
#[tauri::command]
pub async fn add_modpack_list(
    source: String,
    page: u32,
    sort: String,
    category: String,
    filter: Option<String>,
    version: Option<String>,
) -> Result<ProjectDto, String> {
    let source = ModPackType::from_string(&source);

    match source {
        ModPackType::CurseForge => {
            let sort = CurseForgeSortType::from_string(&sort);
            if sort.is_none() {
                return Err(String::from("err.sortTypeNotFound"));
            }
            let sort = sort.unwrap();

            let list = curseforge_api::get_modpack_list(CurseFogreArg {
                version,
                page: Some(page),
                sort: Some(sort),
                filter,
                category: Some(category),
                ..Default::default()
            })
            .await
            .map_err(|err| err.to_string())?;

            let mut list1 = Vec::new();

            let mut map = CURSEFOGRE_INFO.write().await;

            for item in list.data {
                let pid = item.id.to_string();
                let temp = ProjectItemDto::new_curseforge(
                    &item,
                    FileType::Modpack,
                    check_modpack_download(&pid),
                    true,
                    collect_utils::is_star(&pid),
                    check_download_now(&item.id.to_string()).await,
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

            let list = modrinth_api::get_modpack_list(ModrinthSearchArg {
                page: Some(page),
                category: Some(category),
                sort,
                version,
                ..Default::default()
            })
            .await
            .map_err(|err| err.to_string())?;

            let mut list1 = Vec::new();

            let mut map = MODRINTH_INFO.write().await;

            for item in list.hits {
                let pid = item.project_id.clone();
                let temp = ProjectItemDto::new_modrinth(
                    &item,
                    FileType::Modpack,
                    check_modpack_download(&pid),
                    true,
                    collect_utils::is_star(&pid),
                    check_download_now(&pid).await,
                    None,
                )
                .await
                .map_err(|err| err.to_string())?;

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

/// 获取文件列表
///
/// 一页50个项目
/// CurseForge换页需要查询
/// Modrinth没有换页，一次获取所有
#[tauri::command]
pub async fn add_modpack_file(
    source: String,
    pid: String,
    page: u32,
    version: Option<String>,
) -> Result<FileListDto, String> {
    let source = ModPackType::from_string(&source);

    match source {
        ModPackType::CurseForge => {
            let data = curseforge_api::get_mod_info(&pid)
                .await
                .map_err(|err| err.to_string())?;
            let list = curseforge_api::get_files_page(CurseFogreArg {
                id: Some(pid.clone()),
                version,
                page: Some(page),
                ..Default::default()
            })
            .await
            .map_err(|err| err.to_string())?;

            let mut list1 = Vec::new();

            {
                let mut lock = CURSEFOGRE_FILE.write().await;
                let list2 = lock.entry(pid.clone()).or_default();

                for item in list.data {
                    list1.push(FileListItemDto::new_curseforge(
                        &item,
                        FileType::Modpack,
                        check_modpack_version_download(&pid, &item.id.to_string()),
                        check_version_download_now(&pid, &item.id.to_string()).await,
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
                None,
            )
            .await
            .map_err(|err| err.to_string())?;

            let mut list1 = Vec::new();

            let len = list.len() as u64;

            {
                let mut lock = MODRINTH_FILE.write().await;
                let list2 = lock.entry(pid.clone()).or_default();

                for item in list {
                    list1.push(FileListItemDto::new_modrinth(
                        &item,
                        FileType::Modpack,
                        check_modpack_version_download(&pid, &item.id),
                        check_version_download_now(&pid, &item.id).await,
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

fn check_modpack_download(pid: &str) -> bool {
    mcml_game::get_instances().iter().any(|item| {
        let temp = item.read().unwrap();
        temp.is_modpack
            && match temp.pid.as_ref() {
                Some(data) => data == pid,
                None => false,
            }
    })
}

fn check_modpack_version_download(pid: &str, fid: &str) -> bool {
    mcml_game::get_instances().iter().any(|item| {
        let temp = item.read().unwrap();
        temp.is_modpack
            && match temp.pid.as_ref() {
                Some(data) => data == pid,
                None => false,
            }
            && match temp.fid.as_ref() {
                Some(data) => data == fid,
                None => false,
            }
    })
}

async fn check_download_now(pid: &str) -> bool {
    for (key, _) in DOWNLOAD_NOW.read().await.iter() {
        if key.fid == pid {
            return true;
        }
    }

    return false;
}

async fn check_version_download_now(pid: &str, fid: &str) -> bool {
    for (key, _) in DOWNLOAD_NOW.read().await.iter() {
        if key.fid == fid && key.pid == pid {
            return true;
        }
    }

    return false;
}
