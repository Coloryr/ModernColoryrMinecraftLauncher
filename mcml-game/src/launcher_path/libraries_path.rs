use std::{
    collections::HashMap,
    fs,
    hash::Hasher,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use mcml_base::{
    file_item::{
        FileHash::{Sha1, Sha256},
        FileItemObj,
    },
    get_system_info,
    hash_helper::{self, HashType},
    path_helper,
};
use mcml_names::{i18_items::error_type::CoreResult, names};
use mcml_net::{
    authlib_api::{self, AuthlibInjectorObj},
    nide8_api::{self, Nide8Obj},
    urls,
};

use crate::{
    game_launch::GameLaunchObj,
    launcher::{LoaderType, game_setting_obj::GameSettingObj},
};

/// 基础路径
static LIB_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 资源文件路径
static NATIVE_DIR: OnceLock<PathBuf> = OnceLock::new();

static AUTHLIB_FILE: OnceLock<FileItemObj> = OnceLock::new();
static NIDE8_FILE: OnceLock<FileItemObj> = OnceLock::new();

/// 获取基础路径
pub fn get_lib_dir() -> PathBuf {
    LIB_DIR.get().unwrap().clone()
}

/// 获取外部登陆jar
pub fn get_authlib_file() -> Option<PathBuf> {
    let file = AUTHLIB_FILE.get()?;

    Some(file.file.clone())
}

/// 获取统一通行证jar
pub fn get_nide8_file() -> Option<PathBuf> {
    let file = NIDE8_FILE.get()?;

    Some(file.file.clone())
}

/// 运行库信息
#[derive(Clone)]
pub struct LibVersionObj {
    /// 包名
    pub pack: String,
    /// 名字
    pub name: String,
    /// 版本号
    pub version: String,
    /// 额外信息
    pub extr: String,
}

impl PartialEq for LibVersionObj {
    fn eq(&self, other: &Self) -> bool {
        self.eq_without_version(other)
    }
}

impl Eq for LibVersionObj {}

impl std::hash::Hash for LibVersionObj {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pack.hash(state);
        self.name.hash(state);
        self.extr.hash(state);
    }
}

impl LibVersionObj {
    pub fn new(name: &str) -> Self {
        let arg: Vec<&str> = name.split(':').collect();

        if arg.len() < 3 {
            Self {
                pack: String::new(),
                name: String::from(name),
                version: String::new(),
                extr: String::new(),
            }
        } else if arg.len() > 3 {
            Self {
                pack: String::from(arg[0]),
                name: String::from(arg[1]),
                version: String::from(arg[2]),
                extr: String::from(arg[3]),
            }
        } else {
            Self {
                pack: String::from(arg[0]),
                name: String::from(arg[1]),
                version: String::from(arg[2]),
                extr: String::new(),
            }
        }
    }

    /// 判断运行库是否除了版本都一样
    pub fn eq_without_version(&self, obj: &LibVersionObj) -> bool {
        self.pack.eq(&obj.pack) && self.name.eq(&obj.name) && self.extr.eq(&obj.extr)
    }

    pub fn key_without_version(&self) -> (String, String, String) {
        (self.pack.clone(), self.name.clone(), self.extr.clone())
    }
}

/// 初始化版本路径
/// - `dir`: 运行路径
pub(crate) fn init<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    let dir = LIB_DIR.get_or_init(|| dir.as_ref().join(names::LIBRARIES_DIR));

    let sys = get_system_info();

    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    let dir = NATIVE_DIR.get_or_init(|| {
        dir.join(names::NATIVE_DIR)
            .join(sys.os.to_string().to_lowercase())
            .join(sys.system_arch.to_string().to_lowercase())
    });
    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    Ok(())
}

/// 获取Native文件夹
/// - `version`: 游戏版本
pub fn get_native_dir(version: Option<&str>) -> PathBuf {
    match version {
        Some(version) => {
            let dir = NATIVE_DIR.get().unwrap().join(version);
            if !dir.is_dir() {
                fs::create_dir(&dir).unwrap();
            }

            dir
        }
        None => NATIVE_DIR.get().unwrap().clone(),
    }
}

