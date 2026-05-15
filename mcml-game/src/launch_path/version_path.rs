use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, OnceLock, RwLock},
};

use mcml_base::path_helper;
use mcml_config::{config_obj::SourceLocal, config_save};
use mcml_names::{
    i18_items::error_type::{ErrorType, JsonErrorData},
    names, uuids,
};
use mcml_net::url_helper;
use uuid::Uuid;

use crate::{
    launcher::custom_loader_obj::CustomLoaderType,
    loader::{
        LoaderKey, fabric_loader_obj::FabricLoaderObj, forge_install_obj::ForgeInstallObj,
        forge_launch_obj::ForgeLaunchObj, optifine_obj::OptifineObj,
        quilt_loader_obj::QuiltLoaderObj,
    },
    mojang::{
        game_arg_obj::GameArgObj,
        mojang_api, version_checker,
        version_obj::{VersionObj, VersionsObj},
    },
};

static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

static VERSION: OnceLock<RwLock<VersionObj>> = OnceLock::new();

static OPTIFINE_FILE: OnceLock<PathBuf> = OnceLock::new();
static LITELOADER_FILE: OnceLock<PathBuf> = OnceLock::new();

static FORGE_DIR: OnceLock<PathBuf> = OnceLock::new();
static FABRIC_DIR: OnceLock<PathBuf> = OnceLock::new();
static QUILT_DIR: OnceLock<PathBuf> = OnceLock::new();
static NEOFORGE_DIR: OnceLock<PathBuf> = OnceLock::new();

static OPTIFINE_LOADER: OnceLock<RwLock<HashMap<String, Arc<OptifineObj>>>> = OnceLock::new();
static GAME_ARGS: OnceLock<RwLock<HashMap<String, Arc<GameArgObj>>>> = OnceLock::new();
static FORGE_INSTALLS: OnceLock<RwLock<HashMap<LoaderKey, Arc<ForgeInstallObj>>>> = OnceLock::new();
static NEOFORGE_INSTALLS: OnceLock<RwLock<HashMap<LoaderKey, Arc<ForgeInstallObj>>>> =
    OnceLock::new();
static FORGE_LAUNCHS: OnceLock<RwLock<HashMap<LoaderKey, Arc<ForgeLaunchObj>>>> = OnceLock::new();
static NEOFORGE_LAUNCHS: OnceLock<RwLock<HashMap<LoaderKey, Arc<ForgeLaunchObj>>>> =
    OnceLock::new();
static FABRIC_LOADERS: OnceLock<RwLock<HashMap<LoaderKey, Arc<FabricLoaderObj>>>> = OnceLock::new();
static QUILT_LOADERS: OnceLock<RwLock<HashMap<LoaderKey, Arc<QuiltLoaderObj>>>> = OnceLock::new();
static CUSTOM_LOADERS: OnceLock<RwLock<HashMap<Uuid, Arc<CustomLoaderType>>>> = OnceLock::new();

pub fn get_version(version: &String) -> Option<Arc<GameArgObj>> {
    let list = GAME_ARGS.get().unwrap().read().unwrap();
    match list.get(version) {
        None => None,
        Some(data) => Some(data.clone()),
    }
}

