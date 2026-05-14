use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{OnceLock, RwLock},
};

use mcml_base::{inner_path, path_helper};
use mcml_config::config_save;
use mcml_names::{names, uuids::AUTH_UUID};

use crate::{AuthType, LoginObj, UserKeyObj};

/// 保存的账户
static AUTHS: OnceLock<RwLock<HashMap<UserKeyObj, LoginObj>>> = OnceLock::new();
/// 账户数据库保存位置s
static LOCAL: OnceLock<PathBuf> = OnceLock::new();

fn load() {
    import(LOCAL.get().unwrap());
}

fn save() {
    let auths: Vec<LoginObj> = AUTHS
        .get()
        .unwrap()
        .read()
        .unwrap()
        .values()
        .cloned()
        .collect();

    config_save::save(AUTH_UUID, &auths, LOCAL.get().unwrap());
}

/// 初始化
pub fn init() {
    LOCAL
        .set(inner_path::inner().join(names::NAME_AUTH_FILE))
        .unwrap();
    AUTHS.set(RwLock::new(HashMap::new())).unwrap();

    if LOCAL.get().unwrap().exists() {
        load();
    } else {
        save();
    }
}

/// 获取账户
/// - `uuid`: 账户标识
/// - `auth_type`: 账户类型
pub fn get(uuid: String, auth_type: AuthType) -> Option<LoginObj> {
    let auths = AUTHS.get().unwrap().read().unwrap();
    auths.get(&UserKeyObj { uuid, auth_type }).cloned()
}

pub fn import(file: &PathBuf) {
    let temp = path_helper::open_read(file).unwrap();
    let json = serde_json::from_reader::<_, Vec<LoginObj>>(temp);

    let mut auths = AUTHS.get().unwrap().write().unwrap();

    if json.is_ok() {
        let json = json.unwrap();
        for item in json.into_iter() {
            auths.insert(item.get_key(), item);
        }
    }
}

pub fn clear_auths() {
    let mut auths = AUTHS.get().unwrap().write().unwrap();
    auths.clear();

    save();
}

impl LoginObj {
    /// 保存账户
    pub fn save(&self) {
        let key = self.get_key();
        let mut auths = AUTHS.get().unwrap().write().unwrap();

        auths.insert(key, self.clone());

        save();
    }

    /// 删除账户
    pub fn delete(&self) {
        let key = self.get_key();

        let mut auths = AUTHS.get().unwrap().write().unwrap();

        auths.remove(&key);

        save();
    }
}
