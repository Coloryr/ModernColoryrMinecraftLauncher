use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, OnceLock, RwLock},
};

use mcml_base::{
    hash_helper::{self, HashType},
    path_helper,
};
use mcml_config::{config_obj::SourceLocal, config_save};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType},
    names, uuids,
};
use mcml_net::{mojang_api, url_helper};
use tokio::task;
use uuid::Uuid;

use crate::{
    launcher::{custom_loader_obj::CustomLoaderType, game_setting_obj::InstanceSettingObj},
    loader::{
        LoaderKey, LoaderType, fabric_loader_obj::FabricLoaderObj,
        forge_install_obj::ForgeInstallObj, forge_launch_obj::ForgeLaunchObj,
        optifine_obj::OptifineObj, quilt_loader_obj::QuiltLoaderObj,
    },
    mojang::{
        game_arg_obj::GameArgObj,
        version_checker,
        version_obj::{VersionObj, VersionsObj},
    },
};

static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();
static FORGE_DIR: OnceLock<PathBuf> = OnceLock::new();
static FABRIC_DIR: OnceLock<PathBuf> = OnceLock::new();
static QUILT_DIR: OnceLock<PathBuf> = OnceLock::new();
static NEOFORGE_DIR: OnceLock<PathBuf> = OnceLock::new();

static OPTIFINE_FILE: OnceLock<PathBuf> = OnceLock::new();
static LITELOADER_FILE: OnceLock<PathBuf> = OnceLock::new();

static VERSION: LazyLock<RwLock<Option<Arc<VersionObj>>>> = LazyLock::new(|| RwLock::new(None));

static OPTIFINE_LOADER: LazyLock<RwLock<HashMap<String, Arc<OptifineObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static GAME_ARGS: LazyLock<RwLock<HashMap<String, Arc<GameArgObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static FORGE_INSTALLS: LazyLock<RwLock<HashMap<LoaderKey, Arc<ForgeInstallObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static NEOFORGE_INSTALLS: LazyLock<RwLock<HashMap<LoaderKey, Arc<ForgeInstallObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static FORGE_LAUNCHS: LazyLock<RwLock<HashMap<LoaderKey, Arc<ForgeLaunchObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static NEOFORGE_LAUNCHS: LazyLock<RwLock<HashMap<LoaderKey, Arc<ForgeLaunchObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static FABRIC_LOADERS: LazyLock<RwLock<HashMap<LoaderKey, Arc<FabricLoaderObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static QUILT_LOADERS: LazyLock<RwLock<HashMap<LoaderKey, Arc<QuiltLoaderObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static CUSTOM_LOADERS: LazyLock<RwLock<HashMap<Uuid, Arc<CustomLoaderType>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 初始化版本路径
/// - `dir`: 运行路径
pub(crate) fn init<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    let dir = BASE_DIR.get_or_init(|| dir.as_ref().join(names::VERSION_DIR));

    OPTIFINE_FILE.set(dir.join(names::OPTIFINE_FILE)).unwrap();
    LITELOADER_FILE
        .set(dir.join(names::LITELOADER_FILE))
        .unwrap();

    let dir = dir.as_path();
    if !dir.is_dir() {
        path_helper::create_dir_all(dir)?;
    }

    let forge = FORGE_DIR.get_or_init(|| dir.join(names::FORGE_KEY));
    if !forge.is_dir() {
        path_helper::create_dir_all(forge)?;
    }

    let fabric = FABRIC_DIR.get_or_init(|| dir.join(names::FABRIC_KEY));
    if !fabric.is_dir() {
        path_helper::create_dir_all(fabric)?;
    }

    let quilt = QUILT_DIR.get_or_init(|| dir.join(names::QUILT_KEY));
    if !quilt.is_dir() {
        path_helper::create_dir_all(quilt)?;
    }

    let neoforge = NEOFORGE_DIR.get_or_init(|| dir.join(names::NEOFORGED_KEY));
    if !neoforge.is_dir() {
        path_helper::create_dir_all(neoforge)?;
    }

    load_optifine();

    Ok(())
}

/// 获取目录
pub fn get_version_dir() -> PathBuf {
    BASE_DIR.get().unwrap().clone()
}