pub fn init(dir: &PathBuf) {
    let dir = BASE_DIR.get_or_init(|| dir.join(names::NAME_VERSION_DIR));

    FORGE_DIR.set(dir.join(names::NAME_FORGE_KEY)).unwrap();
    FABRIC_DIR.set(dir.join(names::NAME_FABRIC_KEY)).unwrap();
    QUILT_DIR.set(dir.join(names::NAME_QUILT_KEY)).unwrap();
    NEOFORGE_DIR
        .set(dir.join(names::NAME_NEOFORGED_KEY))
        .unwrap();

    OPTIFINE_FILE
        .set(dir.join(names::NAME_OPTIFINE_FILE))
        .unwrap();
    LITELOADER_FILE
        .set(dir.join(names::NAME_LITELOADER_FILE))
        .unwrap();

    OPTIFINE_LOADER.set(RwLock::new(HashMap::new())).unwrap();

    GAME_ARGS.set(RwLock::new(HashMap::new())).unwrap();
    FORGE_INSTALLS.set(RwLock::new(HashMap::new())).unwrap();
    NEOFORGE_INSTALLS.set(RwLock::new(HashMap::new())).unwrap();
    FORGE_LAUNCHS.set(RwLock::new(HashMap::new())).unwrap();
    NEOFORGE_LAUNCHS.set(RwLock::new(HashMap::new())).unwrap();
    FABRIC_LOADERS.set(RwLock::new(HashMap::new())).unwrap();
    QUILT_LOADERS.set(RwLock::new(HashMap::new())).unwrap();
    CUSTOM_LOADERS.set(RwLock::new(HashMap::new())).unwrap();

    let dir = dir.as_path();
    if !dir.is_dir() {
        fs::create_dir(dir).unwrap();
    }

    let dir = FORGE_DIR.get().unwrap();
    if !dir.is_dir() {
        fs::create_dir(dir).unwrap();
    }

    let dir = FABRIC_DIR.get().unwrap();
    if !dir.is_dir() {
        fs::create_dir(dir).unwrap();
    }

    let dir = QUILT_DIR.get().unwrap();
    if !dir.is_dir() {
        fs::create_dir(dir).unwrap();
    }

    let dir = NEOFORGE_DIR.get().unwrap();
    if !dir.is_dir() {
        fs::create_dir(dir).unwrap();
    }

    load_optifine();
}

fn load_optifine() {
    let file = OPTIFINE_FILE.get().unwrap();
    if !file.exists() {
        return;
    }

    let file = path_helper::open_read(file);
    if file.is_none() {
        return;
    }
    let file = file.unwrap();
    let json = serde_json::from_reader::<_, HashMap<String, OptifineObj>>(file);

    match json {
        Err(err) => {
            mcml_log::error_type(ErrorType::JsonError(JsonErrorData {
                error: err.to_string(),
            }));
        }
        Ok(data) => {
            let mut list = OPTIFINE_LOADER.get().unwrap().write().unwrap();
            list.clear();

            list.extend(data.into_iter().map(|(k, v)| (k, Arc::new(v))));
        }
    };
}

fn save_optifine() {
    let file = OPTIFINE_FILE.get().unwrap();
    let list = OPTIFINE_LOADER.get().unwrap().read().unwrap();
    let value = &*list;

    let ref_map: HashMap<&String, &OptifineObj> =
        value.iter().map(|(k, v)| (k, v.as_ref())).collect();

    config_save::save(uuids::OPTIFINE_UUID, &ref_map, file);
}

fn save_versions(data: &Vec<u8>) {
    let file = BASE_DIR.get().unwrap().join(names::NAME_VERSION_FILE);
    path_helper::write_bytes(&file, data).unwrap();
}

/// 从在线获取版本信息
async fn get_version_from_online() {
    let res = mojang_api::get_versions(None).await;
    if res.is_ok() {
        let data: Vec<u8> = res.unwrap();
        let json = serde_json::from_slice::<VersionObj>(&data);
        if json.is_ok() {
            match VERSION.get() {
                None => {
                    VERSION.set(RwLock::new(json.unwrap())).unwrap();
                }
                Some(version) => {
                    *version.write().unwrap() = json.unwrap();
                }
            };

            save_versions(&data);

            return;
        }
    }

    // 获取失败再从官方源获取一次
    let res = mojang_api::get_versions(Some(SourceLocal::Offical)).await;
    if res.is_ok() {
        let data: Vec<u8> = res.unwrap();
        let json = serde_json::from_slice::<VersionObj>(&data);
        if json.is_ok() {
            match VERSION.get() {
                None => {
                    VERSION.set(RwLock::new(json.unwrap())).unwrap();
                }
                Some(version) => {
                    *version.write().unwrap() = json.unwrap();
                }
            };

            save_versions(&data);

            return;
        }
    }

    mcml_log::error_type(ErrorType::GetVersionMetaFail);
}

/// 读取版本信息
async fn read_version() {
    let file = BASE_DIR.get().unwrap().join(names::NAME_VERSION_FILE);
    if file.exists() {
        let file = path_helper::open_read(&file);
        if file.is_some() {
            let file = file.unwrap();
            let json = serde_json::from_reader::<_, VersionObj>(file);
            if json.is_ok() {
                match VERSION.get() {
                    None => {
                        VERSION.set(RwLock::new(json.unwrap())).unwrap();
                    }
                    Some(version) => {
                        *version.write().unwrap() = json.unwrap();
                    }
                };
                return;
            }
        }
    }

    get_version_from_online().await;
}

