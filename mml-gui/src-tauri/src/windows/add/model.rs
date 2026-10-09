//! 添加实例窗口的模型与进度回调：窗口模型、重名确认对话框、进度桥
//!
//! 从 `add/mod.rs` 拆出来的。这一层是"窗口运行态"：`AddWindowModel`（取消令牌 / 关闭保护 /
//! 对话框应答通道）与两个 `IAdd*Gui` 实现（重名确认、进度上报）。
//! `AddWindowModel` 与 `instance_gui` 按原路径再导出（窗口生命周期、下载整合包窗口在用）。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use mml_game::GameInstance;
use mml_game::gui_hook::{
    AddInstanceGui, AddModPackGui, AddModPackState, IAddInstanceGui, IAddModPackGui,
};
use tauri::{Emitter, WebviewWindow};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;

use crate::dtos::{NameConflictDto, PackProgressDto};
use crate::listens;
use crate::windows;
use crate::windows::modpack_task::pack_state_id;

/// 添加实例窗口模型：只存运行态（取消令牌 / 关闭保护 / 重名对话框应答通道），不落盘
pub(crate) struct AddWindowModel {
    /// 进行中安装任务的取消令牌（None = 当前无任务）
    cancel: Mutex<Option<CancellationToken>>,
    /// 关闭保护（查询数据期间为 true，`CloseRequested` 阶段拒绝关闭）
    close_guard: AtomicBool,
    /// 重名确认对话框的应答通道（id → 发送端，前端答复后唤醒创建流程）
    dialog: Mutex<HashMap<u32, oneshot::Sender<bool>>>,
}

impl Default for AddWindowModel {
    fn default() -> Self {
        Self::new()
    }
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
    ///
    /// 覆盖旧令牌前**先取消它**：同窗口连着起两次安装时，旧任务的令牌会被悄悄换掉，
    /// 之后 `add_cancel` 取到的只有新令牌 —— 旧任务就再也取消不掉，只能等它跑完。
    pub(crate) fn set_cancel(&self, token: CancellationToken) {
        if let Some(old) = self.cancel.lock().unwrap().replace(token) {
            old.cancel();
        }
    }

    /// 关窗时把还在等应答的重名确认对话框全部按"拒绝"收尾
    ///
    /// 不这么做：用户没答就关窗 → 安装 future 永久 pending（任务停在"进行中"），
    /// 发送端也一直留在表里。唤醒它们即"窗口关了 = 拒绝"，与 `ask` 的注释一致。
    pub(crate) fn reject_all_dialogs(&self) {
        let pending: Vec<oneshot::Sender<bool>> = self
            .dialog
            .lock()
            .unwrap()
            .drain()
            .map(|(_, tx)| tx)
            .collect();
        for tx in pending {
            let _ = tx.send(false);
        }
    }

    /// 取走当前取消令牌（取后为 None）
    pub(crate) fn take_cancel(&self) -> Option<CancellationToken> {
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
    pub(crate) fn take_dialog(&self, id: u32) -> Option<oneshot::Sender<bool>> {
        self.dialog.lock().unwrap().remove(&id)
    }
}

/// 取添加实例窗口模型
///
/// 按 kind 取而不是按调用方窗口：单窗口模式下"添加实例"只是主窗口里的一页，
/// 命令的调用方窗口是主窗口，按窗口取会拿到主窗口模型（表现为 `err.modelMissing`）。
/// 模型由页面挂载时的 `window_ensure_model` 建立、切走时 `window_drop_model` 释放。
pub(super) fn model(_window: &WebviewWindow) -> Result<Arc<Mutex<AddWindowModel>>, String> {
    windows::model_for_kind("add").ok_or_else(|| "err.modelMissing".to_string())
}

/// 重名确认对话框事件 id 自增
pub(super) static DIALOG_ID: AtomicU32 = AtomicU32::new(1);

/// 实例重名确认对话框事件（kind：overwrite 覆盖 / rename 自动改名）
#[gui_macros::emit]
pub(super) fn emit_add_name_conflict(window: &WebviewWindow, dto: NameConflictDto) {
    let _ = window.emit_to(window.label(), listens::ADD_NAME_CONFLICT, dto);
}

/// 实例创建重名确认：向添加实例窗口发事件弹窗，等待用户在对话框上答复
pub(super) struct AddInstanceDialogGui {
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
pub(super) fn emit_add_pack_progress(window: &WebviewWindow, dto: PackProgressDto) {
    let _ = window.emit_to(window.label(), listens::ADD_PACK_PROGRESS, dto);
}

/// 整合包安装进度回调：把安装状态 / 进度转发为前端事件（弹窗显示）
pub(super) struct PackProgressGui {
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

/// 构建安装进度回调（跟随调用方窗口）
pub(super) fn pack_gui(window: &WebviewWindow) -> AddModPackGui {
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
