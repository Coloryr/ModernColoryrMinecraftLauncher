//! 添加实例窗口：模型 + 创建操作 + 窗口按钮调用的方法
//!
//! 实例创建走 `mml_game::add_game`（新建 / 文件夹导入 / 压缩包 / 在线网址），
//! 创建成功后发 instance-change 事件通知主窗口刷新列表。
//! 模型只保存跟随窗口生命周期的运行态：当前安装任务的取消令牌。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use mml_game::GameInstance;
use mml_game::add_game::{self, PackType};
use mml_game::gui_hook::{
    AddInstanceGui, AddModPackGui, AddModPackState, IAddInstanceGui, IAddModPackGui, IProgressGui,
    ProgressGui,
};
use mml_game::launcher::instance_setting_obj::InstanceSettingObj;
use mml_game::loader::LoaderType;
use tauri::{AppHandle, Emitter, WebviewWindow};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dtos::{
    DetectedPackDto, DirEntry, FolderInstanceDto, LoaderProgressDto, NameConflictDto,
    PackProgressDto,
};
use crate::windows::add_modpack::{LOCAL_PACK_PID, finish_task, register_task};
use crate::windows::add_resource::SourceInfo;
use crate::{listens, windows};

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

    /// 记录当前安装任务的取消令牌
    fn set_cancel(&self, token: CancellationToken) {
        *self.cancel.lock().unwrap() = Some(token);
    }

    /// 取走当前取消令牌（取后为 None）
    fn take_cancel(&self) -> Option<CancellationToken> {
        self.cancel.lock().unwrap().take()
    }

    /// 开关关闭保护
    pub fn set_close_guard(&self, enabled: bool) {
        self.close_guard.store(enabled, Ordering::Release);
    }

    /// 查询关闭保护
    pub fn close_guard(&self) -> bool {
        self.close_guard.load(Ordering::Acquire)
    }

    /// 登记重名确认应答通道
    fn set_dialog(&self, id: u32, tx: oneshot::Sender<bool>) {
        self.dialog.lock().unwrap().insert(id, tx);
    }

    /// 取走重名确认应答通道（取后移除）
    fn take_dialog(&self, id: u32) -> Option<oneshot::Sender<bool>> {
        self.dialog.lock().unwrap().remove(&id)
    }
}

/// 取添加实例窗口模型
///
/// 按 kind 取而不是按调用方窗口：单窗口模式下"添加实例"只是主窗口里的一页，
/// 命令的调用方窗口是主窗口，按窗口取会拿到主窗口模型（表现为 `err.modelMissing`）。
/// 模型由页面挂载时的 `window_ensure_model` 建立、切走时 `window_drop_model` 释放。
fn model(_window: &WebviewWindow) -> Result<Arc<Mutex<AddWindowModel>>, String> {
    windows::model_for_kind("add").ok_or_else(|| "err.modelMissing".to_string())
}

/// 前端加载器 ID -> LoaderType（ID 列表见 [`add_get_loaders`]）
fn parse_loader(id: &str) -> Result<LoaderType, String> {
    LoaderType::from_string(id).ok_or_else(|| "err.unknownLoader".to_string())
}

/// 前端压缩包 ID -> PackType（ID 列表见 [`add_get_pack_types`]）
fn parse_pack_type(id: &str) -> Result<PackType, String> {
    PackType::from_id(id).ok_or_else(|| "err.unknownPackType".to_string())
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

/// 构建重名确认回调（跟随调用方窗口）
pub(crate) fn instance_gui(window: &WebviewWindow) -> AddInstanceGui {
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
    /// 更新进度快照并发事件
    fn update(&self, f: impl FnOnce(&mut PackProgressDto)) {
        let mut dto = self.dto.lock().unwrap();
        f(&mut dto);
        emit_add_pack_progress(&self.window, dto.clone());
    }
}

impl IAddModPackGui for PackProgressGui {
    /// 设置安装阶段
    fn set_state(&self, state: AddModPackState) {
        self.update(|dto| dto.state = pack_state_id(state).into());
    }

    /// 设置主进度（value / all）
    fn set_now(&self, value: usize, all: Option<usize>) {
        self.update(|dto| {
            dto.now = value as u32;
            dto.total = all.unwrap_or(0) as u32;
        });
    }

    /// 设置子任务说明文字
    fn set_sub_text(&self, text: Option<String>) {
        self.update(|dto| dto.sub_text = text);
    }

    /// 设置子任务进度（value / all）
    fn set_sub_now(&self, value: usize, all: Option<usize>) {
        self.update(|dto| {
            dto.sub_now = value as u32;
            dto.sub_total = all.unwrap_or(0) as u32;
        });
    }
}

/// 安装阶段 ID（与前端 i18n 键对应）
pub(crate) fn pack_state_id(state: AddModPackState) -> &'static str {
    match state {
        AddModPackState::DownloadPack => "downloadPack",
        AddModPackState::ReadInfo => "readInfo",
        AddModPackState::GetInfo => "getInfo",
        AddModPackState::DownloadFile => "downloadFile",
        AddModPackState::Extract => "extract",
        AddModPackState::Done => "done",
    }
}

