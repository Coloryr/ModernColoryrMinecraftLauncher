//! 添加资源窗口：从 CurseForge / Modrinth 浏览并获取资源
//!
//! 与「下载整合包」窗口（`add_modpack`）分开：这里按资源类型
//! （模组 / 资源包 / 光影包等）走通用的项目与文件列表查询。

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{LazyLock, RwLock},
    time::Duration,
};

use mml_base::file_item::FileItemObj;
use mml_downloader::download_item::DownloadItem;
use mml_game::{
    curseforge,
    launcher::{FileType, ModPackType, file_online_info_obj::OnlineInfoObj},
    loader::LoaderType,
    modrinth,
};
use mml_names::{i18_items::error_type::ErrorType, names};
use mml_net::{
    curseforge_api::{
        self, CurseFogreArg, CurseForgeSortType, file_obj::CurseForgeFileDataObj,
        list_obj::CurseForgeListDataObj,
    },
    modrinth_api::{
        self, ModrinthSearchArg, ModrinthSortType, search_obj::HitObj,
        version_obj::ModrinthVersionObj,
    },
};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock as AsyncRwLock;
use uuid::Uuid;

use crate::{
    collect_utils,
    dtos::add_resource_dto::{
        FileListDto, FileListItemDto, ProjectDto, ProjectItemDto, ResourceSaveDto,
        ResourceStatusDto, ResourceTaskDto,
    },
    listens,
};

static CURSEFOGRE_INFO: LazyLock<AsyncRwLock<HashMap<String, CurseForgeListDataObj>>> =
    LazyLock::new(|| AsyncRwLock::new(HashMap::new()));
static MODRINTH_INFO: LazyLock<AsyncRwLock<HashMap<String, HitObj>>> =
    LazyLock::new(|| AsyncRwLock::new(HashMap::new()));

static CURSEFOGRE_FILE: LazyLock<AsyncRwLock<HashMap<String, HashMap<String, CurseForgeFileDataObj>>>> =
    LazyLock::new(|| AsyncRwLock::new(HashMap::new()));
static MODRINTH_FILE: LazyLock<AsyncRwLock<HashMap<String, HashMap<String, ModrinthVersionObj>>>> =
    LazyLock::new(|| AsyncRwLock::new(HashMap::new()));

/// 资源下载任务表（实例 → 下载条目）。
/// 下载器的回调是同步的（来自下载线程），这里用 std 锁；命令侧临界区都很短。
static DOWNLOAD_NOW: LazyLock<RwLock<HashMap<Uuid, HashMap<SourceInfo, SourceDownloadInfo>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct SourceInfo {
    pub pid: String,
    pub fid: String,
}

/// 资源下载条目（添加资源窗口顶部进度条的数据源）
pub struct SourceDownloadInfo {
    /// 目标文件路径（下载器回调按路径匹配进度）
    pub file: PathBuf,
    /// 显示名
    pub name: String,
    /// 下载进度（0.0–100.0）
    pub now: f64,
    /// 下载完成（保留一段时间后从任务表移除）
    pub done: bool,
    /// 下载失败
    pub failed: bool,
}

/// 资源下载任务总览快照（事件负载 / 查询返回）
fn build_status(app: &AppHandle) -> ResourceStatusDto {
    let map = DOWNLOAD_NOW.read().unwrap();
    ResourceStatusDto {
        window_open: app.get_webview_window("mml-add_resource").is_some(),
        tasks: map
            .values()
            .flat_map(|entry| {
                entry.iter().map(|(key, info)| ResourceTaskDto {
                    pid: key.pid.clone(),
                    fid: key.fid.clone(),
                    name: info.name.clone(),
                    progress: info.now,
                    done: info.done,
                    failed: info.failed,
                })
            })
            .collect(),
    }
}

/// 资源下载任务总览事件（任务增删 / 进度变化 / 移除时广播）
#[gui_macros::emit]
pub fn emit_add_resource_status(app: &AppHandle, dto: ResourceStatusDto) {
    let _ = app.emit(listens::ADD_RESOURCE_STATUS, dto);
}

/// 查询资源下载任务总览（挂载时同步一次，之后靠事件）
#[tauri::command]
pub fn add_resource_status(app: AppHandle) -> ResourceStatusDto {
    build_status(&app)
}

