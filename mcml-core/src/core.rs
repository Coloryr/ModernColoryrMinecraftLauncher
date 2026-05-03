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

use std::{fs::{self, File}, path::{Path, PathBuf}, sync::{OnceLock, RwLock}};

use const_format::formatcp;
use mcml_log::log;

use crate::{
    config::{config, config_save},
    events::core_stop_event,
    net::downloader::download_manager,
};

/// 启动器主版本号
pub const VERSION_NUM: i32 = 1;
/// 启动器日期
pub const DATE: &str = "20260503";
/// 启动器版本号
pub const VERSION: &str = formatcp!("1.{}.{DATE}", VERSION_NUM);

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
    if !arg.local.exists() {
        let res = fs::DirBuilder::new().recursive(true).create(&arg.local);
        if let Err(err) = res {
            panic!("Run local is not exists {}", err);
        }
    }

    CORE_ARG.set(arg).unwrap();

    BASE_DIR.set(CORE_ARG.get().unwrap().local.to_path_buf()).unwrap();

    log::start(BASE_DIR.get().unwrap().to_path_buf());

    log::info(format!("MCML start {}", VERSION));

    config_save::start();

    config::init(BASE_DIR.get().unwrap().to_path_buf());

    core_stop_event::add_stop_handler(|| config_save::stop());
    core_stop_event::add_stop_handler(|| download_manager::stop());
    core_stop_event::add_stop_handler(|| log::stop());

    *STATE.write().unwrap() = true;
}

pub fn stop() {
    log::info(String::from("MCML stop"));

    core_stop_event::invoke_stop();

    *STATE.write().unwrap() = false;
}
