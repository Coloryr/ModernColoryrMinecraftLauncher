//! 添加实例窗口：模型 + 创建操作 + 窗口按钮调用的方法
//!
//! 实例创建走 `mcml_game::add_game`（新建 / 文件夹导入 / 压缩包 / 在线网址），
//! 创建成功后发 instance-change 事件通知主窗口刷新列表。
//! 模型只保存跟随窗口生命周期的运行态：当前安装任务的取消令牌。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use mcml_game::add_game::{self, PackType};
use mcml_game::gui_hook::{
    AddInstanceGui, AddModPackGui, AddModPackState, IAddInstanceGui, IAddModPackGui, IProgressGui,
    ProgressGui,
};
use mcml_game::launcher::instance_setting_obj::InstanceSettingObj;
use mcml_game::loader::LoaderType;
use mcml_game::GameInstance;
use mcml_net::{curseforge_api, modrinth_api};
use tauri::{AppHandle, Emitter, WebviewWindow};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dtos::{
    DetectedPackDto, DirEntry, LoaderProgressDto, ModpackFileDto, ModpackItemDto, ModpackSearchDto,
    NameConflictDto, PackProgressDto,
};
use crate::listens;

/// 添加实例窗口模型：只存运行态，不落盘
pub struct AddWindowModel {
    /// 进行中安装任务的取消令牌（None = 当前无任务）
    cancel: Mutex<Option<CancellationToken>>,
    /// 关闭保护（查询数据期间为 true，`CloseRequested` 阶段拒绝关闭）
    close_guard: AtomicBool,
    /// 重名确认对话框的应答通道（id → 发送端，前端答复后唤醒创建流程）
    dialog: Mutex<HashMap<u32, oneshot::Sender<bool>>>,
}

impl AddWindowModel {
    pub fn new() -> Self {
        Self {
            cancel: Mutex::new(None),
            close_guard: AtomicBool::new(false),
            dialog: Mutex::new(HashMap::new()),
        }
    }

    fn set_cancel(&self, token: CancellationToken) {
        *self.cancel.lock().unwrap() = Some(token);
    }

    fn take_cancel(&self) -> Option<CancellationToken> {
        self.cancel.lock().unwrap().take()
    }

    pub fn set_close_guard(&self, enabled: bool) {
        self.close_guard.store(enabled, Ordering::Release);
    }

    pub fn close_guard(&self) -> bool {
        self.close_guard.load(Ordering::Acquire)
    }

    fn set_dialog(&self, id: u32, tx: oneshot::Sender<bool>) {
        self.dialog.lock().unwrap().insert(id, tx);
    }

    fn take_dialog(&self, id: u32) -> Option<oneshot::Sender<bool>> {
        self.dialog.lock().unwrap().remove(&id)
    }
}

/// 取添加实例窗口模型（模型跟随窗口生命周期，见 window_manager）
fn model(window: &WebviewWindow) -> Result<Arc<Mutex<AddWindowModel>>, String> {
    crate::window_manager::window_model(window)
        .ok_or_else(|| "添加实例窗口模型未初始化".to_string())
}

/// 前端加载器 ID -> LoaderType（ID 列表见 [`add_get_loaders`]）
fn parse_loader(id: &str) -> Result<LoaderType, String> {
    LoaderType::from_id(id).ok_or_else(|| format!("未知的加载器类型: {id}"))
}

/// 前端压缩包 ID -> PackType（ID 列表见 [`add_get_pack_types`]）
fn parse_pack_type(id: &str) -> Result<PackType, String> {
    PackType::from_id(id).ok_or_else(|| format!("未知的压缩包类型: {id}"))
}

/// 重名确认对话框事件 id 自增
static DIALOG_ID: AtomicU32 = AtomicU32::new(1);

/// 实例重名确认对话框事件（kind：overwrite 覆盖 / rename 自动改名）
#[gui_macros::emit]
pub fn emit_add_name_conflict(window: &WebviewWindow, dto: NameConflictDto) {
    let _ = window.emit_to(window.label(), listens::ADD_NAME_CONFLICT, dto);
}

