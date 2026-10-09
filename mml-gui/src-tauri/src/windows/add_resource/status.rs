//! 添加资源窗口的下载状态：任务总览快照 / 状态事件 / 下载器回调
//!
//! 从 `add_resource/mod.rs` 拆出来的。任务表 `DOWNLOAD_NOW` 仍留在 `super`（mod.rs）——
//! 列表与下载执行都要用它，放这里会变成子模块互相依赖。

use mml_downloader::download_item::DownloadItem;
use tauri::{AppHandle, Emitter, Manager};

use crate::dtos::add_resource_dto::{ResourceStatusDto, ResourceTaskDto};
use crate::listens;

use super::DOWNLOAD_NOW;

/// 资源下载任务总览快照（事件负载 / 查询返回）
pub(super) fn build_status(app: &AppHandle) -> ResourceStatusDto {
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
#[gui_macros::ipc_group("add_resource")]
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
