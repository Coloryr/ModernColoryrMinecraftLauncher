use std::{
    collections::HashMap,
    fs::{self},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use mcml_base::{
    path_helper::{self},
    serialize_tools,
};
use mcml_config::config_save;
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorType, FileSystemErrorData},
    names, uuids,
};

use crate::launcher::{
    custom_game_arg_obj::CustomGameArgObj, file_online_info_obj::FileOnlineInfoObj,
    game_time_obj::GameTimeObj, instance_setting_obj::InstanceSettingObj,
};

static BASE_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 初始化版本路径
/// - `dir`: 运行路径
pub(crate) fn init<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    let dir = BASE_DIR.get_or_init(|| dir.as_ref().join(names::INSTANCE_DIR));

    if !dir.exists() {
        path_helper::create_dir_all(&dir)?;
    }

    Ok(())
}

/// 获取实例目录
pub fn get_instance_dir() -> PathBuf {
    BASE_DIR.get().unwrap().clone()
}

/// 读取所有实例
pub(crate) fn load_instance_dir() -> CoreResult<Vec<InstanceSettingObj>> {
    let dir = BASE_DIR.get().unwrap();
    let dirs = fs::read_dir(dir).map_err(|err| {
        ErrorType::FileSystemError(FileSystemErrorData {
            path: dir.clone(),
            error: err.to_string(),
        })
    })?;

    let mut list = Vec::new();

    for item in dirs {
        if let Ok(dir) = item
            && let Some(instance) = load_instance(dir.path())
        {
            list.push(instance);
        }
    }

    Ok(list)
}

/// 从文件夹路径加载实例
/// - `dir`: 路径
pub(crate) fn load_instance<P: AsRef<Path>>(dir: P) -> Option<InstanceSettingObj> {
    let file = dir.as_ref();
    if !file.is_dir() {
        return None;
    }

    let config = file.join(names::GAME_FILE);
    if !config.exists() || !config.is_file() {
        return None;
    }

    if let Ok(mut obj) = serialize_tools::json_from_file::<InstanceSettingObj>(&config) {
        let path = file.file_name().unwrap_or_default();
        let path = path.to_string_lossy();
        if !path.eq(&obj.dir) {
            obj.dir = path.to_string();
            obj.save();
        }

        Some(obj)
    } else {
        None
    }
}

impl InstanceSettingObj {
    /// 保存
    pub fn save(&self) {
        config_save::save(self.uuid, self, &self.get_json_file());
    }

    /// 获取基础路径
    pub fn get_base_path(&self) -> PathBuf {
        BASE_DIR.get().unwrap().join(&self.dir)
    }

