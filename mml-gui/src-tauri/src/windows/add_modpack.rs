//! 下载整合包窗口：在线整合包的搜索 / 版本列表 / 安装
//!
//! 安装走 `mml_game::add_game`（CurseForge / Modrinth 各自的流程），
//! 进度与重名确认复用添加实例窗口的回调（`super::add`），因为压缩包 / 网址
//! 导入走的是同一套安装流程。
//!
//! 安装是多任务的：任务登记在全局表 [`DOWNLOAD_NOW`]（键 = pid+fid，同键
//! 不允许重复安装），与窗口生命周期解耦——窗口关闭任务照跑，进度改由主窗口
//! 显示；任务进度通过 `add-modpack-status` 事件广播（携带窗口开关状态）。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock, Mutex, RwLock};

use mml_game::add_game;
use mml_game::gui_hook::{AddModPackGui, AddModPackState, IAddModPackGui};
use mml_game::launcher::{FileType, ModPackType};
use mml_net::curseforge_api::file_obj::CurseForgeFileDataObj;
use mml_net::curseforge_api::list_obj::CurseForgeListDataObj;
use mml_net::curseforge_api::{self, CurseFogreArg, CurseForgeSortType};
use mml_net::modrinth_api::search_obj::HitObj;
use mml_net::modrinth_api::version_obj::ModrinthVersionObj;
use mml_net::modrinth_api::{self};
use mml_net::modrinth_api::{ModrinthSearchArg, ModrinthSortType};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow};
use tokio::sync::RwLock as AsyncRwLock;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::collect_utils;
use crate::listens;
use crate::dtos::add_modpack_dto::{ModPackStatusDto, ModPackTaskDto};
use crate::dtos::add_resource_dto::{
    FileListDto, FileListItemDto, ProjectDetailDto, ProjectDto, ProjectItemDto,
};
use crate::windows::add_resource::SourceInfo;

use super::add::{instance_gui, pack_state_id};

/// CurseForge 项目列表缓存（pid → 项目数据，列表页请求时填充）
static CURSEFOGRE_INFO: LazyLock<AsyncRwLock<HashMap<String, CurseForgeListDataObj>>> =
    LazyLock::new(|| AsyncRwLock::new(HashMap::new()));
/// Modrinth 项目列表缓存（pid → 搜索命中项，列表页请求时填充）
static MODRINTH_INFO: LazyLock<AsyncRwLock<HashMap<String, HitObj>>> =
    LazyLock::new(|| AsyncRwLock::new(HashMap::new()));

/// CurseForge 文件缓存（pid → fid → 文件数据，文件列表页请求时填充）
static CURSEFOGRE_FILE: LazyLock<
    AsyncRwLock<HashMap<String, HashMap<String, CurseForgeFileDataObj>>>,
> = LazyLock::new(|| AsyncRwLock::new(HashMap::new()));
/// Modrinth 版本缓存（pid → 版本号 → 版本数据，文件列表页请求时填充）
static MODRINTH_FILE: LazyLock<AsyncRwLock<HashMap<String, HashMap<String, ModrinthVersionObj>>>> =
    LazyLock::new(|| AsyncRwLock::new(HashMap::new()));

/// 进行中的整合包安装任务（键 = pid+fid，同键不重复安装）
static DOWNLOAD_NOW: LazyLock<RwLock<HashMap<SourceInfo, Arc<ModPackTask>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 任务主/子进度快照（各回调分别更新字段）
pub struct ModPackProgress {
    /// 安装阶段 ID（见 [`pack_state_id`])
    state: String,
    /// 主进度：已完成数
    now: u32,
    /// 主进度：总数
    total: u32,
    /// 子任务说明文字
    sub_text: Option<String>,
    /// 子任务进度：已完成数
    sub_now: u32,
    /// 子任务进度：总数
    sub_total: u32,
}

/// 一个整合包安装任务：进度 + 取消令牌 + 终态标记
///
/// 终态（done / failed / cancelled）的任务保留在表里供前端展示，
/// 由 `add_modpack_clear_done` 清除。
pub struct ModPackTask {
    /// 任务 ID（安装开始时生成，与最终实例 uuid 无关）
    uuid: Uuid,
    source: String,
    pid: String,
    fid: String,
    /// 显示名（安装开始时从列表缓存取）
    name: String,
    /// 取消令牌
    cancel: CancellationToken,
    /// 主/子进度快照
    progress: Mutex<ModPackProgress>,
    /// 安装完成
    done: AtomicBool,
    /// 安装失败
    failed: AtomicBool,
    /// 已取消
    cancelled: AtomicBool,
    /// 失败错误文案（failed 为 true 时有值）
    error: Mutex<Option<String>>,
    /// 安装出的实例 uuid（成功后回填）
    instance_uuid: Mutex<Option<String>>,
}

