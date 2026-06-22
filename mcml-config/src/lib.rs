pub mod config_obj;
pub mod config_save;

use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};

use mcml_log;
use mcml_names::{
    i18_items::error_type::{ErrorType, FileSystemErrorData},
    names, uuids,
};

use crate::config_obj::ConfigObj;

pub static CONFIG: OnceLock<RwLock<ConfigObj>> = OnceLock::new();

static FILE: OnceLock<PathBuf> = OnceLock::new();

/// 立即保存配置文件
pub fn save_now() {
    let file = FILE.get().unwrap();

    let stream = File::create(file);

    match stream {
        Ok(stream) => {
            let res = serde_json::to_writer(stream, &*CONFIG.get().unwrap().read().unwrap());
            if let Err(err) = res {
                mcml_log::error_type(ErrorType::ConfigSaveError(FileSystemErrorData {
                    error: err.to_string(),
                    path: file.clone(),
                }));
            }
        }
        Err(err) => {
            mcml_log::error_type(ErrorType::ConfigSaveError(FileSystemErrorData {
                error: err.to_string(),
                path: file.clone(),
            }));
        }
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
    let config = CONFIG.get_or_init(|| RwLock::new(ConfigObj::default()));

    let path = file.as_ref();

    if !Path::exists(&path) {
        save_now();
        return true;
    }

    let stream = File::open(&path);
    if let Err(err) = stream {
        mcml_log::error_type(ErrorType::ConfigReadError(FileSystemErrorData {
            error: err.to_string(),
            path: path.to_path_buf(),
        }));

        return false;
    }
    let stream = stream.unwrap();

    let json = serde_json::from_reader::<_, ConfigObj>(stream);

    if let Err(err) = json {
        mcml_log::error_type(ErrorType::ConfigReadError(FileSystemErrorData {
            error: err.to_string(),
            path: path.to_path_buf(),
        }));

        return false;
    }

    let mut config = config.write().unwrap();
    *config = json.unwrap();
    let version = String::from(mcml_names::VERSION);
    if config.version != version {
        config.version = version;

        save();
    }

    false
}

/// 初始化运行路径
/// - `dir`: 运行路径
pub fn init<P: AsRef<Path>>(dir: P) -> bool {
    FILE.get_or_init(|| dir.as_ref().join(names::CONFIG_FILE));

    load(FILE.get().unwrap())
}
