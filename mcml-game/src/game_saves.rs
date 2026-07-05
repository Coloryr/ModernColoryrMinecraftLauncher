use std::{
    io::{Read, Seek},
    path::{Path, PathBuf},
    sync::Arc,
    time::SystemTime,
};

use chrono::{Datelike, Local, Timelike};
use mcml_base::{
    archives::{self, ArchiveGui, ArchiveType},
    path_helper,
};
use mcml_names::{i18_items::error_type::CoreResult, names};
use mcml_nbt::{NbtType, nbt_file::NbtFile};

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

fn read_save<R: Read + Seek>(stream: &mut R) -> CoreResult<SaveObj> {
    let nbt = NbtFile::read(stream)?;

    let mut obj = SaveObj::default();

    if let Some(nbt) = nbt.nbt.as_compound() {
        if let Some(NbtType::Compound(data)) = nbt.data.get("Data") {
            if let Some(NbtType::Long(data1)) = data.data.get("LastPlayed") {
                obj.last_played = data1.data;
            }

            if let Some(NbtType::Long(data1)) = data.data.get("RandomSeed") {
                obj.random_seed = data1.data;
            }

            if let Some(NbtType::Compound(data1)) = data.data.get("WorldGenSettings") {
                if let Some(NbtType::Long(data2)) = data1.data.get("seed") {
                    obj.random_seed = data2.data;
                }

                if let Some(NbtType::Compound(data2)) = data1.data.get("dimensions") {
                    if let Some(NbtType::Compound(data3)) = data2.data.get("minecraft:overworld") {
                        if let Some(NbtType::Compound(data4)) = data3.data.get("generator") {
                            if let Some(NbtType::String(data5)) = data4.data.get("settings") {
                                obj.generator_name = data5.data.clone();
                            }
                        }
                    }
                }
            }

            if let Some(NbtType::String(data1)) = data.data.get("generatorName") {
                obj.generator_name = format!("minecraft:{}", data1.data);
            }

            if let Some(NbtType::Int(data1)) = data.data.get("GameType") {
                obj.game_type = data1.data;
            }

            if let Some(NbtType::Byte(data1)) = data.data.get("hardcore") {
                obj.hard_core = data1.data;
            }

            if let Some(NbtType::Byte(data1)) = data.data.get("Difficulty") {
                obj.difficulty = data1.data;
            }

            if let Some(NbtType::String(data1)) = data.data.get("LevelName") {
                obj.level_name = data1.data.clone();
            }
        }
    } else {
        obj.broken = true;
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
    pub fn unzip_backup(&self, file: &str, gui: Option<Box<dyn ArchiveGui + Send + Sync>>) {}
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
