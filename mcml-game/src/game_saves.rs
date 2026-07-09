use std::{
    collections::HashMap,
    io::{Read, Seek},
    path::{Path, PathBuf},
    sync::Arc,
};

use chrono::{Datelike, Local, Timelike};
use mcml_base::{
    archives::{self, ArchiveGui, ArchiveType, BaseArchive},
    path_helper,
};
use mcml_config::config_save;
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData},
    names, uuids,
};
use mcml_nbt::{nbt_file::NbtFile, nbt_types::NbtCompound};
use serde::{Deserialize, Serialize};

use crate::launcher::instance_setting_obj::InstanceSettingObj;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct SaveBackupObj {
    pub dir: String,
    pub back: Vec<String>,
}

impl Default for SaveBackupObj {
    fn default() -> Self {
        Self {
            dir: Default::default(),
            back: Default::default(),
        }
    }
}

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

    let Some(nbt) = nbt.nbt.as_compound() else {
        obj.broken = true;
        return Ok(obj);
    };

    let Some(data) = nbt.get_compound("Data") else {
        obj.broken = true;
        return Ok(obj);
    };

    // 基础字段提取
    obj.last_played = data.get_long("LastPlayed").unwrap_or(0);
    obj.random_seed = data.get_long("RandomSeed").unwrap_or(0);
    obj.game_type = data.get_int("GameType").unwrap_or(0);
    obj.hard_core = data.get_byte("hardcore").unwrap_or(0);
    obj.difficulty = data.get_byte("Difficulty").unwrap_or(0);
    obj.level_name = data
        .get_string("LevelName")
        .map(String::from)
        .unwrap_or_default();

    // WorldGenSettings（新版格式，含种子和生成器名）
    if let Some(world_gen) = data.get_compound("WorldGenSettings") {
        if let Some(seed) = world_gen.get_long("seed") {
            obj.random_seed = seed;
        }
        let gen_name = world_gen
            .get_compound("dimensions")
            .and_then(|d: &NbtCompound| d.get_compound("minecraft:overworld"))
            .and_then(|o: &NbtCompound| o.get_compound("generator"))
            .and_then(|g: &NbtCompound| g.get_string("settings"));
        if let Some(name) = gen_name {
            obj.generator_name = name.to_string();
        }
    }

    // generatorName（旧版格式，优先级高于 WorldGenSettings）
    if let Some(name) = data.get_string("generatorName") {
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
                if let Ok(mut stream) = path_helper::open_read(&file) {
                    let data = read_save(&mut stream);
                    match data {
                        Ok(mut obj) => {
                            let file = item.join(names::ICON_FILE);
                            if file.exists() {
                                obj.icon = Some(file);
                            }

                            obj.path = item.clone();
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
    pub fn restore_backup(
        &self,
        info: &SaveBackupObj,
        file: &str,
        gui: Option<Box<dyn ArchiveGui + Send + Sync>>,
    ) -> CoreResult<()> {
        let dir = self.get_backup_path();
        let path = dir.join(info.dir.clone());
        if path.exists() && path.is_dir() {
            path_helper::move_to_trash(&path)?;
        }

        let backup_file = dir.join(file);

        archives::decompress(ArchiveType::Zip, &backup_file, &path, gui)
    }

    /// 获取备份文件列表
    pub fn get_backups(&self) -> CoreResult<HashMap<String, SaveBackupObj>> {
        let file = self.get_backup_file();
        if file.exists() && file.is_file() {
            let stream = path_helper::open_read(&file)?;
            let json = serde_json::from_reader::<_, HashMap<String, SaveBackupObj>>(&stream)
                .map_err(|err| {
                    ErrorType::SerializerError(ErrorData {
                        error: err.to_string(),
                    })
                })?;

            Ok(json)
        } else {
            Ok(HashMap::new())
        }
    }

    /// 保存备份信息
    pub fn save_backups(&self, info: &HashMap<String, SaveBackupObj>) {
        let file = self.get_backup_file();
        config_save::save(uuids::mix_uuid(uuids::BACKUP_UUID, self.uuid), info, &file);
    }

    /// 导入存档
    pub fn import_save<P: AsRef<Path>>(&self, file: P) -> CoreResult<()> {
        let saves_dir = self.get_saves_path();
        if !saves_dir.exists() {
            path_helper::create_dir_all(&saves_dir)?;
        }

        let archive = BaseArchive::open(file.as_ref())?;

        // 如果包内所有条目都套在同一个顶层文件夹里，直接解压到 saves 目录；
        // 否则以压缩包文件名新建一个文件夹再解压进去。
        let output_dir = if archive.single_top_dir().is_some() {
            // 有唯一顶层文件夹：直接解压，顶层文件夹就是存档目录
            saves_dir
        } else {
            // 没有套壳：用压缩包文件名（去掉后缀）建一个新目录
            let file_path = file.as_ref();
            let stem = file_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| "save".to_string());
            // 二次去后缀（处理 .tar.gz 这类双后缀）
            let stem = stem.trim_end_matches(".tar").to_string();
            saves_dir.join(&stem)
        };

        path_helper::create_dir_all(&output_dir)?;
        archive.extract_all(&output_dir, None)
    }
}

impl SaveBackupObj {}

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
        let name = format!(
            "{}_{}_{}_{}_{}_{}_{}.zip",
            self.level_name,
            time.year(),
            time.month(),
            time.day(),
            time.hour(),
            time.minute(),
            time.second()
        );
        let file = path.join(&name);

        archives::compress(ArchiveType::Zip, &file, &self.path, None, &None, gui)?;

        let mut info = self.instance.get_backups()?;
        if let Some(obj) = info.get_mut(&self.level_name) {
            obj.back.push(name);
        } else {
            let path = self.instance.get_saves_path();
            let dir = self
                .path
                .strip_prefix(path)
                .map_err(|err| {
                    ErrorType::FileSystemError(FileSystemErrorData {
                        path: self.path.clone(),
                        error: err.to_string(),
                    })
                })?
                .to_string_lossy()
                .to_string();
            let back: Vec<String> = vec![name];
            info.insert(self.level_name.clone(), SaveBackupObj { dir, back });
        }

        self.instance.save_backups(&info);

        Ok(())
    }
}
