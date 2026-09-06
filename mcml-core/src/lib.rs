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

use mcml_auth::{auths, oauth};
use mcml_base::events::EventHandler;
use mcml_config::config_save;
use mcml_log;
use mcml_names::i18_items::{
    error_type::{CoreResult, ErrorType::Panic},
    info_type::InfoType,
    panic_type::PanicType,
};
use mcml_net::curseforge_api;

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

    mcml_base::init(arg.path.to_path_buf());

    oauth::set_key(&arg.oauth_key);
    curseforge_api::set_key(&arg.curseforge_key);

    mcml_names::init(mcml_base::get_base_dir())?;
    mcml_log::start(mcml_base::get_base_dir())?;
    mcml_log::info_type(InfoType::CoreStart);
    mcml_config::init(mcml_base::get_base_dir())?;
    mcml_game::init(mcml_base::get_base_dir())?;
    mcml_jvms::init(mcml_base::get_base_dir())?;

    config_save::start();

    Ok(())
}

/// 加载配置
pub fn load() -> CoreResult<()> {
    mcml_net::init();
    auths::init();

    mcml_game::load()?;
    mcml_jvms::load();

    CORE_STOP_EVENT.add_handler(config_save::stop);
    CORE_STOP_EVENT.add_handler(mcml_downloader::stop);
    CORE_STOP_EVENT.add_handler(mcml_log::stop);

    *STATE.write().unwrap() = true;

    Ok(())
}

pub fn stop() {
    mcml_log::info(String::from("MCML stop"));

    invoke_core_stop();

    *STATE.write().unwrap() = false;
}