    /// 获取存档备份路径
    pub fn get_backup_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::BACKUP_DIR)
    }

    /// 获取存档备份路径
    pub fn get_temp_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::TEMP_DIR)
    }

    /// 获取游戏缓存路径
    pub fn get_cache_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::CACHE_DIR)
    }

    /// 获取自定义加载器路径
    pub fn get_json_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::JSON_DIR)
    }

    /// 获取自定义加载器运行库路径
    pub fn get_libraries_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::LIBRARIES_DIR)
    }

    /// 获取游戏路径 .minecraft
    pub fn get_game_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
    }

    /// 获取截图路径
    pub fn get_screenshots_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::GAME_SCREENSHOTS_DIR)
    }

    /// 获取资源包路径
    pub fn get_resourcepacks_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::GAME_RESOURCEPACKS_DIR)
    }

    /// 获取光影包路径
    pub fn get_shaderpacks_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::GAME_SHADERPACKS_DIR)
    }

    /// 获取模组路径
    pub fn get_mods_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::GAME_MODS_DIR)
    }

    /// 获取存档路径
    pub fn get_saves_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::GAME_SAVES_DIR)
    }

    /// 获取结构文件路径
    pub fn get_schematics_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::GAME_SCHEMATICS_DIR)
    }

    /// 获取配置文件路径
    pub fn get_config_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::GAME_CONFIG_DIR)
    }

    /// 获取日志路径
    pub fn get_logs_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::GAME_LOGS_DIR)
    }

    /// 获取崩溃日志路径
    pub fn get_crash_path(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::GAME_CRASH_DIR)
    }

    /// 获取储存文件
    pub fn get_json_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_FILE)
    }

    /// 获取图标文件
    pub fn get_icon_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::ICON_FILE)
    }

    /// 获取存档备份信息文件
    pub fn get_backup_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::SAVE_BACKUP_FILE)
    }

    /// 获取在线文件信息文件
    pub fn get_online_info_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::MOD_INFO_FILE)
    }

    /// 获取服务器实例文件
    pub fn get_server_pack_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::SERVER_FILE)
    }

    /// 获取旧服务器实例文件
    pub fn get_server_pack_old_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::SERVER_OLD_FILE)
    }

    /// 获取启动记录数据文件
    pub fn get_launch_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::LAUNCH_COUNT_FILE)
    }

    /// 获取安全Log4j文件
    pub fn get_log4j_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::LOG4J_FILE)
    }

    /// 获取自定义加载器文件
    pub fn get_loader_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::LOADER_FILE)
    }

    /// 获取游戏配置文件
    pub fn get_option_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::OPTION_FILE)
    }

    /// 获取服务器储存文件
    pub fn get_servers_file(&self) -> PathBuf {
        BASE_DIR
            .get()
            .unwrap()
            .join(&self.dir)
            .join(names::GAME_DIR)
            .join(names::GAME_SERVER_FILE)
    }

    /// 读取在线文件信息
    pub fn read_online_info(&self) -> HashMap<String, FileOnlineInfoObj> {
        let file = self.get_online_info_file();
        if file.exists() && file.is_file() {
            let json = serialize_tools::json_from_file::<HashMap<String, FileOnlineInfoObj>>(&file);
            if let Ok(data) = json {
                return data;
            }
        }

        HashMap::new()
    }

    /// 保存在线文件信息
    pub fn save_online_info(&self, info: &HashMap<String, FileOnlineInfoObj>) {
        config_save::save(
            uuids::mix_uuid(self.uuid, uuids::ONLINE_FILE_UUID),
            info,
            &self.get_online_info_file(),
        );
    }

    /// 读取自定义游戏启动配置
    pub fn read_custom_json(&self) -> HashMap<String, CustomGameArgObj> {
        let file = self.get_json_path();
        let mut list = HashMap::new();
        if file.exists()
            && file.is_dir()
            && let Ok(dir) = fs::read_dir(&file)
        {
            for item in dir {
                if let Ok(item) = item {
                    let json = serialize_tools::json_from_file::<CustomGameArgObj>(&item.path());
                    if let Ok(data) = json {
                        list.insert(item.file_name().to_string_lossy().to_string(), data);
                    }
                }
            }
        }

        list
    }

    /// 保存自定义启动配置
    pub fn save_custom_json(&self, info: &HashMap<String, CustomGameArgObj>) -> CoreResult<()> {
        let dir = self.get_json_path();
        path_helper::create_dir_all(&dir)?;
        for (key, value) in info.iter() {
            serialize_tools::json_to_file(value, dir.join(key))?;
        }

        Ok(())
    }

    /// 读取启动统计数据
    pub fn read_launch_count_data(&self) -> GameTimeObj {
        let file = self.get_launch_file();
        if file.exists() && file.is_file() {
            let obj = serialize_tools::json_from_file::<GameTimeObj>(&file);
            if let Ok(obj) = obj {
                return obj;
            }
        }

        Default::default()
    }

    /// 保存启动统计数据
    pub fn save_launch_count_data(&self, obj: &GameTimeObj) {
        config_save::save(
            uuids::mix_uuid(self.uuid, uuids::LAUNCH_COUNT_DATA_FILE_UUID),
            obj,
            &self.get_launch_file(),
        );
    }
}