/// 加载高清修复版本信息
fn load_optifine() {
    let local = OPTIFINE_FILE.get().unwrap();
    if !local.exists() {
        return;
    }

    let file = path_helper::open_read(local);
    if let Err(err) = file {
        mcml_log::error_type(err);

        return;
    }
    let file = file.unwrap();
    let json = serde_json::from_reader::<_, HashMap<String, OptifineObj>>(file);

    match json {
        Err(err) => {
            mcml_log::error_type(ErrorType::JsonError(ErrorData {
                error: err.to_string(),
            }));
        }
        Ok(data) => {
            let mut list = OPTIFINE_LOADER.write().unwrap();
            list.clear();

            list.extend(data.into_iter().map(|(k, v)| (k, Arc::new(v))));
        }
    };
}

/// 保存高清修复版本信息
fn save_optifine() {
    let file = OPTIFINE_FILE.get().unwrap();
    let list = OPTIFINE_LOADER.read().unwrap();
    let value = &*list;

    let ref_map: HashMap<&String, &OptifineObj> =
        value.iter().map(|(k, v)| (k, v.as_ref())).collect();

    config_save::save(uuids::OPTIFINE_UUID, &ref_map, file);
}

/// 从在线获取版本信息
async fn get_version_from_online() -> CoreResult<()> {
    fn save_versions(data: &Vec<u8>) {
        let file = BASE_DIR.get().unwrap().join(names::VERSION_FILE);
        path_helper::write_bytes(&file, data).unwrap();
    }

    let res = mojang_api::get_versions(None).await;
    if res.is_ok() {
        let data: Vec<u8> = res.unwrap();
        let json = serde_json::from_slice::<VersionObj>(&data);
        if json.is_ok() {
            *VERSION.write().unwrap() = Some(Arc::new(json.unwrap()));
            save_versions(&data);

            return Ok(());
        }
    }

    // 获取失败再从官方源获取一次
    let res = mojang_api::get_versions(Some(SourceLocal::Offical)).await;
    if res.is_ok() {
        let data: Vec<u8> = res.unwrap();
        let json = serde_json::from_slice::<VersionObj>(&data);
        if json.is_ok() {
            *VERSION.write().unwrap() = Some(Arc::new(json.unwrap()));
            save_versions(&data);

            return Ok(());
        }
    }

    Err(ErrorType::GetVersionMetaFail)
}

/// 从文件读取版本信息
/// 如果文件不存在就去在线读取
async fn read_version() -> CoreResult<()> {
    let local = BASE_DIR.get().unwrap().join(names::VERSION_FILE);
    if local.exists()
        && local.is_file()
        && let Ok(file) = path_helper::open_read(&local)
        && let Ok(json) = serde_json::from_reader::<_, VersionObj>(file)
    {
        *VERSION.write().unwrap() = Some(Arc::new(json));
        task::spawn(async { get_version_from_online().await });
        return Ok(());
    }

    get_version_from_online().await?;

    Ok(())
}

/// 获取游戏版本列表
/// 不存在就去读取文件
pub async fn get_version_obj() -> CoreResult<Arc<VersionObj>> {
    if let Some(v) = VERSION.read().unwrap().as_ref() {
        return Ok(v.clone());
    }
    read_version().await?;

    Ok(VERSION.read().unwrap().as_ref().unwrap().clone())
}

/// 是否存在版本信息
pub async fn is_have_version_info() -> bool {
    VERSION.read().unwrap().is_some()
}

/// 添加版本信息
/// - `obj`: 游戏数据
pub async fn add_game(obj: &VersionsObj) -> CoreResult<Arc<GameArgObj>> {
    let mut url = obj.url.clone();
    url_helper::change_source(&mut url);

    let data = mojang_api::get_assets(&url).await?;
    let json = serde_json::from_slice::<GameArgObj>(&data).map_err(|err| {
        ErrorType::JsonError(ErrorData {
            error: err.to_string(),
        })
    })?;
    let file = BASE_DIR.get().unwrap().join(format!("{}.json", obj.id));
    path_helper::write_bytes(&file, &data).unwrap();

    let mut list = GAME_ARGS.write().unwrap();

    list.insert(obj.id.clone(), Arc::new(json));
    let json = list.get(&obj.id).unwrap();
    Ok(json.clone())
}

