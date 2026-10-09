//! 主窗口的启动 / 停止与运行态查询
//!
//! 从 `main/mod.rs` 拆出来的。启动流程的状态与日志都走事件（`launch-state` / `game-log` /
//! `game-exit`），这里只负责发起与转发。命令带 `#[gui_macros::ipc_group("main")]`
//! 把组键钉回 `main`（见 AGENTS.md §4）。

use std::time::Duration;

use tauri::{AppHandle, Emitter, WebviewWindow};

use crate::dtos::{ExitEvent, StateEvent};
use crate::listens;
use crate::windows::log::emit_log_line;

use super::model;

/// 启动游戏（占位：标记运行 + 发事件；接入核心后替换为真实启动）
///
/// 启动用户名由后端自己从当前账户解析（`auths::get_current()`），前端不传
#[gui_macros::ipc_group("main")]
#[tauri::command]
pub fn main_launch_game(app: AppHandle, window: WebviewWindow, uuid: String) -> Result<(), String> {
    mml_log::info(format!("[launch_game] uuid={uuid}"));
    let store = model(&window)?;
    {
        let mut store = store.lock().unwrap();
        if store.running.contains(&uuid) {
            return Err("err.instanceRunning".to_string());
        }
        store.running.insert(uuid.clone());
    }
    emit_launch_state(
        &app,
        StateEvent {
            uuid: uuid.clone(),
            state: "launching".into(),
            progress: None,
        },
    );
    emit_log_line(&app, &uuid, "游戏启动中…", true);

    // 占位：3 秒后发出退出事件（真实启动需接入 mml-core）
    // 直接持有模型的 Arc：模型跟随主窗口，销毁后后台线程仍能安全收尾
    let store2 = store.clone();
    let uuid2 = uuid.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(3));
        emit_log_line(&app, &uuid2, "游戏进程已退出", false);
        emit_game_exit(
            &app,
            ExitEvent {
                uuid: uuid2.clone(),
                code: 0,
            },
        );
        let mut s = store2.lock().unwrap();
        s.running.remove(&uuid2);
    });
    Ok(())
}

/// 停止游戏
#[gui_macros::ipc_group("main")]
#[tauri::command]
pub fn main_stop_game(app: AppHandle, window: WebviewWindow, uuid: String) -> Result<(), String> {
    let store = model(&window)?;
    let mut store = store.lock().unwrap();
    store.running.remove(&uuid);
    emit_game_exit(&app, ExitEvent { uuid, code: 0 });
    Ok(())
}

/// 获取运行中实例
#[gui_macros::ipc_group("main")]
#[tauri::command]
pub fn main_get_running(window: WebviewWindow) -> Vec<String> {
    let Ok(store) = model(&window) else {
        mml_log::warn(String::from("[main_get_running] 主窗口模型未初始化"));
        return Vec::new();
    };
    store.lock().unwrap().running.iter().cloned().collect()
}

/// 启动状态事件
#[gui_macros::emit]
fn emit_launch_state(app: &AppHandle, event: StateEvent) {
    let _ = app.emit(listens::LAUNCH_STATE, event);
}

/// 游戏退出事件
#[gui_macros::emit]
fn emit_game_exit(app: &AppHandle, event: ExitEvent) {
    let _ = app.emit(listens::GAME_EXIT, event);
}