impl ModPackTask {
    /// 转前端 DTO（快照当前进度与终态）
    fn dto(&self) -> ModPackTaskDto {
        let progress = self.progress.lock().unwrap();
        ModPackTaskDto {
            uuid: self.uuid.to_string(),
            source: self.source.clone(),
            pid: self.pid.clone(),
            fid: self.fid.clone(),
            name: self.name.clone(),
            state: progress.state.clone(),
            now: progress.now,
            total: progress.total,
            sub_text: progress.sub_text.clone(),
            sub_now: progress.sub_now,
            sub_total: progress.sub_total,
            done: self.done.load(Ordering::Acquire),
            failed: self.failed.load(Ordering::Acquire),
            cancelled: self.cancelled.load(Ordering::Acquire),
            error: self.error.lock().unwrap().clone(),
            instance_uuid: self.instance_uuid.lock().unwrap().clone(),
        }
    }
}

/// 整合包安装任务总览快照（事件负载 / 查询返回）
fn build_status(app: &AppHandle) -> ModPackStatusDto {
    let map = DOWNLOAD_NOW.read().unwrap();
    ModPackStatusDto {
        window_open: app.get_webview_window("mml-add_modpack").is_some(),
        tasks: map.values().map(|task| task.dto()).collect(),
    }
}

/// 整合包安装任务总览事件（任务增删 / 进度变化 / 终态时广播）
#[gui_macros::emit]
pub fn emit_add_modpack_status(app: &AppHandle, dto: ModPackStatusDto) {
    let _ = app.emit(listens::ADD_MODPACK_STATUS, dto);
}

/// 整合包安装进度回调：把安装状态 / 进度写进任务并广播总览事件
struct TaskPackGui {
    task: Arc<ModPackTask>,
    app: AppHandle,
}

impl TaskPackGui {
    /// 更新任务进度并广播总览事件
    fn update(&self, f: impl FnOnce(&mut ModPackProgress)) {
        {
            let mut progress = self.task.progress.lock().unwrap();
            f(&mut progress);
        }
        emit_add_modpack_status(&self.app, build_status(&self.app));
    }
}

impl IAddModPackGui for TaskPackGui {
    /// 设置安装阶段
    fn set_state(&self, state: AddModPackState) {
        self.update(|progress| progress.state = pack_state_id(state).into());
    }

    /// 设置主进度（value / all）
    fn set_now(&self, value: usize, all: Option<usize>) {
        self.update(|progress| {
            progress.now = value as u32;
            progress.total = all.unwrap_or(0) as u32;
        });
    }

    /// 设置子任务说明文字
    fn set_sub_text(&self, text: Option<String>) {
        self.update(|progress| progress.sub_text = text);
    }

    /// 设置子任务进度（value / all）
    fn set_sub_now(&self, value: usize, all: Option<usize>) {
        self.update(|progress| {
            progress.sub_now = value as u32;
            progress.sub_total = all.unwrap_or(0) as u32;
        });
    }
}

/// 各窗口正在进行的列表搜索（键 = 窗口 label），窗口关闭时取消
static SEARCH_CANCEL: LazyLock<Mutex<HashMap<String, CancellationToken>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// 登记本窗口这一发搜索并返回取消令牌，顺手取消上一发
fn register_search(label: &str) -> CancellationToken {
    let mut lock = SEARCH_CANCEL.lock().unwrap();

    if let Some(old) = lock.get(label) {
        old.cancel();
    }

    let token = CancellationToken::new();
    lock.insert(label.to_string(), token.clone());

    token
}

/// 取消该窗口正在进行的列表搜索（窗口关闭时调用）
pub fn cancel_search(label: &str) {
    if let Some(token) = SEARCH_CANCEL.lock().unwrap().remove(label) {
        token.cancel();
    }
}

