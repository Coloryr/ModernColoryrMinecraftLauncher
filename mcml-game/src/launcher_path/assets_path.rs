/// 游戏资源路径
use std::{
    fs::File,
    io::Cursor,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use mcml_auth::LoginObj;
use mcml_base::{path_helper, serialize_tools};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType},
    names,
};
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
/// - `dir`: 运行目录
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
pub fn get_assets_dir() -> PathBuf {
    BASE_DIR.get().unwrap().clone()
}

/// 获取路径
pub fn get_obj_dir() -> PathBuf {
    OBJECTS_DIR.get().unwrap().clone()
}

/// 添加资源数据
///
/// - `data`: 资源文件
pub fn add_index(obj: &GameArgObj, data: &mut Cursor<Vec<u8>>) {
    let index = obj.asset_index.as_ref().unwrap();
    let file = INDEX_DIR.get().unwrap().join(format!("{}.json", index.id));
    path_helper::write_stream(&file, data).unwrap();
}

/// 获取资源数据
/// - `obj`：版本数据资源
pub fn get_index(obj: &GameAssetIndexObj) -> CoreResult<AssetsObj> {
    let file = INDEX_DIR.get().unwrap().join(format!("{}.json", obj.id));
    let obj = serialize_tools::json_from_file::<AssetsObj>(&file)?;
    Ok(obj)
}

/// 保存皮肤图片
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
/// - `url`: 网页地址，以UUID结尾
/// 没有返回 UUID(0)的文件位置
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
    Path::new(&SKIN_DIR.get().unwrap())
        .join(dir)
        .with_file_name(name)
}

/// 读取资源文件
/// - `hash`: 资源文件SHA1值
pub fn read_assets_text(hash: String) -> Option<String> {
    let dir: String = hash.chars().take(2).collect();
    let local = Path::new(&OBJECTS_DIR.get().unwrap())
        .join(dir)
        .with_file_name(hash);

    let file = path_helper::read_text(&local);
    match file {
        Err(err) => {
            mcml_log::error_type(err);

            None
        }
        Ok(file) => Some(file),
    }
}

/// 读取资源文件
/// - `hash`: 资源文件SHA1值
pub fn read_assets_stream(hash: String) -> Option<File> {
    let dir: String = hash.chars().take(2).collect();
    let local = Path::new(&OBJECTS_DIR.get().unwrap())
        .join(dir)
        .with_file_name(hash);

    let file = path_helper::open_read(&local);
    match file {
        Err(err) => {
            mcml_log::error_type(err);

            None
        }
        Ok(file) => Some(file),
    }
}
