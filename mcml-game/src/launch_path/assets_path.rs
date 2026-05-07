/// 游戏资源路径
use std::{fs, io::Cursor, path::PathBuf, sync::OnceLock};

use mcml_base::path_helper;
use mcml_names::{i18_items::error_type::ErrorType, names};

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
/// - `dir`: 运行目录
pub fn init(dir: &PathBuf) {
    let dir = BASE_DIR.get_or_init(|| dir.join(names::NAME_GAME_ASSETS_DIR));

    OBJECTS_DIR.set(dir.join(names::NAME_GAME_INDEX_DIR));
    INDEX_DIR.set(dir.join(names::NAME_GAME_OBJECT_DIR));
    SKIN_DIR.set(dir.join(names::NAME_GAME_SKIN_DIR));

    let dir = dir.as_path();
    if !dir.exists() {
        fs::create_dir(dir).unwrap();
    }

    let dir = OBJECTS_DIR.get().unwrap();
    if !dir.exists() {
        fs::create_dir(dir).unwrap();
    }

    let dir = INDEX_DIR.get().unwrap();
    if !dir.exists() {
        fs::create_dir(dir).unwrap();
    }

    let dir = SKIN_DIR.get().unwrap();
    if !dir.exists() {
        fs::create_dir(dir).unwrap();
    }
}

impl GameArgObj {
    /// 添加资源数据
    ///
    /// - `data`: 资源文件
    pub fn add_index(&self, data: &mut Cursor<Vec<u8>>) {
        let index = &self.asset_index.as_ref().unwrap();
        let file = INDEX_DIR.get().unwrap().join(format!("{}.json", index.id));
        path_helper::write_bytes_from_stream(&file, data);
    }
}

impl GameAssetIndexObj {
    /// 获取资源数据
    pub fn get_index(&self) -> Result<AssetsObj, ErrorType> {
        let file = INDEX_DIR.get().unwrap().join(format!("{}.json", self.id));
        let stream = path_helper::open_read(&file);
        match stream {
            None => Err(ErrorType::FileNotExists(file.display().to_string())),
            Some(stream) => {
                let obj = serde_json::from_reader::<_, AssetsObj>(stream);
                match obj {
                    Err(err) => Err(ErrorType::JsonDecError(err.to_string())),
                    Ok(ok) => Ok(ok),
                }
            }
        }
    }
}
