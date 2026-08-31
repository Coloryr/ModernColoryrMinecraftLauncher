//! MCML 启动器 Tauri 壳
//!
//! 当前阶段：主窗口数据/操作已接入真实 IPC（见 `windows/main.rs`），
//! 其余窗口仍使用前端模拟数据。通用数据模型见 `models/`。
//! 每个窗口的规格 / 专属模型 / 创建操作 / 窗口按钮调用的方法见 `windows/`。
//! 所有窗口的创建 / 聚焦 / 关闭统一由 `window_manager.rs` 处理。

pub mod dtos;
pub mod err_box;
pub mod gui_config;
pub mod models;
pub mod window_manager;
pub mod windows;

use std::sync::Mutex;

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 主窗口由窗口管理器创建（恢复上次几何）
            if let Err(e) = window_manager::create_main(app.handle()) {
                err_box::fatal_error_text(&e);
            }

            let store = windows::main::MainWindowModel::init(app.handle());
            app.manage(Mutex::new(store));

            // 账户存储走 mcml-auth：启动后台配置保存线程并加载 auth.json
            // （接入完整 mcml_core::init 后这两行由 core 启动流程接管）
            mcml_config::config_save::start();
            mcml_auth::auths::init();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 窗口状态 / 配置（window_manager.rs）
            window_manager::window_get_gui_config,
            window_manager::window_save_gui_config,
            window_manager::window_get_window_states,
            window_manager::window_save_window_state,
            // 窗口打开 / 关闭（window_manager.rs）
            window_manager::window_open_window,
            window_manager::window_close_window,
            // 账户（windows/account.rs）
            windows::account::account_get_accounts,
            windows::account::account_add_account,
            windows::account::account_remove_account,
            windows::account::account_refresh_account_token,
            windows::account::account_set_current_account,
            // 主窗口（windows/main.rs）
            windows::main::main_init_core,
            windows::main::main_get_instances,
            windows::main::main_get_groups,
            windows::main::main_get_java_list,
            windows::main::main_get_versions,
            windows::main::main_add_group,
            windows::main::main_remove_group,
            windows::main::main_move_group,
            windows::main::main_create_instance,
            windows::main::main_rename_instance,
            windows::main::main_update_instance,
            windows::main::main_delete_instance,
            windows::main::main_move_instance,
            windows::main::main_launch_game,
            windows::main::main_stop_game,
            windows::main::main_get_game_log,
            windows::main::main_get_running,
            // 添加实例窗口（windows/add.rs）
            windows::add::add_list_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
