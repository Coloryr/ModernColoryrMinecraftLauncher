//! 账户持久化存储模块
//!
//! 本模块管理启动器中所有账户的全局存储，提供账户的加载、保存、
//! 查询、导入和删除功能。
//!
//! # 存储方式
//!
//! - 内存中：使用 `LazyLock<RwLock<HashMap<UserKeyObj, LoginObj>>>` 作为全局单例存储
//! - 磁盘上：以 JSON 格式序列化为 `Vec<LoginObj>`，保存到程序内部目录的 `auths.json` 文件中
//!
//! # 使用流程
//!
//! 1. 启动时调用 [`init()`] 从磁盘加载已有账户
//! 2. 登录成功后调用 `LoginObj::save()` 保存新账户
//! 3. 登录前通过 [`get()`] 查找已保存的账户
//! 4. 切换账户时调用 `LoginObj::delete()` 移除旧账户

use std::{
    collections::HashMap,
    path::Path,
    sync::{LazyLock, RwLock},
};

use mcml_base::{inner_path, serialize_tools};
use mcml_config::config_save;
use mcml_names::{i18_items::error_type::CoreResult, names, uuids::AUTH_UUID};

use crate::{AuthType, LoginObj, UserKeyObj};

/// 全局账户存储
///
/// 使用 `LazyLock` 实现惰性初始化的全局单例，通过 `RwLock` 保证多线程安全。
/// 键为 `UserKeyObj`（UUID + 认证类型），值为 `LoginObj`（完整账户信息）。
static AUTHS: LazyLock<RwLock<HashMap<UserKeyObj, LoginObj>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 从磁盘加载账户列表到内存
///
/// 读取失败时会打印错误日志并触发一次保存操作。
///
/// # 参数
///
/// - `path`: 账户数据文件的路径
fn load<P: AsRef<Path>>(path: P) {
    if let Err(err) = import(path) {
        mcml_log::error_type(err);

        save();
    }
}

/// 将内存中所有账户持久化到磁盘
///
/// 数据以 `Vec<LoginObj>` 格式序列化为 JSON 文件。
fn save() {
    let auths: Vec<LoginObj> = AUTHS.read().unwrap().values().cloned().collect();
    let local = inner_path::get_inner_path().join(names::AUTH_FILE);
    config_save::save(AUTH_UUID, &auths, &local);
}

/// 初始化账户存储
///
/// 在启动器启动时调用。如果磁盘上已有账户数据文件则加载，
/// 否则创建空文件。此函数应仅在程序初始化阶段调用一次。
pub fn init() {
    let local = inner_path::get_inner_path().join(names::AUTH_FILE);

    if local.exists() {
        load(&local);
    } else {
        save();
    }
}

/// 根据 UUID 和认证类型查询已保存的账户
///
/// # 参数
///
/// - `uuid`: 账户标识（Minecraft UUID）
/// - `auth_type`: 账户认证类型
///
/// # 返回值
///
/// 找到则返回 `Some(LoginObj)` 克隆，未找到则返回 `None`
pub fn get(uuid: String, auth_type: AuthType) -> Option<LoginObj> {
    let auths = AUTHS.read().unwrap();
    auths.get(&UserKeyObj { uuid, auth_type }).cloned()
}

/// 从 JSON 文件批量导入账户列表
///
/// 文件内容应为 `Vec<LoginObj>` 格式的 JSON 数组。
/// 导入的账户会合并到已有存储中（使用 `UserKeyObj` 作为唯一键）。
///
/// # 参数
///
/// - `file`: JSON 文件路径
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回反序列化错误
pub fn import<P: AsRef<Path>>(file: P) -> CoreResult<()> {
    let json = serialize_tools::json_from_file::<Vec<LoginObj>>(file)?;

    let mut auths = AUTHS.write().unwrap();

    for item in json.into_iter() {
        auths.insert(item.get_key(), item);
    }

    Ok(())
}

/// 清除所有已保存的账户（内存和磁盘）
///
/// 谨慎使用，此操作不可逆。
pub fn clear_auths() {
    let mut auths = AUTHS.write().unwrap();
    auths.clear();

    save();
}

impl LoginObj {
    /// 将当前账户保存到全局存储并持久化到磁盘
    ///
    /// 如果已存在相同键（UUID + 认证类型）的账户，则覆盖更新。
    pub fn save(&self) {
        let key = self.get_key();
        let mut auths = AUTHS.write().unwrap();

        auths.insert(key, self.clone());

        save();
    }

    /// 从全局存储中删除当前账户并更新磁盘数据
    pub fn delete(&self) {
        let key = self.get_key();

        let mut auths = AUTHS.write().unwrap();

        auths.remove(&key);

        save();
    }
}
