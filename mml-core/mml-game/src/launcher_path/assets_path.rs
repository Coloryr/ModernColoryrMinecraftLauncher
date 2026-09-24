//! 游戏资源（assets）目录

use std::{
    fs::File,
    io::Cursor,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use mml_auth::LoginObj;
use mml_base::serialize_tools;
use mml_names::{i18_items::error_type::CoreResult, names};
use mml_sys::path_helper;
use url::Url;
use uuid::Uuid;

use crate::mojang::{
    assets_obj::AssetsObj,
    game_arg_obj::{GameArgObj, GameAssetIndexObj},
};

/// 基础路径
static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 资源文件路径
static OBJECTS_DIR: OnceLock<PathBuf> = OnceLock::new();
/// 索引文件路径
static INDEX_DIR: OnceLock<PathBuf> = OnceLock::new();
/// 皮肤文件路径
static SKIN_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 初始化
///
/// # 参数
///
/// - `dir`: 运行目录
///
/// # 返回值
///
/// 成功返回 `Ok(())`；创建目录失败返回对应错误
pub(crate) fn init<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    let dir = BASE_DIR.get_or_init(|| dir.as_ref().join(names::GAME_ASSETS_DIR));
    if !dir.exists() {
        path_helper::create_dir_all(dir)?;
    }

    let obj = OBJECTS_DIR.get_or_init(|| dir.join(names::GAME_INDEX_DIR));
    if !obj.is_dir() {
        path_helper::create_dir_all(obj)?;
    }

    let index = INDEX_DIR.get_or_init(|| dir.join(names::GAME_OBJECT_DIR));
    if !index.is_dir() {
        path_helper::create_dir_all(index)?;
    }

    let skin = SKIN_DIR.get_or_init(|| dir.join(names::GAME_SKIN_DIR));
    if !skin.is_dir() {
        path_helper::create_dir_all(skin)?;
    }

    Ok(())
}

/// 获取资源文件夹
///
/// # 返回值
///
/// 返回资源根目录
pub fn get_assets_dir() -> PathBuf {
    BASE_DIR.get().unwrap().clone()
}

/// 获取对象文件目录
///
/// # 返回值
///
/// 返回资源对象文件目录
pub fn get_obj_dir() -> PathBuf {
    OBJECTS_DIR.get().unwrap().clone()
}

/// 添加资源数据
///
/// # 参数
///
/// - `obj`: 版本数据
/// - `data`: 资源索引文件数据
pub fn add_index(obj: &GameArgObj, data: &mut Cursor<Vec<u8>>) {
    let index = obj.asset_index.as_ref().unwrap();
    let file = INDEX_DIR.get().unwrap().join(format!("{}.json", index.id));
    path_helper::write_stream(&file, data).unwrap();
}

/// 获取资源数据
///
/// # 参数
///
/// - `obj`: 版本数据资源
///
/// # 返回值
///
/// 返回解析出的资源索引；读取或解析失败返回对应错误
pub fn get_index(obj: &GameAssetIndexObj) -> CoreResult<AssetsObj> {
    let file = INDEX_DIR.get().unwrap().join(format!("{}.json", obj.id));
    let obj = serialize_tools::json_from_file::<AssetsObj>(&file)?;
    Ok(obj)
}

/// 保存皮肤图片
///
/// # 参数
///
/// - `obj`: 保存的账户
/// - `file`: 需要导入的文件
pub fn save_skin(obj: LoginObj, file: PathBuf) {
    let path = SKIN_DIR
        .get()
        .unwrap()
        .join(format!("{}_skin.png", obj.uuid));
    path_helper::copy_file(&file, &path).unwrap();
}

/// 获取url的皮肤位置
///
/// # 参数
///
/// - `url`: 网页地址，以UUID结尾
///
/// # 返回值
///
/// 返回皮肤文件位置；地址解析失败时返回 UUID(0) 的文件位置
pub fn get_skin_from_url(url: String) -> PathBuf {
    let name = if let Ok(url) = Url::parse(&url)
        && let Some(filename) = url.path().split('/').last()
        && !filename.is_empty()
    {
        filename.to_string()
    } else {
        Uuid::from_u128(0).to_string()
    };

    let dir: String = name.chars().take(2).collect();
    Path::new(&SKIN_DIR.get().unwrap()).join(dir).join(name)
}

/// 读取资源文件
///
/// # 参数
///
/// - `hash`: 资源文件SHA1值
///
/// # 返回值
///
/// 返回文件文本内容；读取失败记录日志并返回 `None`
pub fn read_assets_text(hash: String) -> Option<String> {
    let dir: String = hash.chars().take(2).collect();
    let local = Path::new(&OBJECTS_DIR.get().unwrap())
        .join(dir)
        .with_file_name(hash);

    let file = path_helper::read_text(&local);
    match file {
        Err(err) => {
            mml_log::error_type(err);

            None
        }
        Ok(file) => Some(file),
    }
}

/// 读取资源文件
///
/// # 参数
///
/// - `hash`: 资源文件SHA1值
///
/// # 返回值
///
/// 返回文件读取流；读取失败记录日志并返回 `None`
pub fn read_assets_stream(hash: String) -> Option<File> {
    let dir: String = hash.chars().take(2).collect();
    let local = Path::new(&OBJECTS_DIR.get().unwrap())
        .join(dir)
        .with_file_name(hash);

    let file = path_helper::open_read(&local);
    match file {
        Err(err) => {
            mml_log::error_type(err);

            None
        }
        Ok(file) => Some(file),
    }
}