/// 实例创建重名确认：向添加实例窗口发事件弹窗，等待用户在对话框上答复
struct AddInstanceDialogGui {
    window: WebviewWindow,
}

impl AddInstanceDialogGui {
    /// 弹出确认框并等待答复；窗口已关闭时视为拒绝
    async fn ask(&self, kind: &str, name: &str) -> bool {
        let Ok(store) = model(&self.window) else {
            return false;
        };
        let id = DIALOG_ID.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        store.lock().unwrap().set_dialog(id, tx);
        emit_add_name_conflict(
            &self.window,
            NameConflictDto {
                id,
                kind: kind.to_string(),
                name: name.to_string(),
            },
        );
        rx.await.unwrap_or(false)
    }
}

#[async_trait]
impl IAddInstanceGui for AddInstanceDialogGui {
    /// 是否同意覆盖重名实例
    async fn overwrite(&self, obj: GameInstance) -> bool {
        let name = obj.read().unwrap().name.clone();
        self.ask("overwrite", &name).await
    }

    /// 是否同意自动修改名字
    async fn name_replace(&self, name: &str) -> bool {
        self.ask("rename", name).await
    }
}

/// 构建重名确认回调（跟随添加实例窗口）
fn instance_gui(window: &WebviewWindow) -> AddInstanceGui {
    Some(Arc::new(AddInstanceDialogGui {
        window: window.clone(),
    }) as Arc<dyn IAddInstanceGui>)
}

/// 整合包安装进度事件（跟随添加实例窗口）
#[gui_macros::emit]
pub fn emit_add_pack_progress(window: &WebviewWindow, dto: PackProgressDto) {
    let _ = window.emit_to(window.label(), listens::ADD_PACK_PROGRESS, dto);
}

/// 整合包安装进度回调：把安装状态 / 进度转发为前端事件（弹窗显示）
struct PackProgressGui {
    window: WebviewWindow,
    /// 当前进度快照（各回调分别更新字段后整体发出）
    dto: Mutex<PackProgressDto>,
}

impl PackProgressGui {
    fn update(&self, f: impl FnOnce(&mut PackProgressDto)) {
        let mut dto = self.dto.lock().unwrap();
        f(&mut dto);
        emit_add_pack_progress(&self.window, dto.clone());
    }
}

impl IAddModPackGui for PackProgressGui {
    fn set_state(&self, state: AddModPackState) {
        self.update(|dto| dto.state = pack_state_id(state).into());
    }

    fn set_now(&self, value: usize, all: Option<usize>) {
        self.update(|dto| {
            dto.now = value as u32;
            dto.total = all.unwrap_or(0) as u32;
        });
    }

    fn set_sub_text(&self, text: Option<String>) {
        self.update(|dto| dto.sub_text = text);
    }

    fn set_sub_now(&self, value: usize, all: Option<usize>) {
        self.update(|dto| {
            dto.sub_now = value as u32;
            dto.sub_total = all.unwrap_or(0) as u32;
        });
    }
}

/// 安装阶段 ID（与前端 i18n 键对应）
fn pack_state_id(state: AddModPackState) -> &'static str {
    match state {
        AddModPackState::DownloadPack => "downloadPack",
        AddModPackState::ReadInfo => "readInfo",
        AddModPackState::GetInfo => "getInfo",
        AddModPackState::DownloadFile => "downloadFile",
        AddModPackState::Extract => "extract",
        AddModPackState::Done => "done",
    }
}

/// 构建安装进度回调（跟随添加实例窗口）
fn pack_gui(window: &WebviewWindow) -> AddModPackGui {
    Some(Arc::new(PackProgressGui {
        window: window.clone(),
        dto: Mutex::new(PackProgressDto {
            state: "downloadPack".into(),
            now: 0,
            total: 0,
            sub_text: None,
            sub_now: 0,
            sub_total: 0,
        }),
    }) as Arc<dyn IAddModPackGui>)
}

/// 用户对重名确认对话框的答复（唤醒等待中的创建流程）
#[tauri::command]
pub fn add_answer_name_conflict(window: WebviewWindow, id: u32, answer: bool) {
    if let Ok(store) = model(&window) {
        if let Some(tx) = store.lock().unwrap().take_dialog(id) {
            let _ = tx.send(answer);
        }
    }
}