/// 构建安装进度回调（跟随调用方窗口）
pub(crate) fn pack_gui(window: &WebviewWindow) -> AddModPackGui {
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
pub(crate) fn prepare(
    window: &WebviewWindow,
) -> Result<(Arc<Mutex<AddWindowModel>>, CancellationToken), String> {
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

/// 扫描文件夹里可导入的实例（官方启动器 `versions/*` 与 MMC `instances/*`）
///
/// 选到 `.minecraft` 这类**装着若干实例**的目录时用：一个目录一个实例，
/// 由前端列出来让用户勾选要导入哪些（而不是把整个 `.minecraft` 当成一个实例）。
/// 扫描规则见 `mml_game::scan_game_from_path`。
///
/// **必须异步**：扫描是纯磁盘活（要读并解析各候选目录里的 json，modded 实例的
/// `config/` 下可能几百个），同步命令会占着主线程，窗口直接卡死 ——
/// 放 `spawn_blocking` 里跑，界面照常响应。
#[tauri::command]
pub async fn add_scan_folder(path: String) -> Result<Vec<FolderInstanceDto>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        mml_game::scan_game::scan_game_from_path(&path)
            .into_iter()
            .map(|item| FolderInstanceDto {
                name: item
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                path: item.display().to_string(),
            })
            .collect::<Vec<FolderInstanceDto>>()
    })
    .await
    .map_err(|e| e.to_string())
}

/// 列出压缩包内的条目路径（目录以 `/` 结尾，添加实例窗口预览内容树用）
///
/// **必须异步**：要打开压缩包并读出全部条目，整合包里几千个文件时会明显耗时，
/// 同步命令占主线程会把窗口冻住。
#[tauri::command]
pub async fn add_list_archive(path: String) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let archive = mml_base::archives::BaseArchive::open(&path).map_err(|e| e.to_string())?;
        Ok(archive
            .entries()
            .iter()
            .map(|e| e.name.clone())
            .collect::<Vec<String>>())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// 检测压缩包的整合包类型与推荐实例名（选择压缩包后自动填表用）
///
/// **必须异步**：要打开压缩包扫条目、再解析包内 manifest，同 `add_list_archive`。
#[tauri::command]
pub async fn add_detect_archive(path: String) -> Result<DetectedPackDto, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let pack = mml_game::add_game::detect_pack(&path).map_err(|e| e.to_string())?;
        Ok(DetectedPackDto {
            pack_type: pack.pack_type.id().to_string(),
            name: pack.name,
        })
    })
    .await
    .map_err(|e| e.to_string())?
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
    // 前端传的是分组 uuid；空 / 非法 = 默认分组
    let group = crate::windows::parse_group_id(group);
    let loader = parse_loader(&loader)?;
    let obj = InstanceSettingObj {
        uuid: Uuid::new_v4(),
        name,
        version,
        loader,
        loader_version,
        ..Default::default()
    };
    // mml-game 的安装 future 非 Send，放阻塞线程上 block_on 执行
    let gui = instance_gui(&window);
    let res = tauri::async_runtime::spawn_blocking(move || {
        // 分组不在实例配置里（kernel 的 group_save.json 管），创建时一并指定
        tauri::async_runtime::block_on(obj.create_instance_in_group(gui, group))
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
    // 前端传的是分组 uuid；空 / 非法 = 默认分组
    let group = crate::windows::parse_group_id(group);
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
///
/// CurseForge / Modrinth 两种是**真整合包**：登记成「下载整合包」那套安装任务
/// （任务表与进度事件见 [`super::add_modpack`]），于是标题栏的整合包指示器、
/// 进度弹窗、取消、装完刷新并选中新实例全部复用同一条路径，
/// 与在线安装的差别只剩"包从哪来"（所以它没有项目 ID，用占位 pid）。
/// 其余类型（MMC / HMCL / 直接解压等）行为不变：进度发到本窗口的进度弹窗。
///
/// 命令**等安装结束**再返回（新实例 uuid 得交给添加实例窗口做后续交互），
/// 期间进度由整合包任务那一套显示。
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
    // 前端传的是分组 uuid；空 / 非法 = 默认分组
    let group = crate::windows::parse_group_id(group);
    let pack = parse_pack_type(&pack_type)?;
    let gui = instance_gui(&window);
    let name = name.filter(|item| !item.trim().is_empty());

    // 整合包：进度回调换成任务那一份 —— 本窗口的安装进度弹窗不再出现，
    // 进度改由标题栏的整合包指示器 / 进度弹窗显示（与在线安装一致）
    let (progress, task) = if is_modpack(&pack) {
        let key = SourceInfo {
            pid: LOCAL_PACK_PID.to_string(),
            fid: path.clone(),
        };
        let display = name.clone().unwrap_or_else(|| pack_file_stem(&path));
        let (task, progress) = register_task(&app, key, "local", display, token.clone())?;
        (progress, Some(task))
    } else {
        (pack_gui(&window), None)
    };

    // 安装 future 非 Send：放阻塞线程上 block_on（与 add_create_new 等同一条路子）
    let install_token = token.clone();
    let file = path.clone();
    let res: Result<Uuid, String> = match tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(async {
            add_game::install_archive_from_file(
                &file,
                name,
                group,
                unselect,
                gui,
                progress,
                None,
                pack,
                install_token,
            )
            .await
        })
    })
    .await
    {
        Ok(Ok(uuid)) => Ok(uuid),
        Ok(Err(err)) => Err(err.to_string()),
        Err(err) => Err(err.to_string()),
    };

    // 终态：整合包任务自己记录并广播（成功时连同 instance-change 一起发），
    // 其它类型跟以前一样只补一条 instance-change
    match &task {
        Some(task) => finish_task(&app, task, &token, res.clone()),
        None => {
            if res.is_ok() {
                crate::windows::main::emit_instance_change(&app, "add");
            }
        }
    }

    res.map(|uuid| uuid.to_string())
}

