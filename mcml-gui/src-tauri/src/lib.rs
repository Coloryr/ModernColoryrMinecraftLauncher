//! MCML 启动器 Tauri 壳
//!
//! 当前阶段：主窗口数据/操作已接入真实 IPC（见 `windows/main.rs`），
//! 其余窗口仍使用前端模拟数据。通用数据模型见 `models/`。
//! 每个窗口的规格 / 专属模型 / 创建操作 / 窗口按钮调用的方法见 `windows/`。
//! 所有窗口的创建 / 聚焦 / 关闭统一由 `window_manager.rs` 处理。

pub mod gui_config;
pub mod models;
pub mod window_manager;
pub mod windows;
pub mod err_box;

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
            let account_store = windows::account::AccountStore::init(app.handle());
            app.manage(Mutex::new(account_store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 窗口状态 / 配置（window_manager.rs）
            window_manager::get_gui_config,
            window_manager::save_gui_config,
            window_manager::get_window_states,
            window_manager::save_window_state,
            // 窗口打开 / 关闭（window_manager.rs）
            window_manager::open_window,
            window_manager::close_window,
            // 账户（windows/account.rs）
            windows::account::get_accounts,
            windows::account::add_account,
            windows::account::remove_account,
            windows::account::refresh_account_token,
            windows::account::set_current_account,
            // 主窗口（windows/main.rs）
            windows::main::init_core,
            windows::main::get_instances,
            windows::main::get_groups,
            windows::main::get_java_list,
            windows::main::get_versions,
            windows::main::add_group,
            windows::main::remove_group,
            windows::main::move_group,
            windows::main::create_instance,
            windows::main::rename_instance,
            windows::main::update_instance,
            windows::main::delete_instance,
            windows::main::move_instance,
            windows::main::launch_game,
            windows::main::stop_game,
            windows::main::get_game_log,
            windows::main::get_running,
            // 添加实例窗口（windows/add.rs）
            windows::add::list_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
