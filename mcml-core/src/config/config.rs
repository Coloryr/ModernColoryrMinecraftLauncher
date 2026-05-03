use std::{
    fs::File,
    path::{Path, PathBuf},
    sync::{OnceLock, RwLock},
};

use mcml_log::log;

use crate::{
    config::{config_obj::ConfigObj, config_save},
    core, names,
};

pub static CONFIG: RwLock<OnceLock<ConfigObj>> = RwLock::new(OnceLock::new());

static FILE: OnceLock<PathBuf> = OnceLock::new();

pub fn save_now() {
    let file = FILE.get().unwrap();
    log::info(format!("Save config: {}", file.display()));

    let file = File::create(file);
    if file.is_ok() {
        let file = file.unwrap();
        let res = serde_json::to_writer(file, &CONFIG.read().unwrap().get());
        if let Err(err) = res {
            log::error(format!("Config save error: {}", err));
        }
    }
}

pub fn save() {
    config_save::save(
        String::from("config"),
        CONFIG.read().unwrap().get().unwrap(),
        FILE.get().unwrap(),
    );
}

pub fn load(file: &PathBuf) {
    log::info(format!("Load config: {}", file.display()));

    if !Path::exists(file) {
        *core::NEW_START.write().unwrap() = true;

        CONFIG.write().unwrap().get_or_init(|| ConfigObj::default());

        log::info(format!("Create new config"));

        save_now();
        return;
    }

    let file = File::open(file);
    if let Err(err) = file {
        log::error(format!("Config load error: {}", err));

        CONFIG.write().unwrap().get_or_init(|| ConfigObj::default());
        return;
    }
    let file = file.unwrap();

    let json = serde_json::from_reader::<_, ConfigObj>(file);

    if let Err(err) = json {
        log::error(format!("Json read error: {}", err));

        CONFIG.write().unwrap().get_or_init(|| ConfigObj::default());
        return;
    }

    CONFIG.write().unwrap().get_or_init(|| json.unwrap());

    let mut binding = CONFIG.write().unwrap();
    let config = binding.get_mut().unwrap();
    let version = String::from(core::VERSION);
    if config.version != version {
        config.version = version;

        log::info(format!("Upgrade config"));

        save();
    }
}

pub fn init(local: PathBuf) {
    FILE.get_or_init(|| local.join(names::NAME_CONFIG_FILE));

    load(FILE.get().unwrap());
}
