//! 下载整合包窗口：在线整合包的搜索 / 版本列表 / 安装
//!
//! 安装走 `mcml_game::add_game`（CurseForge / Modrinth 各自的流程），
//! 进度与重名确认复用添加实例窗口的回调（`super::add`），因为压缩包 / 网址
//! 导入走的是同一套安装流程。

use std::collections::HashMap;
use std::sync::LazyLock;

use mcml_game::add_game;
use mcml_game::launcher::{FileType, ModPackType};
use mcml_net::curseforge_api::list_obj::CurseForgeListDataObj;
use mcml_net::curseforge_api::{self, CurseFogreArg, CurseForgeSortType};
use mcml_net::modrinth_api;
use tauri::{AppHandle, WebviewWindow};
use tokio::sync::RwLock;

use crate::collect_utils;
use crate::dtos::add_resource_dto::{ProjectDto, ProjectItemDto};
use crate::dtos::{ModpackFileDto, ModpackItemDto, ModpackSearchDto};

use super::add::{instance_gui, pack_gui, prepare};

static CURSEFOGRE_INFO: LazyLock<RwLock<HashMap<String, CurseForgeListDataObj>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static MODRINTH_INFO: LazyLock<RwLock<HashMap<String, HashMap<String, ModrinthVersionObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 搜索在线整合包（source：curseforge / modrinth；page 从 0 开始，一页 20 条）
///
/// `sort` 用排序方式的线串（见 `CurseForgeSortType::to_string` /
/// `ModrinthSortType::to_string`），未知或缺失时回退到各源的默认排序。
#[tauri::command]
pub async fn add_modpack_search(
    source: String,
    query: Option<String>,
    version: Option<String>,
    sort: Option<String>,
    page: u32,
) -> Result<ModpackSearchDto, String> {
    const PAGE_SIZE: u32 = 20;

    let sort = sort.unwrap_or_default();

    match source.as_str() {
        "curseforge" => {
            let arg = curseforge_api::CurseFogreArg {
                version,
                page: Some(page),
                sort: Some(
                    CurseForgeSortType::from_string(&sort)
                        .unwrap_or(CurseForgeSortType::Popularity),
                ),
                filter: query,
                page_size: Some(PAGE_SIZE),
                ..Default::default()
            };
            let res = curseforge_api::get_modpack_list(arg)
                .await
                .map_err(|e| e.to_string())?;
            Ok(ModpackSearchDto {
                page,
                total: res.pagination.total_count,
                items: res
                    .data
                    .iter()
                    .map(|d| ModpackItemDto {
                        id: d.id.to_string(),
                        name: d.name.clone(),
                        desc: d.summary.clone(),
                        icon: d.logo.url.clone(),
                        author: d
                            .authors
                            .first()
                            .map(|a| a.name.clone())
                            .unwrap_or_default(),
                        downloads: d.download_count,
                    })
                    .collect(),
            })
        }
        "modrinth" => {
            let arg = modrinth_api::ModrinthSearchArg {
                verions: version,
                query,
                sort: modrinth_api::ModrinthSortType::from_string(&sort)
                    .unwrap_or(modrinth_api::ModrinthSortType::Relevance),
                page: Some(page),
                page_size: Some(PAGE_SIZE),
                category: None,
                loader: None,
            };
            let res = modrinth_api::get_modpack_list(arg)
                .await
                .map_err(|e| e.to_string())?;
            Ok(ModpackSearchDto {
                page,
                total: res.total_hits as u64,
                items: res
                    .hits
                    .iter()
                    .map(|h| ModpackItemDto {
                        id: h.project_id.clone(),
                        name: h.title.clone(),
                        desc: h.description.clone(),
                        icon: h.icon_url.clone(),
                        author: h.author.clone(),
                        downloads: h.downloads,
                    })
                    .collect(),
            })
        }
        _ => Err(String::from("err.unknownSource")),
    }
}

/// 获取整合包的可安装版本列表（按游戏版本过滤，version 传 None 取全部）
#[tauri::command]
pub async fn add_modpack_files(
    source: String,
    project_id: String,
    version: Option<String>,
) -> Result<Vec<ModpackFileDto>, String> {
    match source.as_str() {
        "curseforge" => {
            let arg = curseforge_api::CurseFogreArg {
                id: Some(project_id),
                version,
                ..Default::default()
            };
            let res = curseforge_api::get_files_page(arg)
                .await
                .map_err(|e| e.to_string())?;
            Ok(res
                .data
                .iter()
                .map(|f| ModpackFileDto {
                    id: f.id.to_string(),
                    name: f.display_name.clone(),
                    file_name: f.file_name.clone(),
                    date: f.file_date.clone(),
                    size: f.file_length,
                })
                .collect())
        }
        "modrinth" => {
            let res = modrinth_api::get_file_versions(&project_id, version.as_deref(), None)
                .await
                .map_err(|e| e.to_string())?;
            Ok(res
                .iter()
                .map(|v| {
                    // 主文件优先（.mrpack），没有主文件取第一个
                    let file = v
                        .files
                        .iter()
                        .find(|f| f.primary)
                        .or_else(|| v.files.first());
                    ModpackFileDto {
                        id: v.id.clone(),
                        name: if v.name.is_empty() {
                            v.version_number.clone()
                        } else {
                            v.name.clone()
                        },
                        file_name: file
                            .map(|f| f.filename.clone())
                            .unwrap_or_else(|| v.version_number.clone()),
                        date: v.date_published.clone(),
                        size: file.map(|f| f.size).unwrap_or(0),
                    }
                })
                .collect())
        }
        _ => Err(String::from("err.unknownSource")),
    }
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
                    let mut list = curseforge_api::get_files(vec![fid])
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
                let temp = ProjectItemDto::new_curseforge(&item, FileType::Modpack.to_string(), check_modpack_download(&pid), true, collect_utils::is_star(&pid), None);

                list1.push(temp);
                map.insert(pid, item);
            }

            Ok(ProjectDto {
                items: list1,
                count: list.pagination.total_count
            })
        }
        ModPackType::Modrinth => {
            todo!()
        }
        _ => Err(String::from("err.sourceType")),
    }
}

fn check_modpack_download(pid: &str) -> bool {
    mcml_game::get_instances().iter().any(|item|  {
        let temp = item.read().unwrap();
        temp.is_modpack && match temp.pid.as_ref() {
            Some(data) => data == pid,
            None => false,
        }
    })
}