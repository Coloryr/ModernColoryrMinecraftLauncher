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

pub static CONFIG: RwLock<OnceLock<ConfigObj>> = RwLock::new(OnceLock::new());

static FILE: OnceLock<PathBuf> = OnceLock::new();

/// 立即保存配置文件
pub fn save_now() {
    let file = FILE.get().unwrap();

    let stream = File::create(file);

    match stream {
        Ok(stream) => {
            let res = serde_json::to_writer(stream, &CONFIG.read().unwrap().get());
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
    config_save::save(
        uuids::CONFIG_UUID,
        CONFIG.read().unwrap().get().unwrap(),
        FILE.get().unwrap(),
    );
}

pub fn load(file: &PathBuf) -> bool {
    if !Path::exists(file) {
        CONFIG.write().unwrap().get_or_init(|| ConfigObj::default());

        save_now();
        return true;
    }

    let stream = File::open(file);
    if let Err(err) = stream {
        mcml_log::error_type(ErrorType::ConfigReadError(ConfigErrorData {
            error: err.to_string(),
            file: file.display().to_string(),
        }));

        CONFIG.write().unwrap().get_or_init(|| ConfigObj::default());
        return false;
    }
    let stream = stream.unwrap();

    let json = serde_json::from_reader::<_, ConfigObj>(stream);

    if let Err(err) = json {
        mcml_log::error_type(ErrorType::ConfigReadError(ConfigErrorData {
            error: err.to_string(),
            file: file.display().to_string(),
        }));

        CONFIG.write().unwrap().get_or_init(|| ConfigObj::default());
        return false;
    }

    CONFIG.write().unwrap().get_or_init(|| json.unwrap());

    let mut binding = CONFIG.write().unwrap();
    let config = binding.get_mut().unwrap();
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