/// 任务前置检查：核心加载完成（HTTP 客户端就绪）+ 模型存在，返回模型与取消令牌
fn prepare(
    window: &WebviewWindow,
) -> Result<(Arc<Mutex<AddWindowModel>>, CancellationToken), String> {
    if !mcml_net::is_init() {
        return Err("核心尚未加载完成，请稍后再试".to_string());
    }
    let store = model(window)?;
    let token = CancellationToken::new();
    store.lock().unwrap().set_cancel(token.clone());
    Ok((store, token))
}

/// 列出目录的直接内容（目录优先，再按名称排序）；
/// 添加实例窗口选择文件夹时调用，用于预览文件夹内容树
#[tauri::command]
pub fn add_list_dir(path: String) -> Result<Vec<DirEntry>, String> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name().to_string_lossy().to_string();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
        entries.push(DirEntry { name, is_dir });
    }
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    Ok(entries)
}

/// 列出压缩包内的条目路径（目录以 `/` 结尾，添加实例窗口预览内容树用）
#[tauri::command]
pub fn add_list_archive(path: String) -> Result<Vec<String>, String> {
    let archive = mcml_base::archives::BaseArchive::open(&path).map_err(|e| e.to_string())?;
    Ok(archive.entries().iter().map(|e| e.name.clone()).collect())
}

/// 检测压缩包的整合包类型与推荐实例名（选择压缩包后自动填表用）
#[tauri::command]
pub fn add_detect_archive(path: String) -> Result<DetectedPackDto, String> {
    let pack = mcml_game::add_game::detect_pack(&path).map_err(|e| e.to_string())?;
    Ok(DetectedPackDto {
        pack_type: pack.pack_type.id().to_string(),
        name: pack.name,
    })
}

/// 从头新建实例（版本 + 加载器）
#[tauri::command]
pub async fn add_create_new(
    window: WebviewWindow,
    app: AppHandle,
    name: String,
    version: String,
    loader: String,
    loader_version: Option<String>,
    group: Option<String>,
) -> Result<String, String> {
    let (_store, _token) = prepare(&window)?;
    let loader = parse_loader(&loader)?;
    let obj = InstanceSettingObj {
        uuid: Uuid::new_v4(),
        name,
        version,
        group,
        loader,
        loader_version,
        ..Default::default()
    };
    // mcml-game 的安装 future 非 Send，放阻塞线程上 block_on 执行
    let gui = instance_gui(&window);
    let res = tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(obj.create_instance(gui))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let uuid = { res.read().unwrap().uuid.to_string() };
    crate::windows::main::emit_instance_change(&app, "add");
    Ok(uuid)
}