/// 下载器回调（DownloadGuiHook 转发）：按目标文件路径匹配资源下载条目并更新进度。
/// 匹配不到（整合包 / 游戏文件等其它下载）直接返回，无额外开销。
pub(crate) fn on_download_item(file: &std::sync::Arc<DownloadItem>, app: &AppHandle) {
    let total = file.get_all_size();
    if total == 0 {
        return;
    }
    let progress = file.progress().min(100.0);
    let path = &file.base.file;

    let changed = {
        let mut map = DOWNLOAD_NOW.write().unwrap();
        let mut changed = false;
        for entry in map.values_mut() {
            for info in entry.values_mut() {
                // 进度变化超过 0.5% 才算变化，避免按块刷事件
                if info.file == *path
                    && !info.done
                    && !info.failed
                    && (progress - info.now).abs() >= 0.5
                {
                    info.now = progress;
                    changed = true;
                }
            }
        }
        changed
    };

    if changed {
        let dto = build_status(app);
        let _ = app.emit(listens::ADD_RESOURCE_STATUS, dto);
    }
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
        // CurseForge 的数据包走固定 categoryId，不支持再按分类过滤
        ModPackType::CurseForge if file_type == FileType::DataPacks => Ok(HashMap::new()),
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

                    list1.push(FileListItemDto::new_modrinth(
                        &item,
                        file_type,
                        download,
                        check_version_download_now(&uuid, &pid, &item.id),
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

            let mut map = MODRINTH_INFO.write().await;

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

    return false;
}

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

    return false;
}

/// 获取实例的存档列表（下载数据包时选择目标存档）
#[tauri::command]
pub async fn add_resource_saves(game: String) -> Result<Vec<ResourceSaveDto>, String> {
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

    // get_saves 是异步方法，但 std 锁卫不能跨 await（future 须 Send）：
    // 放进阻塞线程里 block_on，锁卫不离开该线程
    let saves = tauri::async_runtime::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(async { game.read().unwrap().get_saves().await })
    })
    .await
    .map_err(|err| err.to_string())?;

    Ok(saves
        .iter()
        .filter(|item| !item.broken)
        .filter_map(|item| {
            let dir = item.path.file_name()?.to_string_lossy().to_string();
            if dir.is_empty() {
                return None;
            }
            Some(ResourceSaveDto {
                name: item.level_name.clone(),
                dir,
            })
        })
        .collect())
}