/// 获取游戏核心路径
/// - `version`: 游戏版本
pub fn get_game_file(version: &str) -> PathBuf {
    LIB_DIR
        .get()
        .unwrap()
        .join("net")
        .join("minecraft")
        .join("client")
        .join(version)
        .join(format!("client-{version}.jar"))
}

/// 获取游戏核心路径
/// - `custom`: 自定义版本号
pub fn get_game_file_with_custom(custom: &str) -> PathBuf {
    LIB_DIR
        .get()
        .unwrap()
        .join("net")
        .join("minecraft")
        .join("client")
        .join(format!("{custom}.jar"))
}

/// 获取OptiFine路径
/// - `mc`: 游戏版本
/// - `version`: optifine版本
pub fn get_optifine_file(mc: &str, version: &str) -> PathBuf {
    LIB_DIR
        .get()
        .unwrap()
        .join("optifine")
        .join("installer")
        .join(format!("OptiFine-{mc}-{version}.jar"))
}

impl GameSettingObj {
    /// 获取OptiFine路径
    pub fn get_optifine_file(&self) -> PathBuf {
        get_optifine_file(&self.version, self.loader_version.as_ref().unwrap())
    }

    /// 获取游戏核心路径
    pub fn get_game_file(&self) -> PathBuf {
        get_game_file(&self.version)
    }

    /// 获取所有运行库
    /// - `arg`: 启动参数
    pub fn get_libs(&self, arg: &GameLaunchObj) -> Vec<PathBuf> {
        let mut game_list = Vec::new();
        for item in &arg.game_libs {
            let key = LibVersionObj::new(&item.name);
            if let Some(pos) = game_list
                .iter()
                .position(|(k, _): &(LibVersionObj, PathBuf)| k.eq_without_version(&key))
            {
                game_list.remove(pos);
            }
            game_list.push((key, item.file.clone()));
        }

        if let Some(data) = &self.custom_loader
            && data.custom_json
        {
            return game_list
                .into_iter()
                .map(|(_, path)| path)
                .chain(std::iter::once(arg.game_jar.file.clone()))
                .collect();
        }

        let mut loader_list = Vec::new();
        for item in &arg.loader_libs {
            let key = LibVersionObj::new(&item.name);
            if let Some(pos) = loader_list
                .iter()
                .position(|(k, _): &(LibVersionObj, PathBuf)| k.eq_without_version(&key))
            {
                loader_list.remove(pos);
            }
            loader_list.push((key, item.file.clone()));
        }

        // 如果是自定义加载器则判断是否后置原版库
        let result = if self.loader == LoaderType::Custom
            && let Some(data) = &self.custom_loader
            && data.offset_lib
        {
            let mut temp = HashMap::with_capacity(loader_list.len() + game_list.len());

            for (key, value) in loader_list {
                add_or_update_lib_kv(&mut temp, key, value);
            }

            // 是否删除原版库
            if !data.remove_lib {
                for (key, value) in game_list {
                    if !temp.contains_key(&key) {
                        temp.insert(key, value);
                    }
                }
            }
            temp
        } else {
            let mut temp = HashMap::with_capacity(game_list.len() + loader_list.len());

            if let Some(data) = &self.custom_loader {
                // 是否删除原版库
                if !data.remove_lib {
                    for (key, value) in game_list {
                        add_or_update_lib_kv(&mut temp, key, value);
                    }
                }
            }

            for (key, value) in loader_list {
                add_or_update_lib_kv(&mut temp, key, value);
            }
            temp
        };

        let mut output: Vec<PathBuf> = result.into_values().collect();

        if self.loader != LoaderType::NeoForge {
            output.push(arg.game_jar.file.clone());
        }

        output
    }
}

