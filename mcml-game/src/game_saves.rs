use std::{
    io::{Read, Seek},
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{Datelike, Local, Timelike};
use mcml_base::{
    archives::{self, ArchiveGui, ArchiveType},
    path_helper,
};
use mcml_names::{i18_items::error_type::CoreResult, names};
use mcml_nbt::{NbtType, nbt_file::NbtFile, nbt_types::NbtCompound};

use crate::launcher::instance_setting_obj::InstanceSettingObj;

/// 游戏存档
pub struct SaveObj {
    /// 地图种子
    pub random_seed: i64,
    /// 上次游玩
    pub last_played: i64,
    /// 游戏类型
    pub game_type: i32,
    /// 极限模式
    pub hard_core: u8,
    /// 世界名字
    pub level_name: String,
    /// 难度
    pub difficulty: u8,
    /// 生成器名字
    pub generator_name: String,
    /// 路径
    pub path: PathBuf,
    /// 图标
    pub icon: Option<PathBuf>,
    /// 是否损坏
    pub broken: bool,
    /// 实例
    pub instance: Arc<InstanceSettingObj>,
}

impl Default for SaveObj {
    fn default() -> Self {
        Self {
            random_seed: Default::default(),
            game_type: Default::default(),
            hard_core: Default::default(),
            level_name: Default::default(),
            difficulty: Default::default(),
            generator_name: Default::default(),
            path: Default::default(),
            icon: Default::default(),
            broken: Default::default(),
            last_played: Default::default(),
            instance: Default::default(),
        }
    }
}

// ---- NBT 辅助提取函数 ----

/// 从 NbtCompound 中提取 `&NbtCompound`
fn get_compound<'a>(c: &'a NbtCompound, key: &str) -> Option<&'a NbtCompound> {
    match c.data.get(key) {
        Some(NbtType::Compound(v)) => Some(v),
        _ => None,
    }
}

/// 从 NbtCompound 中提取 i64
fn get_long(c: &NbtCompound, key: &str) -> Option<i64> {
    match c.data.get(key) {
        Some(NbtType::Long(v)) => Some(v.data),
        _ => None,
    }
}

/// 从 NbtCompound 中提取 i32
fn get_int(c: &NbtCompound, key: &str) -> Option<i32> {
    match c.data.get(key) {
        Some(NbtType::Int(v)) => Some(v.data),
        _ => None,
    }
}

/// 从 NbtCompound 中提取 u8
fn get_byte(c: &NbtCompound, key: &str) -> Option<u8> {
    match c.data.get(key) {
        Some(NbtType::Byte(v)) => Some(v.data),
        _ => None,
    }
}

/// 从 NbtCompound 中提取 &str
fn get_string<'a>(c: &'a NbtCompound, key: &str) -> Option<&'a str> {
    match c.data.get(key) {
        Some(NbtType::String(v)) => Some(&v.data),
        _ => None,
    }
}

fn read_save<R: Read + Seek>(stream: &mut R) -> CoreResult<SaveObj> {
    let nbt = NbtFile::read(stream)?;
    let mut obj = SaveObj::default();

    let Some(nbt) = nbt.nbt.as_compound() else {
        obj.broken = true;
        return Ok(obj);
    };

    let Some(data) = get_compound(nbt, "Data") else {
        obj.broken = true;
        return Ok(obj);
    };

    // 基础字段提取
    obj.last_played = get_long(data, "LastPlayed").unwrap_or(0);
    obj.random_seed = get_long(data, "RandomSeed").unwrap_or(0);
    obj.game_type = get_int(data, "GameType").unwrap_or(0);
    obj.hard_core = get_byte(data, "hardcore").unwrap_or(0);
    obj.difficulty = get_byte(data, "Difficulty").unwrap_or(0);
    obj.level_name = get_string(data, "LevelName")
        .map(String::from)
        .unwrap_or_default();

    // WorldGenSettings（新版格式，含种子和生成器名）
    if let Some(world_gen) = get_compound(data, "WorldGenSettings") {
        if let Some(seed) = get_long(world_gen, "seed") {
            obj.random_seed = seed;
        }
        let gen_name = get_compound(world_gen, "dimensions")
            .and_then(|d: &NbtCompound| get_compound(d, "minecraft:overworld"))
            .and_then(|o: &NbtCompound| get_compound(o, "generator"))
            .and_then(|g: &NbtCompound| get_string(g, "settings"));
        if let Some(name) = gen_name {
            obj.generator_name = name.to_string();
        }
    }

    // generatorName（旧版格式，优先级高于 WorldGenSettings）
    if let Some(name) = get_string(data, "generatorName") {
        obj.generator_name = format!("minecraft:{}", name);
    }

    Ok(obj)
}

impl InstanceSettingObj {
    /// 获取实例存档列表
    pub fn get_saves(&self) -> Vec<SaveObj> {
        let mut list = Vec::new();
        let dir = self.get_saves_path();
        let dirs = path_helper::get_dirs(dir);

        for item in dirs.iter() {
            let file = item.join(names::LEVEL_FILE);
            if file.exists() && file.is_file() {
                let stream = path_helper::open_read(&file);
                if let Ok(mut stream) = stream {
                    let data = read_save(&mut stream);
                    match data {
                        Ok(mut obj) => {
                            let file = item.join(names::ICON_FILE);
                            if file.exists() {
                                obj.icon = Some(file);
                            }

                            list.push(obj);
                        }
                        Err(err) => {
                            mcml_log::error_type(err);
                        }
                    }
                }
            }
        }

        list
    }

    /// 还原备份
    pub fn unzip_backup(
        &self,
        file: &str,
        gui: Option<Box<dyn ArchiveGui + Send + Sync>>,
    ) -> CoreResult<()> {
        let backup_file = self.get_backup_path().join(file);
        let saves_dir = self.get_saves_path();

        archives::decompress(ArchiveType::Zip, &backup_file, &saves_dir, gui)
    }

    /// 获取备份文件列表
    pub fn get_backup_files(&self) -> Vec<PathBuf> {
        path_helper::get_files(self.get_backup_path())
    }
}

impl SaveObj {
    /// 获取数据包文件夹
    pub fn get_datapack_path(&self) -> PathBuf {
        self.path.join(names::GAME_DATAPACK_DIR)
    }

    /// 删除存档
    pub fn delete(&self) -> CoreResult<()> {
        path_helper::move_to_trash(&self.path)
    }

    /// 导出存档
    pub fn export<P: AsRef<Path>>(
        &self,
        path: P,
        archive_type: ArchiveType,
        gui: Option<Box<dyn ArchiveGui + Send + Sync>>,
    ) -> CoreResult<()> {
        archives::compress(
            archive_type,
            path.as_ref(),
            self.path.as_ref(),
            None::<&Path>,
            &None,
            gui,
        )
    }

    /// 备份存档
    pub fn backup(&self, gui: Option<Box<dyn ArchiveGui + Send + Sync>>) -> CoreResult<()> {
        let path = self.instance.get_backup_path();
        path_helper::create_dir_all(&path)?;

        let time = Local::now();
        let file = path.join(format!(
            "{}_{}_{}_{}_{}_{}_{}.zip",
            self.level_name,
            time.year(),
            time.month(),
            time.day(),
            time.hour(),
            time.minute(),
            time.second()
        ));

        archives::compress(ArchiveType::Zip, &file, &self.path, None, &None, gui)
    }
}