/// 安装在线整合包（多任务：命令立即返回，任务在后台跑，
/// 进度走 `add-modpack-status` 事件；name 取自整合包元数据）
#[tauri::command]
pub async fn add_modpack_install(
    window: WebviewWindow,
    app: AppHandle,
    source: String,
    project_id: String,
    file_id: String,
    group: Option<String>,
) -> Result<(), String> {
    let source_type = ModPackType::from_string(&source);
    if !matches!(source_type, ModPackType::CurseForge | ModPackType::Modrinth) {
        return Err(String::from("err.sourceType"));
    }

    let key = SourceInfo {
        pid: project_id.clone(),
        fid: file_id.clone(),
    };

    // 显示名从列表缓存取（列表页已缓存过项目元数据）
    let name = match source_type {
        ModPackType::CurseForge => {
            CURSEFOGRE_INFO
                .read()
                .await
                .get(&project_id)
                .map(|item| item.name.clone())
        }
        ModPackType::Modrinth => {
            MODRINTH_INFO
                .read()
                .await
                .get(&project_id)
                .map(|item| item.title.clone())
        }
        _ => None,
    }
    .unwrap_or_else(|| project_id.clone());

    // 登记任务：同 pid+fid 不允许重复安装
    let token = CancellationToken::new();
    let task = Arc::new(ModPackTask {
        uuid: Uuid::new_v4(),
        source: source.clone(),
        pid: project_id.clone(),
        fid: file_id.clone(),
        name,
        cancel: token.clone(),
        progress: Mutex::new(ModPackProgress {
            state: pack_state_id(AddModPackState::DownloadPack).into(),
            now: 0,
            total: 0,
            sub_text: None,
            sub_now: 0,
            sub_total: 0,
        }),
        done: AtomicBool::new(false),
        failed: AtomicBool::new(false),
        cancelled: AtomicBool::new(false),
        error: Mutex::new(None),
        instance_uuid: Mutex::new(None),
    });
    {
        let mut map = DOWNLOAD_NOW.write().unwrap();
        if map.contains_key(&key) {
            return Err(String::from("err.alreadyDownloading"));
        }
        map.insert(key, task.clone());
    }
    emit_add_modpack_status(&app, build_status(&app));

    let gui = instance_gui(&window);
    let pack_gui: AddModPackGui = Some(Arc::new(TaskPackGui {
        task: task.clone(),
        app: app.clone(),
    }));

    // 后台安装：命令不等安装结束（安装 future 非 Send，放阻塞线程上 block_on）
    let app_task = app.clone();
    tauri::async_runtime::spawn(async move {
        let res = tauri::async_runtime::spawn_blocking(move || {
            tauri::async_runtime::block_on(async {
                match source.as_str() {
                    "curseforge" => {
                        let fid: u64 = file_id
                            .parse()
                            .map_err(|_| "err.badFileId".to_string())?;
                        let list = curseforge_api::get_files(vec![fid])
                            .await
                            .map_err(|e| e.to_string())?;
                        let mut data = list
                            .into_iter()
                            .next()
                            .ok_or_else(|| "err.fileNotFound".to_string())?;
                        add_game::install_curseforge(
                            &mut data, group, None, gui, pack_gui, None, token,
                        )
                        .await
                        .map_err(|e| e.to_string())
                    }
                    "modrinth" => {
                        let data = modrinth_api::get_version(&project_id, &file_id)
                            .await
                            .map_err(|e| e.to_string())?;
                        add_game::install_modrinth(&data, group, None, gui, pack_gui, None, token)
                            .await
                            .map_err(|e| e.to_string())
                    }
                    _ => Err(String::from("err.unknownSource")),
                }
            })
        })
        .await
        .unwrap_or_else(|e| Err(e.to_string()));

        // 记终态并广播（cancelled 标记已由取消命令置位，优先于 failed）
        match res {
            Ok(uuid) => {
                *task.instance_uuid.lock().unwrap() = Some(uuid.to_string());
                task.done.store(true, Ordering::Release);
                crate::windows::main::emit_instance_change(&app_task, "add");
            }
            Err(e) => {
                *task.error.lock().unwrap() = Some(e);
                task.failed.store(true, Ordering::Release);
            }
        }
        emit_add_modpack_status(&app_task, build_status(&app_task));
    });

    Ok(())
}