/// 导入文件夹为实例
#[tauri::command]
pub async fn add_import_folder(
    window: WebviewWindow,
    app: AppHandle,
    path: String,
    name: Option<String>,
    group: Option<String>,
) -> Result<String, String> {
    let (_store, token) = prepare(&window)?;
    // 安装 future 非 Send：阻塞线程 block_on；读锁在块内释放后再发事件
    let gui = instance_gui(&window);
    let res = tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(async {
            add_game::add_game_folder(&path, name, group, None, gui, None, token).await
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    let uuid = { res.read().unwrap().uuid.to_string() };
    crate::windows::main::emit_instance_change(&app, "add");
    Ok(uuid)
}

/// 导入整合包压缩包为实例（packType：CurseForge / Modrinth / McMod / 本地；
/// unselect：按压缩包内完整条目名排除的文件，来自前端文件树未勾选项）
#[tauri::command]
pub async fn add_import_archive(
    window: WebviewWindow,
    app: AppHandle,
    path: String,
    pack_type: String,
    name: Option<String>,
    group: Option<String>,
    unselect: Option<Vec<String>>,
) -> Result<String, String> {
    let (_store, token) = prepare(&window)?;
    let pack = parse_pack_type(&pack_type)?;
    let gui = instance_gui(&window);
    let progress = pack_gui(&window);
    let uuid = tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(async {
            add_game::install_archive_from_file(
                &path, name, group, unselect, gui, progress, None, pack, token,
            )
            .await
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    crate::windows::main::emit_instance_change(&app, "add");
    Ok(uuid.to_string())
}

/// 从网址安装实例（默认按直接解压处理）
#[tauri::command]
pub async fn add_import_url(
    window: WebviewWindow,
    app: AppHandle,
    url: String,
    name: Option<String>,
    group: Option<String>,
) -> Result<String, String> {
    let (_store, token) = prepare(&window)?;
    let gui = instance_gui(&window);
    let progress = pack_gui(&window);
    let uuid = tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(async {
            add_game::install_archive_from_url(
                &url, name, group, None, gui, progress, None, PackType::ArchivePack, token,
            )
            .await
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;
    crate::windows::main::emit_instance_change(&app, "add");
    Ok(uuid.to_string())
}

/// 获取加载器的可用版本列表（添加实例窗口的加载器版本下拉）
///
/// - `loader`: 加载器独立 ID（见 [`add_get_loaders`]）
/// - `mc`: 游戏版本号（Forge 的 BMCLAPI 源、OptiFine / LiteLoader 需要按版本过滤）
#[tauri::command]
pub async fn add_get_loader_versions(loader: String, mc: String) -> Result<Vec<String>, String> {
    if !mcml_net::is_init() {
        return Ok(Vec::new());
    }
    let loader = parse_loader(&loader)?;
    mcml_game::loader::loader_versions::get_loader_versions(&loader, &mc)
        .await
        .map_err(|e| e.to_string())
}

/// 获取加载器 ID 列表（添加实例窗口的加载器下拉）
#[tauri::command]
pub fn add_get_loaders() -> Vec<&'static str> {
    LoaderType::ids()
}

/// 加载器支持列表查询进度事件（前端弹窗显示进度条）
#[gui_macros::emit]
pub fn emit_add_loader_progress(app: &AppHandle, dto: LoaderProgressDto) {
    let _ = app.emit(listens::ADD_LOADER_PROGRESS, dto);
}

/// 加载器支持列表查询进度回调：把步数发到前端弹窗（进度条）
struct SupportLoadersProgressGui {
    app: AppHandle,
}

impl IProgressGui for SupportLoadersProgressGui {
    fn set_progress_text(&self, _text: Option<String>) {}

    fn set_progress_now(&self, value: usize, all: Option<usize>) {
        emit_add_loader_progress(
            &self.app,
            LoaderProgressDto {
                step: value as u32,
                total: all.unwrap_or(0) as u32,
            },
        );
    }
}

/// 查询指定游戏版本支持的加载器 ID 列表（选中版本后调用）
///
/// 全局查询：结果按版本号缓存（主窗口 / 添加实例窗口共用），
/// 同版本并发查询单飞共享，每查完一个加载器发一次进度事件（前端显示进度条）。
#[tauri::command]
pub async fn add_get_support_loaders(app: AppHandle, mc: String) -> Result<Vec<String>, String> {
    use std::sync::LazyLock;
    use tokio::sync::{Mutex, OnceCell};

    /// 支持列表查询结果缓存（版本号 -> 查询单飞单元）
    static SUPPORT_LOADERS_CACHE: LazyLock<Mutex<HashMap<String, Arc<OnceCell<Vec<String>>>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    if !mcml_net::is_init() {
        return Ok(Vec::new());
    }
    // 取/建该版本的查询单元：并发请求共享同一次查询
    let cell = {
        let mut cache = SUPPORT_LOADERS_CACHE.lock().await;
        cache.entry(mc.clone()).or_default().clone()
    };
    let gui_app = app.clone();
    let mc_clone = mc.clone();
    let result = cell
        .get_or_try_init(|| async move {
            let gui: ProgressGui = Some(Arc::new(SupportLoadersProgressGui { app: gui_app }) as Arc<dyn IProgressGui>);
            mcml_game::loader::loader_versions::get_support_loaders(&mc_clone, gui).await
        })
        .await;
    // 失败不缓存：单元留空，下次查询重试
    match result {
        Ok(list) => Ok(list.clone()),
        Err(e) => Err(e.to_string()),
    }
}

/// 获取压缩包类型 ID 列表（添加实例窗口的整合包类型下拉）
#[tauri::command]
pub fn add_get_pack_types() -> Vec<&'static str> {
    PackType::ids()
}

/// 获取游戏版本类型列表（添加实例窗口的版本类型下拉，release / snapshot / old_beta / old_alpha）
#[tauri::command]
pub fn add_get_version_types() -> Vec<&'static str> {
    mcml_game::get_version_types()
}

/// 取消进行中的安装任务
#[tauri::command]
pub fn add_cancel(window: WebviewWindow) -> bool {
    let Ok(store) = model(&window) else {
        return false;
    };
    match store.lock().unwrap().take_cancel() {
        Some(token) => {
            token.cancel();
            true
        }
        None => false,
    }
}

/// 设置本窗口的关闭保护（查询数据期间开启，`CloseRequested` 阶段拒绝关闭）
#[tauri::command]
pub fn add_set_close_guard(window: WebviewWindow, enabled: bool) {
    if let Ok(store) = model(&window) {
        store.lock().unwrap().set_close_guard(enabled);
    }
}

/// 前端排序 ID -> CurseForge 排序方式（未知 ID 按流行度）
fn cf_sort(id: &str) -> curseforge_api::CurseForgeSortType {
    match id {
        "featured" => curseforge_api::CurseForgeSortType::Featured,
        "downloads" => curseforge_api::CurseForgeSortType::TotalDownloads,
        "updated" => curseforge_api::CurseForgeSortType::LastUpdated,
        "name" => curseforge_api::CurseForgeSortType::Name,
        _ => curseforge_api::CurseForgeSortType::Popularity,
    }
}

/// 前端排序 ID -> Modrinth 排序方式（CurseForge 独有的排序回退到推荐）
fn mr_sort(id: &str) -> modrinth_api::ModrinthSortType {
    match id {
        "downloads" => modrinth_api::ModrinthSortType::Downloads,
        "updated" => modrinth_api::ModrinthSortType::Updated,
        _ => modrinth_api::ModrinthSortType::Relevance,
    }
}

/// 搜索在线整合包（source：curseforge / modrinth；page 从 0 开始，一页 20 条）
#[tauri::command]
pub async fn add_search_modpacks(
    source: String,
    query: Option<String>,
    version: Option<String>,
    sort: Option<String>,
    page: u32,
) -> Result<ModpackSearchDto, String> {
    if !mcml_net::is_init() {
        return Err("核心尚未加载完成，请稍后再试".to_string());
    }
    const PAGE_SIZE: u32 = 20;

    match source.as_str() {
        "curseforge" => {
            let arg = curseforge_api::CurseFogreArg {
                version,
                page: Some(page),
                sort: cf_sort(sort.as_deref().unwrap_or("popularity")),
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
                sort: mr_sort(sort.as_deref().unwrap_or("popularity")),
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
        _ => Err(format!("未知的整合包来源: {source}")),
    }
}

/// 获取整合包的可安装版本列表（按游戏版本过滤，version 传 None 取全部）
#[tauri::command]
pub async fn add_get_modpack_files(
    source: String,
    project_id: String,
    version: Option<String>,
) -> Result<Vec<ModpackFileDto>, String> {
    if !mcml_net::is_init() {
        return Err("核心尚未加载完成，请稍后再试".to_string());
    }

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
        _ => Err(format!("未知的整合包来源: {source}")),
    }
}

/// 安装在线整合包（下载压缩包后走对应类型的安装流程；name 取自整合包元数据）
#[tauri::command]
pub async fn add_install_modpack(
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
                    let fid: u64 = file_id
                        .parse()
                        .map_err(|_| format!("无效的文件编号: {file_id}"))?;
                    let mut list = curseforge_api::get_files(vec![fid])
                        .await
                        .map_err(|e| e.to_string())?;
                    let mut data = list
                        .into_iter()
                        .next()
                        .ok_or_else(|| "未找到该版本的文件".to_string())?;
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
                _ => Err(format!("未知的整合包来源: {source}")),
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

