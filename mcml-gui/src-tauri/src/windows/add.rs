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
use mcml_game::gui_hook::{AddInstanceGui, IAddInstanceGui, IProgressGui, ProgressGui};
use mcml_game::launcher::instance_setting_obj::InstanceSettingObj;
use mcml_game::loader::LoaderType;
use mcml_game::GameInstance;
use tauri::{AppHandle, Emitter, WebviewWindow};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dtos::{DirEntry, LoaderProgressDto, NameConflictDto};
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

/// 导入整合包压缩包为实例（packType：CurseForge / Modrinth / McMod / 本地）
#[tauri::command]
pub async fn add_import_archive(
    window: WebviewWindow,
    app: AppHandle,
    path: String,
    pack_type: String,
    name: Option<String>,
    group: Option<String>,
) -> Result<String, String> {
    let (_store, token) = prepare(&window)?;
    let pack = parse_pack_type(&pack_type)?;
    let gui = instance_gui(&window);
    let uuid = tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(async {
            add_game::install_archive_from_file(
                &path, name, group, None, gui, None, None, pack, token,
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
    let uuid = tauri::async_runtime::spawn_blocking(move || {
        tauri::async_runtime::block_on(async {
            add_game::install_archive_from_url(
                &url, name, group, None, gui, None, None, PackType::ArchivePack, token,
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
/// 每查完一个加载器向本窗口发一次进度事件（前端显示进度条）。
#[tauri::command]
pub async fn add_get_support_loaders(app: AppHandle, mc: String) -> Result<Vec<String>, String> {
    if !mcml_net::is_init() {
        return Ok(Vec::new());
    }
    let gui: ProgressGui =
        Some(Arc::new(SupportLoadersProgressGui { app }) as Arc<dyn IProgressGui>);
    mcml_game::loader::loader_versions::get_support_loaders(&mc, gui)
        .await
        .map_err(|e| e.to_string())
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

