//! M²L 启动器 Tauri 壳
//!
//! 当前阶段：主窗口数据/操作已接入真实 IPC（见 `windows/main.rs`），
//! 其余窗口仍使用前端模拟数据。通用数据模型见 `models/`。
//! 每个窗口的规格 / 专属模型 / 创建操作 / 窗口按钮调用的方法见 `windows/`。
//! 所有窗口的创建 / 聚焦 / 关闭统一由 `window_manager.rs` 处理。

use mml_names::i18;

use crate::windows::{download, main};

pub mod collect_utils;
pub mod dtos;
pub mod err_box;
pub mod gui_config;
pub mod image_manager;
pub mod windows;

include!(concat!(env!("OUT_DIR"), "/invokes_gen.rs"));

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("mml-image", move |_app, request, responder| {
            // 该回调跑在 WebView2 主线程的窗口过程里，不在 tokio 运行时上下文内，
            // 用 `tokio::spawn` 会 panic（no reactor running）；必须走 Tauri 的全局 Handle
            tauri::async_runtime::spawn(async move {
                image_manager::url_image(request, responder).await;
            });
        })
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(windows::on_window_event)
        .setup(|app| {
            if let Err(e) = windows::show_main_window(app.handle()) {
                err_box::fatal_error_text(&e);
            }

            // Java 列表变更（mml_jvms 添加 / 删除 / 配置加载完成）→ 通知前端刷新
            let handle = app.handle().clone();
            mml_jvms::add_jvm_change(move || {
                main::emit_java_change(&handle);
            });

            // 下载器：挂接 UI 回调（转发为前端事件）并启动下载线程池
            let handle = app.handle().clone();
            mml_downloader::set_gui_handel(Box::new(download::DownloadGuiHook::new(handle)));
            mml_downloader::start();

            // 收藏数据与核心无关，但必须等 `mml_core::init` 里的日志系统起来后再读，
            // 否则解析失败会没有输出、看起来像没加载
            if let Err(err) = collect_utils::load() {
                mml_log::error_type(err);
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match mml_core::load() {
                    Ok(()) => {
                        main::emit_load_done(&handle, None);
                    }
                    Err(err) => {
                        mml_log::error_type(err.clone());
                        main::emit_load_done(&handle, Some(i18::get_error(err)));
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri_commands!())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
