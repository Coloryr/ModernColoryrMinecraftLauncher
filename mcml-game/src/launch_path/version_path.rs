use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, OnceLock, RwLock},
};

use mcml_base::{hash_helper, path_helper};
use mcml_config::{config_obj::SourceLocal, config_save};
use mcml_names::{
    i18_items::error_type::{ErrorType, JsonErrorData},
    names, uuids,
};
use mcml_net::url_helper;
use uuid::Uuid;

use crate::{
    launcher::{LoaderType, custom_loader_obj::CustomLoaderType, game_setting_obj::GameSettingObj},
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

static VERSION: OnceLock<RwLock<Option<Arc<VersionObj>>>> = OnceLock::new();

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

/// 初始化版本路径
/// - `dir`: 运行路径
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
    VERSION.set(RwLock::new(None)).unwrap();

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

/// 加载高清修复版本信息
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

/// 保存高清修复版本信息
fn save_optifine() {
    let file = OPTIFINE_FILE.get().unwrap();
    let list = OPTIFINE_LOADER.get().unwrap().read().unwrap();
    let value = &*list;

    let ref_map: HashMap<&String, &OptifineObj> =
        value.iter().map(|(k, v)| (k, v.as_ref())).collect();

    config_save::save(uuids::OPTIFINE_UUID, &ref_map, file);
}

/// 从在线获取版本信息
async fn get_version_from_online() {
    fn save_versions(data: &Vec<u8>) {
        let file = BASE_DIR.get().unwrap().join(names::NAME_VERSION_FILE);
        path_helper::write_bytes(&file, data).unwrap();
    }

    let res = mojang_api::get_versions(None).await;
    if res.is_ok() {
        let data: Vec<u8> = res.unwrap();
        let json = serde_json::from_slice::<VersionObj>(&data);
        if json.is_ok() {
            *VERSION.get().unwrap().write().unwrap() = Some(Arc::new(json.unwrap()));
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
            *VERSION.get().unwrap().write().unwrap() = Some(Arc::new(json.unwrap()));
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
                *VERSION.get().unwrap().write().unwrap() = Some(Arc::new(json.unwrap()));
                return;
            }
        }
    }

    get_version_from_online().await;
}

/// 获取游戏版本列表
pub async fn get_version_obj() -> Option<Arc<VersionObj>> {
    if VERSION.get().is_none() {
        read_version().await;
    }

    let temp = VERSION.get().unwrap().read().unwrap();

    if temp.is_none() {
        None
    } else {
        let temp = temp.clone().unwrap();
        Some(temp.clone())
    }
}