/// 保存Fabric-Loader信息
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub fn add_fabric(
    obj: FabricLoaderObj,
    data: &Vec<u8>,
    mc: &str,
    version: &str,
) -> Arc<FabricLoaderObj> {
    let file = FABRIC_DIR.get().unwrap().join(format!("{}.json", obj.id));
    path_helper::write_bytes(&file, &data).unwrap();

    let key = LoaderKey::new(mc, version);
    let mut list = FABRIC_LOADERS.write().unwrap();
    let info = Arc::new(obj);
    list.insert(key.clone(), info.clone());

    info
}

/// 添加Forge启动信息
/// - `obj`: 信息
pub fn add_forge(
    obj: ForgeLaunchObj,
    data: &Vec<u8>,
    mc: &str,
    version: &str,
    neo: bool,
) -> Arc<ForgeLaunchObj> {
    let v222 = version_checker::is_game_version_1202(mc);
    let name = if neo && v222 {
        format!("{}-{}", names::NEOFORGE_KEY, version)
    } else {
        format!("{}-{}-{}", names::FORGE_KEY, mc, version)
    };
    let file = if neo {
        NEOFORGE_DIR.get().unwrap().join(format!("{}.json", name))
    } else {
        FORGE_DIR.get().unwrap().join(format!("{}.json", name))
    };
    path_helper::write_bytes(&file, &data).unwrap();

    let key = LoaderKey::new(mc, version);
    let mut list = if neo {
        NEOFORGE_LAUNCHS.write().unwrap()
    } else {
        FORGE_LAUNCHS.write().unwrap()
    };
    let info = Arc::new(obj);
    list.insert(key, info.clone());

    info
}

/// 添加Forge安装信息
/// - `obj`: 信息
/// - `data`: 文本
pub fn add_forge_install(
    obj: ForgeInstallObj,
    data: &Vec<u8>,
    mc: &str,
    version: &str,
    neo: bool,
) -> Arc<ForgeInstallObj> {
    let name = get_forge_json_name(mc, version, neo, true);
    let file = if neo {
        NEOFORGE_DIR.get().unwrap().join(format!("{}.json", name))
    } else {
        FORGE_DIR.get().unwrap().join(format!("{}.json", name))
    };
    path_helper::write_bytes(&file, &data).unwrap();

    let key = LoaderKey::new(mc, version);
    let mut list = if neo {
        NEOFORGE_INSTALLS.write().unwrap()
    } else {
        FORGE_INSTALLS.write().unwrap()
    };
    let info = Arc::new(obj);
    list.insert(key, info.clone());

    info
}

/// 添加Quilt信息
/// - `obj`: Quilt加载器数据
/// - `data`: 文本
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub fn add_quilt(
    obj: QuiltLoaderObj,
    data: &Vec<u8>,
    mc: &str,
    version: &str,
) -> Arc<QuiltLoaderObj> {
    let file = QUILT_DIR.get().unwrap().join(format!("{}.json", obj.id));
    path_helper::write_bytes(&file, &data).unwrap();

    let key = LoaderKey::new(mc, version);
    let mut list = QUILT_LOADERS.write().unwrap();
    let info = Arc::new(obj);
    list.insert(key.clone(), info.clone());

    info
}

/// 添加自定义加载器信息
/// - `obj`: 自定义加载器
/// - `uuid`: 游戏实例
pub fn add_custom_loader(obj: CustomLoaderType, uuid: Uuid) {
    let mut list = CUSTOM_LOADERS.write().unwrap();
    list.insert(uuid, Arc::new(obj));
}

/// 添加高清修复信息
/// - `obj`: 高清修复信息
pub fn add_optifine(obj: OptifineObj) -> Arc<OptifineObj> {
    let mut list = OPTIFINE_LOADER.write().unwrap();
    let info = Arc::new(obj);
    list.insert(info.version.clone(), info.clone());

    save_optifine();

    info
}

/// 获取版本信息
/// - `version`: 游戏版本
pub fn get_version(version: &str) -> CoreResult<Arc<GameArgObj>> {
    let list = GAME_ARGS.read().unwrap();
    let data = list.get(version);

    match data {
        None => {
            let local = BASE_DIR
                .get()
                .unwrap()
                .join(format!("{}{}", version, names::JSON_EXT));
            let file = path_helper::open_read(&local)?;
            let json = serde_json::from_reader::<_, GameArgObj>(file).map_err(|err| {
                ErrorType::JsonError(ErrorData {
                    error: err.to_string(),
                })
            })?;
            let mut list = GAME_ARGS.write().unwrap();
            let data = Arc::new(json);
            list.insert(String::from(version), data.clone());

            Ok(data)
        }
        Some(data) => Ok(data.clone()),
    }
}

