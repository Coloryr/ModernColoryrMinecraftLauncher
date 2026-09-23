/// 核心初始化参数
#[derive(Debug)]
pub struct CoreInitObj {
    /// 运行路径
    pub path: PathBuf,
    /// 微软登录密钥
    pub oauth_key: String,
    /// CF平台密钥
    pub curseforge_key: String,
}

use std::{
    path::PathBuf,
    sync::{LazyLock, RwLock},
};

use mml_auth::{auths, oauth};
use mml_base::events::EventHandler;
use mml_config::config_save;
use mml_log;
use mml_names::i18_items::{
    error_type::{CoreResult, ErrorType::Panic},
    info_type::InfoType,
    panic_type::PanicType,
};
use mml_net::curseforge_api;

/// 是否为第一次启动
pub static NEW_START: RwLock<bool> = RwLock::new(false);

static STATE: RwLock<bool> = RwLock::new(false);

static CORE_STOP_EVENT: LazyLock<EventHandler> = LazyLock::new(|| EventHandler::new());

pub fn add_core_stop<F>(handler: F) -> u64
where
    F: Fn() + Send + Sync + 'static,
{
    CORE_STOP_EVENT.add_handler(Box::new(handler))
}

pub fn remove_core_stop(id: u64) {
    CORE_STOP_EVENT.remove_handle(id);
}

pub fn invoke_core_stop() {
    CORE_STOP_EVENT.emit();
}

pub fn get_state() -> bool {
    return *STATE.read().unwrap();
}

/// 初始化核心
/// 这一步只设置允许目录，不加载内容
///
/// arg 核心参数
pub fn init(arg: CoreInitObj) -> CoreResult<()> {
    if arg.path.as_os_str().is_empty() {
        return Err(Panic(PanicType::CoreArgLocalEmpty));
    }
    if !arg.path.exists() {
        return Err(Panic(PanicType::CoreArgLocalError));
    }

    mml_base::init(arg.path.to_path_buf());

    oauth::set_key(&arg.oauth_key);
    curseforge_api::set_key(&arg.curseforge_key);

    let path = mml_base::get_base_dir();
    mml_names::init(&path)?;
    mml_log::start(&path)?;
    mml_log::info_type(InfoType::CoreStart);
    mml_tex_draw::init(&path)?;
    mml_config::init(&path)?;
    mml_game::init(&path)?;
    mml_jvms::init(&path)?;
    mml_downloader::init(&path)?;

    config_save::start();

    Ok(())
}

/// 加载配置
pub fn load() -> CoreResult<()> {
    mml_net::init();
    auths::init();
    mml_jvms::load();

    mml_game::load()?;
    mml_tex_draw::load()?;

    CORE_STOP_EVENT.add_handler(config_save::stop);
    CORE_STOP_EVENT.add_handler(mml_downloader::stop);
    CORE_STOP_EVENT.add_handler(mml_log::stop);

    *STATE.write().unwrap() = true;

    Ok(())
}

pub fn stop() {
    mml_log::info(String::from("M²L stop"));

    invoke_core_stop();

    *STATE.write().unwrap() = false;
}
