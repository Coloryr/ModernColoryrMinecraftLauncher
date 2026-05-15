pub mod config_obj;
pub mod config_save;

use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};

use mcml_log;
use mcml_names::{
    i18_items::error_type::{ConfigErrorData, ErrorType},
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
                mcml_log::error_type(ErrorType::ConfigSaveError(ConfigErrorData {
                    error: err.to_string(),
                    file: file.display().to_string(),
                }));
            }
        }
        Err(err) => {
            mcml_log::error_type(ErrorType::ConfigSaveError(ConfigErrorData {
                error: err.to_string(),
                file: file.display().to_string(),
            }));
        }
    }
}

pub fn save() {
    let config = &*CONFIG.get().unwrap().read().unwrap();
    config_save::save(uuids::CONFIG_UUID, config, FILE.get().unwrap());
}

pub fn load(file: &PathBuf) -> bool {
    let config = CONFIG.get_or_init(|| RwLock::new(ConfigObj::default()));

    if !Path::exists(file) {
        save_now();
        return true;
    }

    let stream = File::open(file);
    if let Err(err) = stream {
        mcml_log::error_type(ErrorType::ConfigReadError(ConfigErrorData {
            error: err.to_string(),
            file: file.display().to_string(),
        }));

        return false;
    }
    let stream = stream.unwrap();

    let json = serde_json::from_reader::<_, ConfigObj>(stream);

    if let Err(err) = json {
        mcml_log::error_type(ErrorType::ConfigReadError(ConfigErrorData {
            error: err.to_string(),
            file: file.display().to_string(),
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

pub fn init(local: &PathBuf) -> bool {
    FILE.get_or_init(|| local.join(names::NAME_CONFIG_FILE));

    load(FILE.get().unwrap())
}