/// 检查游戏版本更新
/// - `version`: 游戏版本
pub async fn check_update(version: &str) -> CoreResult<Arc<GameArgObj>> {
    // 直接从在线更新数据
    get_version_from_online().await?;

    let versions = get_version_obj().await?;
    let item = versions
        .versions
        .iter()
        .filter(|&item| item.id.eq_ignore_ascii_case(version))
        .next();

    match item {
        // 在线也没有这个版本号
        None => Err(ErrorType::InfoNotFound),
        Some(item) => {
            let local = BASE_DIR.get().unwrap().join(format!("{}.json", version));
            let sha1 = hash_helper::gen_hash_from_file_async(HashType::Sha1, &local).await?;
            if sha1 != item.sha1 {
                Ok(add_game(item).await?)
            } else {
                Ok(get_version(version)?)
            }
        }
    }
}

/// 获取json名字
pub fn get_forge_json_name(mc: &str, version: &str, neo: bool, install: bool) -> String {
    if neo {
        let v222 = version_checker::is_game_version_1202(&mc);

        if install {
            if v222 {
                format!(
                    "{}-{}-{}{}",
                    names::NEOFORGE_KEY,
                    version,
                    names::FILE_INSTALL,
                    names::JSON_EXT
                )
            } else {
                format!(
                    "{}-{}-{}-{}{}",
                    names::FORGE_KEY,
                    mc,
                    version,
                    names::FILE_INSTALL,
                    names::JSON_EXT
                )
            }
        } else {
            if v222 {
                format!("{}-{}{}", names::NEOFORGE_KEY, version, names::JSON_EXT)
            } else {
                format!("{}-{}-{}{}", names::FORGE_KEY, mc, version, names::JSON_EXT)
            }
        }
    } else {
        if install {
            format!(
                "{}-{}-{}{}",
                names::FORGE_KEY,
                version,
                names::FILE_INSTALL,
                names::JSON_EXT
            )
        } else {
            format!("{}-{}{}", names::FORGE_KEY, version, names::JSON_EXT)
        }
    }
}