/// 是否存在版本信息
pub async fn is_have_version_info() -> bool {
    get_version_obj().await.is_some()
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
pub fn add_fabric(obj: FabricLoaderObj, data: &Vec<u8>, mc: &String, version: &String) {
    let file = FABRIC_DIR.get().unwrap().join(format!("{}.json", obj.id));
    path_helper::write_bytes(&file, &data).unwrap();

    let key = LoaderKey::new(mc.clone(), version.clone());
    let mut list = FABRIC_LOADERS.get().unwrap().write().unwrap();
    list.insert(key.clone(), Arc::new(obj));
}

/// 添加Forge启动信息
/// - `obj`: 信息
pub fn add_forge(obj: ForgeLaunchObj, data: &Vec<u8>, mc: &String, version: &String, neo: bool) {
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
pub fn add_forge_install(
    obj: ForgeInstallObj,
    data: &Vec<u8>,
    mc: &String,
    version: &String,
    neo: bool,
) {
    let name = get_forge_json_name(mc, version, true, true);
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

/// 添加Quilt信息
/// - `obj`: Quilt加载器数据
/// - `data`: 文本
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub fn add_quilt_loader(obj: QuiltLoaderObj, data: &Vec<u8>, mc: &String, version: &String) {
    let file = QUILT_DIR.get().unwrap().join(format!("{}.json", obj.id));
    path_helper::write_bytes(&file, &data).unwrap();

    let key = LoaderKey::new(mc.clone(), version.clone());
    let mut list = QUILT_LOADERS.get().unwrap().write().unwrap();
    list.insert(key.clone(), Arc::new(obj));
}

/// 添加自定义加载器信息
/// - `obj`: 自定义加载器
/// - `uuid`: 游戏实例
pub fn add_custom_loader(obj: CustomLoaderType, uuid: Uuid) {
    let mut list = CUSTOM_LOADERS.get().unwrap().write().unwrap();
    list.insert(uuid, Arc::new(obj));
}

/// 添加高清修复信息
/// - `obj`: 高清修复信息
pub fn add_optifine(obj: OptifineObj) {
    let mut list = OPTIFINE_LOADER.get().unwrap().write().unwrap();
    list.insert(obj.version.clone(), Arc::new(obj));

    save_optifine();
}

/// 获取版本信息
/// - `version`: 游戏版本
pub fn get_version(version: &String) -> Option<Arc<GameArgObj>> {
    let list = GAME_ARGS.get().unwrap().read().unwrap();
    let data = list.get(version);

    match data {
        None => {
            let file = BASE_DIR.get().unwrap().join(format!("{}.json", version));
            let file = path_helper::open_read(&file)?;
            let json = serde_json::from_reader::<_, GameArgObj>(file);
            match json {
                Ok(json) => {
                    let mut list = GAME_ARGS.get().unwrap().write().unwrap();
                    let data = Arc::new(json);
                    let data1 = data.clone();
                    list.insert(version.clone(), data);

                    Some(data1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(JsonErrorData {
                        error: err.to_string(),
                    }));

                    None
                }
            }
        }
        Some(data) => Some(data.clone()),
    }
}

/// 检查游戏版本更新
/// - `version`: 游戏版本
pub async fn check_update(version: &String) -> Option<Arc<GameArgObj>> {
    get_version_from_online().await;

    match get_version_obj().await {
        None => None,
        Some(obj) => {
            let item = obj
                .versions
                .iter()
                .filter(|&item| item.id.eq_ignore_ascii_case(version))
                .next();

            match item {
                None => None,
                Some(item) => {
                    let file = BASE_DIR.get().unwrap().join(format!("{}.json", version));
                    let mut file = path_helper::open_read(&file)?;

                    let sha1 = hash_helper::gen_sha1_from_reader(&mut file).unwrap();

                    if sha1 != item.sha1 {
                        add_game(item).await
                    } else {
                        get_version(version)
                    }
                }
            }
        }
    }
}

/// 获取json名字
pub fn get_forge_json_name(mc: &String, version: &String, neo: bool, install: bool) -> String {
    if neo {
        let v222 = version_checker::is_game_version_1202(&mc);

        if install {
            if v222 {
                format!(
                    "{}-{}-{}{}",
                    names::NAME_NEOFORGE_KEY,
                    version,
                    names::NAME_FORGE_INSTALL,
                    names::NAME_JSON_EXT
                )
            } else {
                format!(
                    "{}-{}-{}-{}{}",
                    names::NAME_FORGE_KEY,
                    mc,
                    version,
                    names::NAME_FORGE_INSTALL,
                    names::NAME_JSON_EXT
                )
            }
        } else {
            if v222 {
                format!(
                    "{}-{}{}",
                    names::NAME_NEOFORGE_KEY,
                    version,
                    names::NAME_JSON_EXT
                )
            } else {
                format!(
                    "{}-{}-{}{}",
                    names::NAME_FORGE_KEY,
                    mc,
                    version,
                    names::NAME_JSON_EXT
                )
            }
        }
    } else {
        if install {
            format!(
                "{}-{}-{}{}",
                names::NAME_FORGE_KEY,
                version,
                names::NAME_FORGE_INSTALL,
                names::NAME_JSON_EXT
            )
        } else {
            format!(
                "{}-{}{}",
                names::NAME_FORGE_KEY,
                version,
                names::NAME_JSON_EXT
            )
        }
    }
}

/// 获取NeoForge安装数据
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub fn get_neoforge_install_obj(mc: &String, version: &String) -> Option<Arc<ForgeInstallObj>> {
    let key = LoaderKey::new(mc.clone(), version.clone());

    let list = NEOFORGE_INSTALLS.get().unwrap().read().unwrap();
    let item = list.get(&key);
    match item {
        Some(item) => Some(item.clone()),
        None => {
            let name = NEOFORGE_DIR
                .get()
                .unwrap()
                .join(get_forge_json_name(mc, version, true, true));
            let file = path_helper::open_read(&name)?;
            let json = serde_json::from_reader::<_, ForgeInstallObj>(&file);

            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = NEOFORGE_INSTALLS.get().unwrap().write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(JsonErrorData {
                        error: err.to_string(),
                    }));

                    None
                }
            }
        }
    }
}

/// 获取NeoForge启动数据
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub fn get_neoforge(mc: &String, version: &String) -> Option<Arc<ForgeLaunchObj>> {
    let key = LoaderKey::new(mc.clone(), version.clone());

    let list = NEOFORGE_LAUNCHS.get().unwrap().read().unwrap();
    let item = list.get(&key);
    match item {
        Some(item) => Some(item.clone()),
        None => {
            let name = NEOFORGE_DIR
                .get()
                .unwrap()
                .join(get_forge_json_name(mc, version, true, false));
            let file = path_helper::open_read(&name)?;
            let json = serde_json::from_reader::<_, ForgeLaunchObj>(&file);

            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = NEOFORGE_LAUNCHS.get().unwrap().write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(JsonErrorData {
                        error: err.to_string(),
                    }));

                    None
                }
            }
        }
    }
}

