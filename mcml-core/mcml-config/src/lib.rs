/// 总配置文件
pub mod config_obj;
pub mod config_save;

use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use mcml_base::serialize_tools;
use mcml_log;
use mcml_names::{
    i18_items::error_type::{ErrorType, FileSystemErrorData},
    names, uuids,
};

use crate::config_obj::ConfigObj;

static CONFIG: OnceLock<RwLock<ConfigObj>> = OnceLock::new();
static FILE: OnceLock<PathBuf> = OnceLock::new();

/// 初始化运行路径
/// - `dir`: 运行路径
pub fn init<P: AsRef<Path>>(dir: P) -> bool {
    FILE.get_or_init(|| dir.as_ref().join(names::CONFIG_FILE));

    load(FILE.get().unwrap())
}

/// 获取配置文件
pub fn read_config() -> RwLockReadGuard<'static, ConfigObj> {
    CONFIG.get().unwrap().read().unwrap()
}

/// 写配置文件
pub fn write_config() -> RwLockWriteGuard<'static, ConfigObj> {
    CONFIG.get().unwrap().write().unwrap()
}

/// 立即保存配置文件
pub fn save_now() {
    let file = FILE.get().unwrap();
    if let Err(err) = serialize_tools::json_to_file(&*CONFIG.get().unwrap().read().unwrap(), file) {
        mcml_log::error_type(err);
    }
}

/// 保存配置文件
pub fn save() {
    let config = &*CONFIG.get().unwrap().read().unwrap();
    config_save::save(uuids::CONFIG_UUID, config, FILE.get().unwrap());
}

/// 加载配置文件
/// - `file`: 配置文件
pub fn load<P: AsRef<Path>>(file: P) -> bool {
    let config = CONFIG.get_or_init(|| RwLock::new(Default::default()));

    if !file.as_ref().exists() {
        save_now();
        return true;
    }

    let json = serialize_tools::json_from_file::<ConfigObj>(&file);

    if let Err(err) = json {
        mcml_log::error_type(ErrorType::ConfigReadError(FileSystemErrorData {
            error: err.to_string(),
            path: file.as_ref().to_path_buf(),
        }));

        return false;
    }

    let mut config_obj = json.unwrap();
    let version = mcml_names::VERSION.clone();
    if config_obj.version != version {
        config_obj.version = version;

        config_save::save(uuids::CONFIG_UUID, &config_obj, FILE.get().unwrap());
    }
    let mut guard = config.write().unwrap();
    *guard = config_obj;

    false
}
