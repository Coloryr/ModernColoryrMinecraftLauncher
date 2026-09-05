//! MCML 启动器 Tauri 壳
//!
//! 当前阶段：主窗口数据/操作已接入真实 IPC（见 `windows/main.rs`），
//! 其余窗口仍使用前端模拟数据。通用数据模型见 `models/`。
//! 每个窗口的规格 / 专属模型 / 创建操作 / 窗口按钮调用的方法见 `windows/`。
//! 所有窗口的创建 / 聚焦 / 关闭统一由 `window_manager.rs` 处理。

use mcml_names::i18;

use crate::windows::main;

pub mod dtos;
pub mod err_box;
pub mod gui_config;
pub mod image_manager;
pub mod models;
pub mod window_manager;
pub mod windows;

include!(concat!(env!("OUT_DIR"), "/invokes_gen.rs"));

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .register_asynchronous_uri_scheme_protocol("mcml-image", move |_app, request, responder| {
            tokio::spawn(async move {
                image_manager::url_image(request, responder).await;
            });
        })
        .plugin(tauri_plugin_dialog::init())
        .on_window_event(window_manager::on_window_event)
        .setup(|app| {
            if let Err(e) = window_manager::show_main_window(app.handle()) {
                err_box::fatal_error_text(&e);
            }

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match mcml_core::load() {
                    Ok(()) => {
                        main::emit_load_done(&handle, None);
                    }
                    Err(err) => {
                        mcml_log::error_type(err.clone());
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
