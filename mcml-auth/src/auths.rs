/// 账户存储
use std::{
    collections::HashMap,
    path::Path,
    sync::{LazyLock, RwLock},
};

use mcml_base::{inner_path, serialize_tools};
use mcml_config::config_save;
use mcml_names::{i18_items::error_type::CoreResult, names, uuids::AUTH_UUID};

use crate::{AuthType, LoginObj, UserKeyObj};

/// 保存的账户
static AUTHS: LazyLock<RwLock<HashMap<UserKeyObj, LoginObj>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 加载登陆的账户列表
/// - `local`
fn load<P: AsRef<Path>>(path: P) {
    if let Err(err) = import(path) {
        mcml_log::error_type(err);

        save();
    }
}

/// 保持账户列表
fn save() {
    let auths: Vec<LoginObj> = AUTHS.read().unwrap().values().cloned().collect();
    let local = inner_path::get_inner_path().join(names::AUTH_FILE);
    config_save::save(AUTH_UUID, &auths, &local);
}

/// 初始化
pub fn init() {
    let local = inner_path::get_inner_path().join(names::AUTH_FILE);

    if local.exists() {
        load(&local);
    } else {
        save();
    }
}

/// 获取账户
/// - `uuid`: 账户标识
/// - `auth_type`: 账户类型
pub fn get(uuid: String, auth_type: AuthType) -> Option<LoginObj> {
    let auths = AUTHS.read().unwrap();
    auths.get(&UserKeyObj { uuid, auth_type }).cloned()
}

/// 导入账户列表
/// - `file`: 文件位置
pub fn import<P: AsRef<Path>>(file: P) -> CoreResult<()> {
    let json = serialize_tools::json_from_file::<Vec<LoginObj>>(file)?;

    let mut auths = AUTHS.write().unwrap();

    for item in json.into_iter() {
        auths.insert(item.get_key(), item);
    }

    Ok(())
}

/// 清理所有账户
pub fn clear_auths() {
    let mut auths = AUTHS.write().unwrap();
    auths.clear();

    save();
}

impl LoginObj {
    /// 保存账户
    pub fn save(&self) {
        let key = self.get_key();
        let mut auths = AUTHS.write().unwrap();

        auths.insert(key, self.clone());

        save();
    }

    /// 删除账户
    pub fn delete(&self) {
        let key = self.get_key();

        let mut auths = AUTHS.write().unwrap();

        auths.remove(&key);

        save();
    }
}