/// 下载资源到实例
///
/// 模组 / 资源包 / 光影包直接放入实例对应目录；数据包必须指定存档
/// `world`（saves/ 下的文件夹名），zip 原样放入其 datapacks 目录；
/// 存档先下到下载目录，下载完成后再解压导入（`import_save`）。
///
/// 下载走全局下载器（下载窗口可见进度），命令立即返回；
/// 进度按目标文件路径匹配后经 add-resource-status 事件推给前端，
/// 完成后条目保留一段时间再从任务表移除（窗口顶部进度条「完成」停留）
#[tauri::command]
pub async fn add_resource_download(
    app: AppHandle,
    game: String,
    source: String,
    pid: String,
    fid: String,
    file_type: String,
    world: Option<String>,
) -> Result<(), String> {
    let uuid = Uuid::parse_str(&game);
    if uuid.is_err() {
        return Err(String::from("err.uuid"));
    }
    let uuid = uuid.unwrap();
    let instance = mml_game::get_instance(&uuid);
    if instance.is_none() {
        return Err(String::from("err.gameNotFound"));
    }
    let instance = instance.unwrap();
    let file_type = FileType::from_string(&file_type);
    if file_type.is_none() {
        return Err(String::from("err.fileTypeNotFound"));
    }
    let file_type = file_type.unwrap();
    if !matches!(
        file_type,
        FileType::Mod | FileType::Resourcepack | FileType::Shaderpack | FileType::Save | FileType::DataPacks
    ) {
        return Err(String::from("err.fileTypeNotFound"));
    }
    let source = ModPackType::from_string(&source);
    if !matches!(source, ModPackType::CurseForge | ModPackType::Modrinth) {
        return Err(String::from("err.sourceType"));
    }
    // 存档只有 CurseForge 有（Modrinth 没有 world 类型）
    if file_type == FileType::Save && !matches!(source, ModPackType::CurseForge) {
        return Err(String::from("addResource.saveSourceLimit"));
    }
    let world = world.map(|item| item.trim().to_string()).filter(|item| !item.is_empty());
    // 数据包必须指定目标存档
    if file_type == FileType::DataPacks && world.is_none() {
        return Err(String::from("err.saveNotFound"));
    }

    // 目标目录
    let dir = {
        let game = instance.read().unwrap();
        match file_type {
            FileType::Mod => game.get_mods_path(),
            FileType::Resourcepack => game.get_resourcepacks_path(),
            FileType::Shaderpack => game.get_shaderpacks_path(),
            FileType::DataPacks => game
                .get_saves_path()
                .join(world.as_deref().unwrap())
                .join(names::GAME_DATAPACK_DIR),
            // 存档先下到临时目录，下载完成后再解压导入
            _ => mml_downloader::get_download_path(),
        }
    };

    // 同一实例同一文件不重复下载（先占位，名字与目标路径等文件信息拿到后再补全）
    let key = SourceInfo {
        pid: pid.clone(),
        fid: fid.clone(),
    };
    {
        let mut map = DOWNLOAD_NOW.write().unwrap();
        let entry = map.entry(uuid).or_default();
        if entry.contains_key(&key) {
            return Err(String::from("err.alreadyDownloading"));
        }
        entry.insert(
            key.clone(),
            SourceDownloadInfo {
                file: PathBuf::new(),
                name: String::new(),
                now: 0.0,
                done: false,
                failed: false,
            },
        );
    }
    emit_add_resource_status(&app, build_status(&app));

    // 构建下载项目与已下载标记，失败路径都要移除 DOWNLOAD_NOW 条目
    let res: Result<(FileItemObj, OnlineInfoObj), String> = async {
        match source {
            ModPackType::CurseForge => {
                let fnum: u64 = fid.parse().map_err(|_| String::from("err.fileNotFound"))?;
                let mut list = curseforge_api::get_files(vec![fnum])
                    .await
                    .map_err(|err| err.to_string())?;
                let mut data = list
                    .pop()
                    .ok_or_else(|| String::from("err.fileNotFound"))?;
                let obj = curseforge::make_file_item_obj(&mut data, &dir);
                let info = OnlineInfoObj {
                    path: dir.to_string_lossy().to_string(),
                    name: data.display_name.clone(),
                    file: data.file_name.clone(),
                    sha1: data.sha1_hash(),
                    url: obj.url.clone(),
                    modid: data.mod_id.to_string(),
                    fileid: data.id.to_string(),
                };
                Ok((obj, info))
            }
            ModPackType::Modrinth => {
                let data = modrinth_api::get_version(&pid, &fid)
                    .await
                    .map_err(|err| err.to_string())?;
                let obj = modrinth::make_download_obj(&data, &dir);
                let file = data
                    .files
                    .iter()
                    .find(|item| item.primary)
                    .unwrap_or(data.files.first().unwrap());
                let info = OnlineInfoObj {
                    path: dir.to_string_lossy().to_string(),
                    name: data.name.clone(),
                    file: file.filename.clone(),
                    sha1: file.hashes.sha1.clone(),
                    url: file.url.clone(),
                    modid: pid.clone(),
                    fileid: data.id.clone(),
                };
                Ok((obj, info))
            }
            _ => Err(String::from("err.sourceType")),
        }
    }
    .await;

    let (obj, online_info) = match res {
        Ok(item) => item,
        Err(err) => {
            DOWNLOAD_NOW.write().unwrap().entry(uuid).or_default().remove(&key);
            emit_add_resource_status(&app, build_status(&app));
            return Err(err);
        }
    };

    // 补全条目的显示名与目标路径（进度回调按路径匹配）
    {
        let mut map = DOWNLOAD_NOW.write().unwrap();
        if let Some(info) = map.get_mut(&uuid).and_then(|entry| entry.get_mut(&key)) {
            info.file = obj.file.clone();
            info.name = obj.name.clone();
        }
    }
    emit_add_resource_status(&app, build_status(&app));

    let is_save = file_type == FileType::Save;
    let save_zip = obj.file.clone();
    let pid2 = pid.clone();
    let app2 = app.clone();

    // 后台任务：下载 → 终态标记 → 存档导入 / 已下载标记 → 延时移除
    tauri::async_runtime::spawn(async move {
        let ok = mml_downloader::start_download_task(vec![obj]).await;

        // 终态：进度拉满并打标记广播（完成后条目保留一段时间再移除）
        {
            let mut map = DOWNLOAD_NOW.write().unwrap();
            if let Some(info) = map.get_mut(&uuid).and_then(|entry| entry.get_mut(&key)) {
                info.now = 100.0;
                if ok {
                    info.done = true;
                } else {
                    info.failed = true;
                }
            }
        }
        emit_add_resource_status(&app2, build_status(&app2));

        if ok {
            if is_save {
                let instance = instance.clone();
                if let Ok(res) = tauri::async_runtime::spawn_blocking(move || {
                    instance.read().unwrap().import_save(&save_zip)
                })
                .await
                {
                    if let Err(err) = res {
                        mml_log::error_type(err);
                    }
                }
            }

            let mut game = instance.write().unwrap();
            let mut list = game.read_online_info();
            list.insert(pid2, online_info);
            game.save_online_info(&list);
        }

        // 完成保留 5 秒、失败保留 15 秒（留时间看清错误）再从任务表移除
        tokio::time::sleep(Duration::from_secs(if ok { 5 } else { 15 })).await;
        DOWNLOAD_NOW.write().unwrap().entry(uuid).or_default().remove(&key);
        emit_add_resource_status(&app2, build_status(&app2));
    });

    Ok(())
}