/// 取一个进行中的整合包安装任务（pid + fid 定位）
#[tauri::command]
pub async fn add_modpack_cancel(app: AppHandle, pid: String, fid: String) -> Result<(), String> {
    let key = SourceInfo { pid, fid };
    let task = DOWNLOAD_NOW.read().unwrap().get(&key).cloned();
    let Some(task) = task else {
        return Err(String::from("err.taskNotFound"));
    };
    task.cancelled.store(true, Ordering::Release);
    task.cancel.cancel();
    emit_add_modpack_status(&app, build_status(&app));
    Ok(())
}

/// 查询整合包安装任务总览（窗口挂载时同步一次，之后靠事件）
#[tauri::command]
pub async fn add_modpack_status(app: AppHandle) -> Result<ModPackStatusDto, String> {
    Ok(build_status(&app))
}

/// 清除已结束（完成 / 失败 / 取消）的安装任务
#[tauri::command]
pub async fn add_modpack_clear_done(app: AppHandle) -> Result<(), String> {
    DOWNLOAD_NOW.write().unwrap().retain(|_, task| {
        !(task.done.load(Ordering::Acquire)
            || task.failed.load(Ordering::Acquire)
            || task.cancelled.load(Ordering::Acquire))
    });
    emit_add_modpack_status(&app, build_status(&app));
    Ok(())
}

/// 获取项目列表
#[tauri::command]
pub async fn add_modpack_list(
    window: WebviewWindow,
    source: String,
    page: u32,
    sort: String,
    category: Option<String>,
    filter: Option<String>,
    version: Option<String>,
) -> Result<ProjectDto, String> {
    // 同一窗口同时只跑一发：新的一发顶掉上一发，窗口关闭时由 `cancel_search` 全部取消
    let token = register_search(window.label());

    tokio::select! {
        res = list_projects(source, page, sort, category, filter, version) => res,
        _ = token.cancelled() => Err(String::from("err.cancelled")),
    }
}

/// 按下载源拉取项目列表
async fn list_projects(
    source: String,
    page: u32,
    sort: String,
    category: Option<String>,
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
                category,
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
                query: filter,
                category,
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

/// 获取项目详情（双击列表项）
///
/// Modrinth：project 拿正文（markdown）/ 截图，team 拿作者头像；
/// CurseForge：只有简介 + 截图，优先用列表缓存，缓存未命中再查 mod_info
#[tauri::command]
pub async fn add_modpack_detail(source: String, pid: String) -> Result<ProjectDetailDto, String> {
    let source = ModPackType::from_string(&source);

    match source {
        ModPackType::CurseForge => {
            // 列表时已缓存了 mod 数据，直接借用构造；未命中才发请求
            let detail = {
                let map = CURSEFOGRE_INFO.read().await;
                map.get(&pid).map(ProjectDetailDto::new_curseforge)
            };
            match detail {
                Some(detail) => Ok(detail),
                None => {
                    let data = curseforge_api::get_mod_info(&pid)
                        .await
                        .map_err(|err| err.to_string())?;
                    Ok(ProjectDetailDto::new_curseforge(&data.data))
                }
            }
        }
        ModPackType::Modrinth => {
            let data = modrinth_api::get_project(&pid)
                .await
                .map_err(|err| err.to_string())?;

            ProjectDetailDto::new_modrinth(&data)
                .await
                .map_err(|err| err.to_string())
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

/// 项目是否已安装过（实例表里有同 pid 的整合包实例）
fn check_modpack_download(pid: &str) -> bool {
    mml_game::get_instances().iter().any(|item| {
        let temp = item.read().unwrap();
        temp.is_modpack
            && match temp.pid.as_ref() {
                Some(data) => data == pid,
                None => false,
            }
    })
}

/// 具体版本是否已安装过（实例表里有同 pid + fid 的整合包实例）
fn check_modpack_version_download(pid: &str, fid: &str) -> bool {
    mml_game::get_instances().iter().any(|item| {
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

/// 项目是否正在安装（列表角标：该项目任一版本在装）
async fn check_download_now(pid: &str) -> bool {
    DOWNLOAD_NOW
        .read()
        .unwrap()
        .keys()
        .any(|key| key.pid == pid)
}

/// 具体版本是否正在安装
async fn check_version_download_now(pid: &str, fid: &str) -> bool {
    DOWNLOAD_NOW.read().unwrap().contains_key(&SourceInfo {
        pid: pid.to_string(),
        fid: fid.to_string(),
    })
}