/// 获取Forge安装数据
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub fn get_forge_install_obj(mc: &String, version: &String) -> Option<Arc<ForgeInstallObj>> {
    let key = LoaderKey::new(mc.clone(), version.clone());

    let list = FORGE_INSTALLS.get().unwrap().read().unwrap();
    let item = list.get(&key);
    match item {
        Some(item) => Some(item.clone()),
        None => {
            let name = FORGE_DIR
                .get()
                .unwrap()
                .join(get_forge_json_name(mc, version, false, true));
            let file = path_helper::open_read(&name)?;
            let json = serde_json::from_reader::<_, ForgeInstallObj>(&file);

            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = FORGE_INSTALLS.get().unwrap().write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(JsonErrorData {
                        error: err.to_string(),
                    }));

                    None
                }
            }
        }
    }
}

/// 获取Forge启动数据
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub fn get_forge(mc: &String, version: &String) -> Option<Arc<ForgeLaunchObj>> {
    let key = LoaderKey::new(mc.clone(), version.clone());

    let list = FORGE_LAUNCHS.get().unwrap().read().unwrap();
    let item = list.get(&key);
    match item {
        Some(item) => Some(item.clone()),
        None => {
            let name = FORGE_DIR
                .get()
                .unwrap()
                .join(get_forge_json_name(mc, version, false, false));

            let file = path_helper::open_read(&name)?;
            let json = serde_json::from_reader::<_, ForgeLaunchObj>(&file);

            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = FORGE_LAUNCHS.get().unwrap().write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(JsonErrorData {
                        error: err.to_string(),
                    }));

                    None
                }
            }
        }
    }
}

/// 获取Fabric加载器数据
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub fn get_fabric(mc: &String, version: &String) -> Option<Arc<FabricLoaderObj>> {
    let key = LoaderKey::new(mc.clone(), version.clone());
    let list = FABRIC_LOADERS.get().unwrap().read().unwrap();
    match list.get(&key) {
        None => {
            let file = FABRIC_DIR.get().unwrap().join(format!(
                "{}-{}-{}{}",
                names::NAME_FABRIC_LOADER_KEY,
                version,
                mc,
                names::NAME_JSON_EXT
            ));
            let file = path_helper::open_read(&file)?;
            let json = serde_json::from_reader::<_, FabricLoaderObj>(&file);
            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = FABRIC_LOADERS.get().unwrap().write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(JsonErrorData {
                        error: err.to_string(),
                    }));

                    None
                }
            }
        }
        Some(data) => Some(data.clone()),
    }
}

/// 获取Quilt加载器数据
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub fn get_quilt(mc: &String, version: &String) -> Option<Arc<QuiltLoaderObj>> {
    let key = LoaderKey::new(mc.clone(), version.clone());
    let list = QUILT_LOADERS.get().unwrap().read().unwrap();
    match list.get(&key) {
        None => {
            let file = FABRIC_DIR.get().unwrap().join(format!(
                "{}-{}-{}{}",
                names::NAME_FABRIC_LOADER_KEY,
                version,
                mc,
                names::NAME_JSON_EXT
            ));
            let file = path_helper::open_read(&file)?;
            let json = serde_json::from_reader::<_, QuiltLoaderObj>(&file);
            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = QUILT_LOADERS.get().unwrap().write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(JsonErrorData {
                        error: err.to_string(),
                    }));

                    None
                }
            }
        }
        Some(data) => Some(data.clone()),
    }
}

/// 获取高清修复信息
/// - `version`: 版本号
pub fn get_optifine(version: &String) -> Option<Arc<OptifineObj>> {
    let list = OPTIFINE_LOADER.get().unwrap().read().unwrap();
    Some(list.get(version)?.clone())
}

impl GameSettingObj {
    /// 获取neoforge加载器信息
    pub fn get_neoforge(&self) -> Option<Arc<ForgeLaunchObj>> {
        match &self.loader_version {
            None => None,
            Some(data) => get_neoforge(&self.version, &data),
        }
    }

    /// 获取Fabric加载器数据
    pub fn get_fabric(&self) -> Option<Arc<FabricLoaderObj>> {
        match &self.loader_version {
            None => None,
            Some(data) => get_fabric(&self.version, &data),
        }
    }

    /// 获取Quilt加载器数据
    pub fn get_quilt(&self) -> Option<Arc<QuiltLoaderObj>> {
        match &self.loader_version {
            None => None,
            Some(data) => get_quilt(&self.version, &data),
        }
    }

    /// 获取自定义加载器数据
    pub fn get_custom_loader(&self) -> Option<Arc<CustomLoaderType>> {
        let list = CUSTOM_LOADERS.get().unwrap().read().unwrap();
        Some(list.get(&self.uuid)?.clone())
    }

    /// 获取高清修复信息
    pub fn get_optifine(&self) -> Option<Arc<OptifineObj>> {
        if self.loader != LoaderType::OptiFine || self.loader_version.is_none() {
            None
        } else {
            get_optifine(&self.loader_version.clone().unwrap())
        }
    }
}
