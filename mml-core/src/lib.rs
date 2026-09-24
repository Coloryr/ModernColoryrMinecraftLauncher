//! M²L 启动器内核总入口
//!
//! 聚合 mml-core 下各子 crate：按顺序完成初始化（init）、配置加载（load）、
//! 停止（stop），并提供核心停止事件（core stop）的注册与触发。
//! 各子 crate 的职责见其自身的 `//!` 头部说明。

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

/// 核心是否已通过 load 完成加载
static STATE: RwLock<bool> = RwLock::new(false);

/// 核心停止事件（stop 时触发，各子 crate 在此挂清理回调）
static CORE_STOP_EVENT: LazyLock<EventHandler> = LazyLock::new(|| EventHandler::new());

/// 注册核心停止事件回调
///
/// - `handler`: 停止时执行的回调
///
/// # 返回值
///
/// 返回回调ID（remove_core_stop 用）
pub fn add_core_stop<F>(handler: F) -> u64
where
    F: Fn() + Send + Sync + 'static,
{
    CORE_STOP_EVENT.add_handler(Box::new(handler))
}

/// 移除核心停止事件回调
///
/// - `id`: add_core_stop 返回的回调ID
pub fn remove_core_stop(id: u64) {
    CORE_STOP_EVENT.remove_handle(id);
}

/// 触发核心停止事件（执行全部已注册回调）
pub fn invoke_core_stop() {
    CORE_STOP_EVENT.emit();
}

/// 核心是否已加载完成
///
/// # 返回值
///
/// load 完成后为 true，stop 后回到 false
pub fn get_state() -> bool {
    return *STATE.read().unwrap();
}

/// 初始化核心
/// 这一步只设置允许目录，不加载内容
///
/// - `arg`: 核心参数
///
/// # 返回值
///
/// 运行路径非法或子模块初始化失败时返回相应错误，成功返回 `Ok(())`
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
///
/// # 返回值
///
/// 子模块加载失败时返回相应错误，成功返回 `Ok(())`
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

/// 停止核心（触发停止事件，各子 crate 的清理回调随之执行）
pub fn stop() {
    mml_log::info(String::from("M²L stop"));

    invoke_core_stop();

    *STATE.write().unwrap() = false;
}