/// 删除冲突的库
fn add_or_update_lib_kv(
    map: &mut HashMap<LibVersionObj, PathBuf>,
    key: LibVersionObj,
    value: PathBuf,
) {
    map.retain(|k, _| !k.eq_without_version(&key));
    map.insert(key, value);
}

/// 创建AuthlibInjector下载实例
/// - `obj`: AuthlibInjector信息
pub fn build_authlib_injector_item(obj: &AuthlibInjectorObj) -> FileItemObj {
    FileItemObj {
        name: format!("moe.yushi:authlibinjector:{}", obj.version),
        file: LIB_DIR
            .get()
            .unwrap()
            .join("moe")
            .join("yushi")
            .join("authlibinjector")
            .join(&obj.version)
            .join(format!("authlib-injector-{}.jar", obj.version)),
        url: obj.download_url.clone(),
        hash: Sha256(obj.checksums.sha256.clone()),
        later: Default::default(),
    }
}

async fn read_authlib_injector() -> CoreResult<Option<FileItemObj>> {
    let obj = authlib_api::get_obj().await?;
    let item = build_authlib_injector_item(&obj);

    if item.file.exists() {
        let sha256 = hash_helper::gen_hash_from_file_async(HashType::Sha256, &item.file).await?;
        if obj.checksums.sha256 != sha256 {
            Ok(Some(item))
        } else {
            AUTHLIB_FILE.set(item.clone());

            Ok(None)
        }
    } else {
        Ok(Some(item))
    }
}

/// 初始化AuthlibInjector，不存在返回下载项目
pub async fn ready_authlib_injector() -> CoreResult<Option<FileItemObj>> {
    match AUTHLIB_FILE.get() {
        Some(obj) => {
            let path = &obj.file;
            if !path.exists() {
                Ok(Some(obj.clone()))
            } else {
                let sha256 = hash_helper::gen_hash_from_file_async(HashType::Sha256, &path).await?;
                if !obj.hash.eq(&sha256) {
                    Ok(Some(obj.clone()))
                } else {
                    Ok(None)
                }
            }
        }
        None => read_authlib_injector().await,
    }
}

/// 创建Nide8Injector下载实例
/// - `obj`: 下载信息
pub fn build_nide8_item(obj: &Nide8Obj) -> FileItemObj {
    FileItemObj {
        name: format!("com.nide8.login2:nide8auth:{}", obj.jar_version),
        file: LIB_DIR
            .get()
            .unwrap()
            .join("com")
            .join("nide8")
            .join("login2")
            .join("nide8auth")
            .join(&obj.jar_version)
            .join(format!("nide8auth-{}.jar", obj.jar_version)),
        url: String::from(urls::NIDE8_JAR_URL),
        hash: Sha1(obj.jar_hash.clone()),
        later: Default::default(),
    }
}

async fn read_nide8() -> CoreResult<Option<FileItemObj>> {
    let obj = nide8_api::get_obj().await?;
    let item = build_nide8_item(&obj);

    if item.file.exists() {
        let sha1 = hash_helper::gen_hash_from_file_async(HashType::Sha1, &item.file).await?;
        if obj.jar_hash != sha1 {
            Ok(Some(item))
        } else {
            NIDE8_FILE.set(item.clone());

            Ok(None)
        }
    } else {
        Ok(Some(item))
    }
}

/// 初始化Nide8Injector，不存在返回下载项目
pub async fn ready_nide8() -> CoreResult<Option<FileItemObj>> {
    match NIDE8_FILE.get() {
        Some(obj) => {
            let path = &obj.file;
            if !path.exists() {
                Ok(Some(obj.clone()))
            } else {
                let sha256 = hash_helper::gen_hash_from_file_async(HashType::Sha256, &path).await?;
                if !obj.hash.eq(&sha256) {
                    Ok(Some(obj.clone()))
                } else {
                    Ok(None)
                }
            }
        }
        None => read_nide8().await,
    }
}