/// 获取NeoForge安装数据
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub fn get_neoforge_install_obj(mc: &str, version: &str) -> Option<Arc<ForgeInstallObj>> {
    let key = LoaderKey::new(mc, version);

    let list = NEOFORGE_INSTALLS.read().unwrap();
    let item = list.get(&key);
    match item {
        Some(item) => Some(item.clone()),
        None => {
            let local = NEOFORGE_DIR
                .get()
                .unwrap()
                .join(get_forge_json_name(mc, version, true, true));
            let file = path_helper::open_read(&local);
            if let Err(err) = file {
                mcml_log::error_type(err);

                return None;
            }
            let file = file.unwrap();
            let json = serde_json::from_reader::<_, ForgeInstallObj>(&file);

            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = NEOFORGE_INSTALLS.write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(ErrorData {
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
pub fn get_neoforge(mc: &str, version: &str) -> Option<Arc<ForgeLaunchObj>> {
    let key = LoaderKey::new(mc, version);

    let list = NEOFORGE_LAUNCHS.read().unwrap();
    let item = list.get(&key);
    match item {
        Some(item) => Some(item.clone()),
        None => {
            let local = NEOFORGE_DIR
                .get()
                .unwrap()
                .join(get_forge_json_name(mc, version, true, false));
            let file = path_helper::open_read(&local);
            if let Err(err) = file {
                mcml_log::error_type(err);

                return None;
            }
            let file = file.unwrap();
            let json = serde_json::from_reader::<_, ForgeLaunchObj>(&file);

            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = NEOFORGE_LAUNCHS.write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(ErrorData {
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
pub fn get_forge_install_obj(mc: &str, version: &str) -> Option<Arc<ForgeInstallObj>> {
    let key = LoaderKey::new(mc, version);

    let list = FORGE_INSTALLS.read().unwrap();
    let item = list.get(&key);
    match item {
        Some(item) => Some(item.clone()),
        None => {
            let local = FORGE_DIR
                .get()
                .unwrap()
                .join(get_forge_json_name(mc, version, false, true));
            let file = path_helper::open_read(&local);
            if let Err(err) = file {
                mcml_log::error_type(err);

                return None;
            }
            let file = file.unwrap();
            let json = serde_json::from_reader::<_, ForgeInstallObj>(&file);

            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = FORGE_INSTALLS.write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(ErrorData {
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
pub fn get_forge(mc: &str, version: &str) -> Option<Arc<ForgeLaunchObj>> {
    let key = LoaderKey::new(mc, version);

    let list = FORGE_LAUNCHS.read().unwrap();
    let item = list.get(&key);
    match item {
        Some(item) => Some(item.clone()),
        None => {
            let local = FORGE_DIR
                .get()
                .unwrap()
                .join(get_forge_json_name(mc, version, false, false));

            let file = path_helper::open_read(&local);
            if let Err(err) = file {
                mcml_log::error_type(err);

                return None;
            }
            let file = file.unwrap();
            let json = serde_json::from_reader::<_, ForgeLaunchObj>(&file);

            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = FORGE_LAUNCHS.write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(ErrorData {
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
pub fn get_fabric(mc: &str, version: &str) -> Option<Arc<FabricLoaderObj>> {
    let key = LoaderKey::new(mc, version);
    let list = FABRIC_LOADERS.read().unwrap();
    match list.get(&key) {
        None => {
            let local = FABRIC_DIR.get().unwrap().join(format!(
                "{}-{}-{}{}",
                names::FABRIC_LOADER_KEY,
                version,
                mc,
                names::JSON_EXT
            ));
            let file = path_helper::open_read(&local);
            if let Err(err) = file {
                mcml_log::error_type(err);

                return None;
            }
            let file = file.unwrap();
            let json = serde_json::from_reader::<_, FabricLoaderObj>(&file);
            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = FABRIC_LOADERS.write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(ErrorData {
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
pub fn get_quilt(mc: &str, version: &str) -> Option<Arc<QuiltLoaderObj>> {
    let key = LoaderKey::new(mc, version);
    let list = QUILT_LOADERS.read().unwrap();
    match list.get(&key) {
        None => {
            let local = FABRIC_DIR.get().unwrap().join(format!(
                "{}-{}-{}{}",
                names::FABRIC_LOADER_KEY,
                version,
                mc,
                names::JSON_EXT
            ));
            let file = path_helper::open_read(&local);
            if let Err(err) = file {
                mcml_log::error_type(err);

                return None;
            }
            let file = file.unwrap();
            let json = serde_json::from_reader::<_, QuiltLoaderObj>(&file);
            match json {
                Ok(json) => {
                    let temp = Arc::new(json);
                    let temp1 = temp.clone();

                    let mut list = QUILT_LOADERS.write().unwrap();
                    list.insert(key, temp);

                    Some(temp1)
                }
                Err(err) => {
                    mcml_log::error_type(ErrorType::JsonError(ErrorData {
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
pub fn get_optifine(version: &str) -> Option<Arc<OptifineObj>> {
    let list = OPTIFINE_LOADER.read().unwrap();
    Some(list.get(version)?.clone())
}

impl InstanceSettingObj {
    /// 获取游戏版本类型
    pub fn get_version_type(&self) -> String {
        let temp = VERSION.read().unwrap();

        if temp.is_none() {
            Default::default()
        } else {
            let temp = temp.clone().unwrap();

            if let Some(data) = temp
                .versions
                .iter()
                .filter(|item| item.id.eq_ignore_ascii_case(&self.version))
                .next()
            {
                data.version_type.clone()
            } else {
                Default::default()
            }
        }
    }

    /// 获取neoforge加载器信息
    pub fn get_forge(&self) -> Option<Arc<ForgeLaunchObj>> {
        match &self.loader_version {
            None => None,
            Some(data) => get_forge(&self.version, &data),
        }
    }

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
        let list = CUSTOM_LOADERS.read().unwrap();
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

    /// 更新游戏版本json
    pub async fn check_version_update(&self) -> CoreResult<Arc<GameArgObj>> {
        check_update(&self.version).await
    }
}
