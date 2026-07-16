/// 游戏实例存档相关
/// 包括存档里面的数据包
use std::{
    collections::HashMap,
    io::{Read, Seek},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use chrono::{Datelike, Local, Timelike};
use mcml_base::{
    archives::{self, ArchiveGui, ArchiveType, BaseArchive},
    path_helper, serialize_tools,
};
use mcml_config::config_save;
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData},
    names, uuids,
};
use mcml_nbt::{
    nbt_file::NbtFile,
    nbt_types::{NbtCompound, NbtList, NbtString},
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use zip::ZipArchive;

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
    /// 存档信息
    pub nbt: NbtFile,
}

/// 数据包信息
pub struct SaveDataPackObj {
    /// 名字
    pub name: String,
    /// 路径
    pub path: PathBuf,
    /// 描述
    pub description: String,
    /// 格式版本号
    pub pack_format: i64,
    /// 是否启用
    pub enable: Option<bool>,
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
            nbt: Default::default(),
        }
    }
}

fn read_save<R: Read + Seek>(stream: &mut R) -> CoreResult<SaveObj> {
    let nbt_file = NbtFile::read(stream)?;
    let mut obj = SaveObj::default();

    let Some(nbt) = nbt_file.nbt.as_compound() else {
        obj.broken = true;
        obj.nbt = nbt_file;
        return Ok(obj);
    };

    let Some(data) = nbt.get_compound("Data") else {
        obj.broken = true;
        obj.nbt = nbt_file;
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

    obj.nbt = nbt_file;
    Ok(obj)
}

impl InstanceSettingObj {
    /// 获取实例存档列表
    pub async fn get_saves(&self) -> Vec<SaveObj> {
        let dir = self.get_saves_path();
        let dirs = path_helper::get_dirs(dir);

        tokio::task::spawn_blocking(move || {
            let list = Mutex::new(Vec::new());

            dirs.par_iter().for_each(|item| {
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
                                list.lock().unwrap().push(obj);
                            }
                            Err(err) => {
                                mcml_log::error_type(err);
                            }
                        }
                    }
                }
            });

            list.into_inner().unwrap()
        })
        .await
        .unwrap_or_default()
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
            serialize_tools::json_stream(&stream)
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

impl SaveObj {
    /// 获取数据包文件夹
    pub fn get_datapack_path(&self) -> PathBuf {
        self.path.join(names::GAME_DATAPACK_DIR)
    }

