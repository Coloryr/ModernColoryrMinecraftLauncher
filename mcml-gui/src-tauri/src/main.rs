// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;

use mcml_core::CoreInitObj;
use mcml_gui_lib::{err_box, gui_config, window_manager};
use mcml_names::names;
use mcml_sys::path_helper;

fn main() {
    let mut obj = CoreInitObj {
        path: Default::default(),
        curseforge_key: "$2a$10$6L8AkVsaGMcZR36i8XvCr.O4INa2zvDwMhooYdLZU0bb/E78AsT0m".to_string(),
        oauth_key: "aa0dd576-d717-4950-b257-a478d2c20968".to_string(),
    };

    let path = get_run_path();
    let temp = path.join("test");
    let res = path_helper::write_text(temp, "test write");
    obj.path = if res.is_err() { get_save_path() } else { path };

    // 窗口状态 / GUI 配置与核心共用同一运行路径
    window_manager::init(obj.path.clone());
    gui_config::init(obj.path.clone());

    if let Err(e) = mcml_core::init(obj) {
        err_box::fatal_error(e);
    }

    mcml_gui_lib::run()
}

fn get_save_path() -> PathBuf {
    let dir = dirs::data_dir()
        .or(dirs::data_local_dir())
        .or(dirs::home_dir())
        .unwrap();

    dir.join(names::MCML_DIR).to_path_buf()
}

#[cfg(debug_assertions)]
fn get_run_path() -> PathBuf {
    use mcml_names::names;
    use std::env;

    let exe_path = env::current_exe().expect("Failed to get exe path");
    let exe_dir = exe_path.parent().expect("Failed to get exe directory");

    exe_dir.join(names::MCML)
}

#[cfg(not(debug_assertions))]
fn get_config_path() -> Option<PathBuf> {
    let dir = dirs::data_local_dir()?;
    let file = dir.join(names::MCML_DIR).join("run");

    if file.exists() && file.is_file() {
        if let Ok(text) = path_helper::read_text(file) {
            let path = Path::new(&text);
            if path.exists() && path.is_dir() {
                return Some(path.to_path_buf());
            }
        }
    }

    None
}

#[cfg(not(debug_assertions))]
#[cfg(target_os = "windows")]
fn get_run_path() -> PathBuf {
    use mcml_names::names;
    use std::env;

    get_config_path().unwrap_or({
        let exe_path = env::current_exe().expect("Failed to get exe path");
        let exe_dir = exe_path.parent().expect("Failed to get exe directory");

        exe_dir.join(names::MCML)
    })
}

#[cfg(not(debug_assertions))]
#[cfg(target_os = "linux")]
fn get_run_path() -> PathBuf {
    use mcml_names::names;

    get_config_path().unwrap_or({
        let dir = dirs::home_dir().unwrap();

        dir.join(names::MCML_INNER_DIR).to_path_buf()
    })
}

#[cfg(not(debug_assertions))]
#[cfg(target_os = "macos")]
fn get_run_path() -> PathBuf {
    get_config_path().unwrap_or({ Path::new("/Users/shared/mcml/").to_path_buf() })
}
