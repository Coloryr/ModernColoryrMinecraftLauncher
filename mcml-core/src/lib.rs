pub mod events;

/// 核心初始化参数
#[derive(Debug)]
pub struct CoreInitObj {
    /// 运行路径
    pub local: PathBuf,
    /// 微软登录密钥
    pub oauth_key: String,
    /// CF平台密钥
    pub curseforge_key: String,
}

impl CoreInitObj {
    /// 创建核心初始化参数
    pub fn new(local: PathBuf, oauth_key: String, curseforge_key: String) -> Self {
        CoreInitObj {
            local,
            oauth_key,
            curseforge_key,
        }
    }
}

use std::{
    fs::{self},
    path::PathBuf,
    sync::{OnceLock, RwLock},
};

use mcml_log;
use mcml_names::{i18, i18_items::info_type::InfoType, i18_items::panic_type::PanicType};

use crate::events::core_stop_event;

/// 基础运行路径
pub static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();
/// 核心参数
pub static CORE_ARG: OnceLock<CoreInitObj> = OnceLock::new();
/// 是否为第一次启动
pub static NEW_START: RwLock<bool> = RwLock::new(false);

static STATE: RwLock<bool> = RwLock::new(false);

pub fn get_state() -> bool {
    return *STATE.read().unwrap();
}

/// 初始化核心
/// arg 核心参数
pub fn init(arg: CoreInitObj) {
    if arg.local.as_os_str().is_empty() {
        panic!("{}", i18::get_panic(PanicType::CoreArgLocalEmpty));
    }
    if !arg.local.exists() {
        let res = fs::DirBuilder::new().recursive(true).create(&arg.local);
        if let Err(err) = res {
            panic!(
                "{}",
                i18::get_panic(PanicType::CoreArgLocalError(err.to_string()))
            );
        }
    }

    CORE_ARG.set(arg).unwrap();

    let dir = BASE_DIR.get_or_init(|| CORE_ARG.get().unwrap().local.to_path_buf());

    mcml_names::init(dir);

    mcml_log::info_type(InfoType::CoreStart);

    mcml_log::start(dir);
    mcml_config::config_save::start();
    mcml_downloader::start();
    mcml_http::init();
    mcml_config::init(dir);

    core_stop_event::add_stop_handler(|| mcml_config::config_save::stop());
    core_stop_event::add_stop_handler(|| mcml_downloader::stop());
    core_stop_event::add_stop_handler(|| mcml_log::stop());

    *STATE.write().unwrap() = true;
}

pub fn stop() {
    mcml_log::info(String::from("MCML stop"));

    core_stop_event::invoke_stop();

    *STATE.write().unwrap() = false;
}