/// 该压缩包类型是否为整合包（即内核里走整合包安装 worker 的那两种 manifest）
fn is_modpack(pack: &PackType) -> bool {
    matches!(pack, PackType::CurseForge | PackType::Modrinth)
}

/// 压缩包文件名（去路径与扩展名）：本地包没有项目元数据，任务显示名只能取它
fn pack_file_stem(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .map(|item| item.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string())
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
    // 前端传的是分组 uuid；空 / 非法 = 默认分组
    let group = crate::windows::parse_group_id(group);
    let gui = instance_gui(&window);
    let progress = pack_gui(&window);
    let uuid = tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(async {
            add_game::install_archive_from_url(
                &url,
                name,
                group,
                None,
                gui,
                progress,
                None,
                PackType::ArchivePack,
                token,
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
    let loader = parse_loader(&loader)?;
    mml_game::loader::loader_versions::get_loader_versions(&loader, &mc)
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

    /// 把步数转发为进度事件
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
///
/// **中断自动重试**：用户改代理时 `mml_net` 会中断全部在途请求，这一次查询随之失败。
/// 此时**自动用新客户端重跑一次**——因为旧请求绑定的是发出时那份客户端，
/// 只有重新发起才会走新代理。最多重试 [`RETRY_ON_ABORT`] 次，避免代理不通时死循环。
#[tauri::command]
pub async fn add_get_support_loaders(app: AppHandle, mc: String) -> Result<Vec<String>, String> {
    use std::sync::LazyLock;
    use tokio::sync::{Mutex, OnceCell};

    /// 因"中断"（改代理）而失败时允许的重试次数
    const RETRY_ON_ABORT: usize = 2;

    /// 支持列表查询结果缓存（版本号 -> 查询单飞单元）
    static SUPPORT_LOADERS_CACHE: LazyLock<Mutex<HashMap<String, Arc<OnceCell<Vec<String>>>>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));

    let mut attempt = 0usize;
    loop {
        // 取/建该版本的查询单元：并发请求共享同一次查询
        let cell = {
            let mut cache = SUPPORT_LOADERS_CACHE.lock().await;
            cache.entry(mc.clone()).or_default().clone()
        };
        let gui_app = app.clone();
        let mc_clone = mc.clone();
        let result = cell
            .get_or_try_init(|| async move {
                let gui: ProgressGui =
                    Some(Arc::new(SupportLoadersProgressGui { app: gui_app })
                        as Arc<dyn IProgressGui>);
                mml_game::loader::loader_versions::get_support_loaders(&mc_clone, gui).await
            })
            .await;

        match result {
            Ok(list) => return Ok(list.clone()),
            Err(e) => {
                // 失败不缓存：把这个单元摘掉，下次查询重新发起（拿到新代理 / 重新联网）
                {
                    let mut cache = SUPPORT_LOADERS_CACHE.lock().await;
                    // 只摘掉还是同一个单元的那条：期间可能已被别的请求换成新的，别误删
                    if cache
                        .get(&mc)
                        .is_some_and(|current| Arc::ptr_eq(current, &cell))
                    {
                        cache.remove(&mc);
                    }
                }

                // 被中断（用户改了代理）→ 用新客户端重跑一次
                if mml_net::is_aborted(&e) && attempt < RETRY_ON_ABORT {
                    attempt += 1;
                    mml_log::info(format!(
                        "加载器查询被中断，用新客户端重试（第 {attempt} 次）：mc={mc}"
                    ));
                    continue;
                }

                return Err(e.to_string());
            }
        }
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
    mml_game::get_version_types()
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