/// 是否存在版本信息
pub async fn is_have_version_info() -> bool {
    if VERSION.get().is_none() {
        read_version().await;
    };

    VERSION.get().is_some()
}

/// 添加版本信息
/// - `obj`: 游戏数据
pub async fn add_game(obj: &VersionsObj) -> Option<Arc<GameArgObj>> {
    let mut url = obj.url.clone();
    url_helper::change_source(&mut url);

    let data = mojang_api::get_assets(&url).await;
    match data {
        Err(err) => {
            mcml_log::error_type(err);
            None
        }
        Ok(data) => {
            let json = serde_json::from_slice::<GameArgObj>(&data);
            match json {
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(JsonErrorData {
                        error: err.to_string(),
                    }));
                    None
                }
                Ok(json) => {
                    let file = BASE_DIR.get().unwrap().join(format!("{}.json", obj.id));
                    path_helper::write_bytes(&file, &data).unwrap();

                    let mut list = GAME_ARGS.get().unwrap().write().unwrap();

                    list.insert(obj.id.clone(), Arc::new(json));
                    let json = list.get(&obj.id).unwrap();
                    Some(json.clone())
                }
            }
        }
    }
}

/// 保存Fabric-Loader信息
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub async fn add_fabric(obj: FabricLoaderObj, data: &Vec<u8>, mc: &String, version: &String) {
    let file = FABRIC_DIR.get().unwrap().join(format!("{}.json", obj.id));
    path_helper::write_bytes(&file, &data).unwrap();

    let key = LoaderKey::new(mc.clone(), version.clone());
    let mut list = FABRIC_LOADERS.get().unwrap().write().unwrap();
    list.insert(key.clone(), Arc::new(obj));
}

/// 添加Forge启动信息
/// - `obj`: 信息
pub async fn add_forge(
    obj: ForgeLaunchObj,
    data: &Vec<u8>,
    mc: &String,
    version: &String,
    neo: bool,
) {
    let v222 = version_checker::is_game_version_1202(&mc);
    let name = if neo && v222 {
        format!("{}-{}", names::NAME_NEOFORGE_KEY, version)
    } else {
        format!("{}-{}-{}", names::NAME_FORGE_KEY, mc, version)
    };
    let file = if neo {
        NEOFORGE_DIR.get().unwrap().join(format!("{}.json", name))
    } else {
        FORGE_DIR.get().unwrap().join(format!("{}.json", name))
    };
    path_helper::write_bytes(&file, &data).unwrap();

    let key = LoaderKey::new(mc.clone(), version.clone());
    let mut list = if neo {
        NEOFORGE_LAUNCHS.get().unwrap().write().unwrap()
    } else {
        FORGE_LAUNCHS.get().unwrap().write().unwrap()
    };
    list.insert(key, Arc::new(obj));
}

/// 添加Forge安装信息
/// - `obj`: 信息
/// - `data`: 文本
pub async fn add_forge_install(
    obj: ForgeInstallObj,
    data: &Vec<u8>,
    mc: &String,
    version: &String,
    neo: bool,
) {
    let v222 = version_checker::is_game_version_1202(&mc);
    let name = if neo && v222 {
        format!(
            "{}-{}-{}",
            names::NAME_NEOFORGE_KEY,
            version,
            names::NAME_FORGE_INSTALL
        )
    } else {
        format!(
            "{}-{}-{}-{}",
            names::NAME_FORGE_KEY,
            mc,
            version,
            names::NAME_FORGE_INSTALL
        )
    };
    let file = if neo {
        NEOFORGE_DIR.get().unwrap().join(format!("{}.json", name))
    } else {
        FORGE_DIR.get().unwrap().join(format!("{}.json", name))
    };
    path_helper::write_bytes(&file, &data).unwrap();

    let key = LoaderKey::new(mc.clone(), version.clone());
    let mut list = if neo {
        NEOFORGE_INSTALLS.get().unwrap().write().unwrap()
    } else {
        FORGE_INSTALLS.get().unwrap().write().unwrap()
    };
    list.insert(key, Arc::new(obj));
}
