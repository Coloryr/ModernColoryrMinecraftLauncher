//! MCML 启动器 Tauri 壳
//!
//! 当前阶段：主窗口数据/操作已接入真实 IPC（见 `windows/main.rs`），
//! 其余窗口仍使用前端模拟数据。通用数据模型见 `models/`。
//! 每个窗口的规格 / 专属模型 / 创建操作 / 窗口按钮调用的方法见 `windows/`。

pub mod models;
pub mod windows;
pub mod window_manager;
pub mod gui_config;

use std::sync::Mutex;

use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            // 创建主窗口：几何直接带进创建参数（而非创建后再 set），
            // 这样窗口首次显示就在保存的位置/大小，不会先按默认几何显示再跳变。
            let geom =
                windows::state::window_state_for(app.handle(), windows::state::MAIN_WINDOW_UUID);
            let builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("MCML 启动器")
                .min_inner_size(900.0, 600.0);
            let win = match geom {
                Some(g) => builder
                    .inner_size(g.width as f64, g.height as f64)
                    .position(g.x as f64, g.y as f64)
                    .build(),
                None => builder.inner_size(1100.0, 720.0).center().build(),
            };
            if let Err(e) = win {
                eprintln!("创建主窗口失败: {e}");
            }
            // 初始化主窗口数据存储（从磁盘加载，做环境检测）
            let store = windows::main::MainWindowModel::init(app.handle());
            app.manage(Mutex::new(store));
            // 初始化账户存储
            let account_store = windows::account::AccountStore::init(app.handle());
            app.manage(Mutex::new(account_store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 窗口状态 / 配置（windows/state.rs）
            windows::state::get_gui_config,
            windows::state::save_gui_config,
            windows::state::get_window_states,
            windows::state::save_window_state,
            windows::state::get_main_window_uuid,
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
            // 窗口打开 / 添加实例窗口
            windows::open_window,
            windows::add::list_dir
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