    /// 获取存档信息文件
    pub fn get_level_file(&self) -> PathBuf {
        self.path.join(names::LEVEL_FILE)
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
    pub fn backup(
        &self,
        instance: &Arc<InstanceSettingObj>,
        gui: Option<Box<dyn ArchiveGui + Send + Sync>>,
    ) -> CoreResult<()> {
        let path = instance.get_backup_path();
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

        let mut info = instance.get_backups()?;
        if let Some(obj) = info.get_mut(&self.level_name) {
            obj.back.push(name);
        } else {
            let path = instance.get_saves_path();
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

        instance.save_backups(&info);

        Ok(())
    }

    /// 存储
    fn save_nbt(&self) -> CoreResult<()> {
        let file = self.get_level_file();
        let mut stream = path_helper::open_read(file)?;

        self.nbt.write(&mut stream)?;

        Ok(())
    }

    /// 获取数据包列表
    pub fn get_datapacks(&self) -> CoreResult<Vec<SaveDataPackObj>> {
        let path = self.get_datapack_path();
        if !path.exists() || !path.is_dir() {
            return Ok(Vec::new());
        }

        let list = Mutex::new(Vec::new());

        if let Some(nbt) = self
            .nbt
            .nbt
            .as_compound()
            .and_then(|map| map.get_compound("Data"))
            .and_then(|map1| map1.get_compound("DataPacks"))
        {
            let ens = nbt.get_list("Enabled");
            let dis = nbt.get_list("Disabled");

            let files = path_helper::get_files(&path);

            files.par_iter().for_each(|item| {
                if let Some(ext) = item.extension()
                    && ext.eq_ignore_ascii_case(names::ZIP_EXT)
                {
                    let stream = path_helper::open_read(item);
                    if let Err(err) = stream {
                        mcml_log::error_type(err);
                        return;
                    }

                    let stream = stream.unwrap();
                    let zip = ZipArchive::new(stream);
                    if let Err(err) = zip {
                        mcml_log::error_type(ErrorType::ArchiveOpenError(FileSystemErrorData {
                            path: path.clone(),
                            error: err.to_string(),
                        }));
                        return;
                    }

                    let mut zip = zip.unwrap();
                    let meta = zip.by_name(names::PACK_META_FILE);
                    if let Err(err) = meta {
                        mcml_log::error_type(ErrorType::ArchiveReadError(ErrorData {
                            error: err.to_string(),
                        }));
                        return;
                    }
                    let json: Result<Value, ErrorType> =
                        serialize_tools::json_stream(meta.unwrap());
                    if let Err(err) = json {
                        mcml_log::error_type(ErrorType::SerializerError(ErrorData {
                            error: err.to_string(),
                        }));
                        return;
                    }
                    let json = json.unwrap();
                    if let Some(data) = read_pack(path.clone(), ens, dis, json) {
                        list.lock().unwrap().push(data);
                    }
                }
            });

            let files = path_helper::get_dirs(&path);

            files.par_iter().for_each(|item| {
                let meta = item.join(names::PACK_META_FILE);
                if !meta.exists() || !meta.is_file() {
                    return;
                }
                let stream = path_helper::open_read(meta);
                if let Err(err) = stream {
                    mcml_log::error_type(err);
                    return;
                }
                let json = serde_json::from_reader::<_, Value>(stream.unwrap());
                if let Err(err) = json {
                    mcml_log::error_type(ErrorType::SerializerError(ErrorData {
                        error: err.to_string(),
                    }));
                    return;
                }
                let json = json.unwrap();
                if let Some(data) = read_pack(path.clone(), ens, dis, json) {
                    list.lock().unwrap().push(data);
                }
            });
        }

        Ok(list.into_inner().unwrap())
    }

    /// 修改数据包启用状态
    pub fn change_data_pack(&mut self, list: &Vec<SaveDataPackObj>) -> CoreResult<()> {
        let Some(data) = self
            .nbt
            .nbt
            .as_compound_mut()
            .and_then(|map| map.get_compound_mut("Data"))
        else {
            return Err(ErrorType::InfoNotFound("Data".to_string()));
        };
        let Some(data_packs) = data.get_compound_mut("DataPacks") else {
            return Err(ErrorType::InfoNotFound("DataPacks".to_string()));
        };

        // Collect info with immutable access first
        let mut remove_from_enabled: Vec<usize> = Vec::new();
        let mut remove_from_disabled: Vec<usize> = Vec::new();
        let mut add_to_enabled: Vec<String> = Vec::new();
        let mut add_to_disabled: Vec<String> = Vec::new();

        {
            let Some(ens) = data_packs.get_list("Enabled") else {
                return Err(ErrorType::InfoNotFound("Enabled".to_string()));
            };
            let Some(dis) = data_packs.get_list("Disabled") else {
                return Err(ErrorType::InfoNotFound("Disabled".to_string()));
            };

            for item in list.iter() {
                let nbt_enable = (0..ens.len()).find(|&index| {
                    ens.get_item(index)
                        .and_then(|data| data.as_string())
                        .map(|s| s.data.eq_ignore_ascii_case(&item.name))
                        .unwrap_or(false)
                });

                let nbt_disable = (0..dis.len()).find(|&index| {
                    dis.get_item(index)
                        .and_then(|data| data.as_string())
                        .map(|s| s.data.eq_ignore_ascii_case(&item.name))
                        .unwrap_or(false)
                });

                if nbt_enable.is_some() && nbt_disable.is_some() {
                    remove_from_disabled.push(nbt_disable.unwrap());
                } else if nbt_enable.is_some() && nbt_disable.is_none() {
                    remove_from_enabled.push(nbt_enable.unwrap());
                    add_to_disabled.push(item.name.clone());
                } else if nbt_disable.is_some() {
                    remove_from_disabled.push(nbt_disable.unwrap());
                } else {
                    add_to_enabled.push(item.name.clone());
                }
            }
        } // immutable refs dropped here

        // Apply mutations — one list at a time to avoid double mutable borrow
        {
            let Some(ens) = data_packs.get_list_mut("Enabled") else {
                return Err(ErrorType::InfoNotFound("Enabled".to_string()));
            };
            remove_from_enabled.sort_by(|a, b| b.cmp(a));
            for idx in remove_from_enabled {
                ens.remove(idx);
            }
            for name in add_to_enabled {
                ens.add_item(NbtString::new(name).to_nbt());
            }
        }
        {
            let Some(dis) = data_packs.get_list_mut("Disabled") else {
                return Err(ErrorType::InfoNotFound("Disabled".to_string()));
            };
            remove_from_disabled.sort_by(|a, b| b.cmp(a));
            for idx in remove_from_disabled {
                dis.remove(idx);
            }
            for name in add_to_disabled {
                dis.add_item(NbtString::new(name).to_nbt());
            }
        }

        self.save_nbt()?;

        Ok(())
    }

    /// 删除数据包
    pub fn delete_datapack(&mut self, list: &Vec<SaveDataPackObj>) -> CoreResult<()> {
        let Some(data) = self
            .nbt
            .nbt
            .as_compound_mut()
            .and_then(|map| map.get_compound_mut("Data"))
        else {
            return Err(ErrorType::InfoNotFound("Data".to_string()));
        };
        let Some(data_packs) = data.get_compound_mut("DataPacks") else {
            return Err(ErrorType::InfoNotFound("DataPacks".to_string()));
        };

        // Collect indices to remove with immutable access first
        let mut remove_from_enabled: Vec<usize> = Vec::new();
        let mut remove_from_disabled: Vec<usize> = Vec::new();

        {
            let Some(ens) = data_packs.get_list("Enabled") else {
                return Err(ErrorType::InfoNotFound("Enabled".to_string()));
            };
            let Some(dis) = data_packs.get_list("Disabled") else {
                return Err(ErrorType::InfoNotFound("Disabled".to_string()));
            };

            for item in list.iter() {
                if let Some(idx) = (0..ens.len()).find(|&index| {
                    ens.get_item(index)
                        .and_then(|data| data.as_string())
                        .map(|s| s.data.eq_ignore_ascii_case(&item.name))
                        .unwrap_or(false)
                }) {
                    remove_from_enabled.push(idx);
                }

                if let Some(idx) = (0..dis.len()).find(|&index| {
                    dis.get_item(index)
                        .and_then(|data| data.as_string())
                        .map(|s| s.data.eq_ignore_ascii_case(&item.name))
                        .unwrap_or(false)
                }) {
                    remove_from_disabled.push(idx);
                }
            }
        } // immutable refs dropped here

        // Apply removals — one list at a time, descending order
        if !remove_from_enabled.is_empty() {
            let Some(ens) = data_packs.get_list_mut("Enabled") else {
                return Err(ErrorType::InfoNotFound("Enabled".to_string()));
            };
            remove_from_enabled.sort_by(|a, b| b.cmp(a));
            for idx in remove_from_enabled {
                ens.remove(idx);
            }
        }

        if !remove_from_disabled.is_empty() {
            let Some(dis) = data_packs.get_list_mut("Disabled") else {
                return Err(ErrorType::InfoNotFound("Disabled".to_string()));
            };
            remove_from_disabled.sort_by(|a, b| b.cmp(a));
            for idx in remove_from_disabled {
                dis.remove(idx);
            }
        }

        self.save_nbt()?;

        Ok(())
    }
}

fn read_pack(
    path: PathBuf,
    ens: Option<&NbtList>,
    dis: Option<&NbtList>,
    data: Value,
) -> Option<SaveDataPackObj> {
    if let Some(pack) = data.as_object().and_then(|map| map.get("pack")) {
        let mut obj = SaveDataPackObj {
            name: format!(
                "file/{}",
                path.file_name().unwrap_or_default().to_string_lossy()
            ),
            path,
            description: pack
                .as_object()
                .and_then(|map| map.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            pack_format: pack
                .as_object()
                .and_then(|map| map.get("pack_format"))
                .and_then(|v| v.as_i64())
                .unwrap_or(-1),
            enable: None,
        };

        if let Some(ens) = ens {
            for item in ens.iter() {
                if let Some(name) = item.as_string()
                    && name.data.eq_ignore_ascii_case(&obj.name)
                {
                    obj.enable = Some(true);
                    break;
                }
            }
        }

        if let Some(dis) = dis {
            for item in dis.iter() {
                if let Some(name) = item.as_string()
                    && name.data.eq_ignore_ascii_case(&obj.name)
                {
                    obj.enable = Some(false);
                    break;
                }
            }
        }

        Some(obj)
    } else {
        None
    }
}
