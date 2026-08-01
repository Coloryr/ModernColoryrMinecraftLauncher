use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, RwLock},
    thread,
    time::Duration,
};

use mcml_base::{Os, events::EventArgHandler, path_helper};
use mcml_names::{
    i18_items::error_type::{ArgEmptyData, CoreResult, ErrorType, FileSystemErrorData},
    names,
};

use mcml_net::input_file::InputFile;
use uuid::Uuid;

use crate::{
    game_launch::InstanceHandle,
    game_log::{GameLog, GameLogItemObj, InstanceRuntimeLog},
    gui_hook::{IAddInstanceGui, ICopyGui},
    launcher::{LogEncoding, game_time_obj::GameTimeObj, instance_setting_obj::InstanceSettingObj},
    launcher_path::instance_path::{self, CustomGameArgList, OnlineInfoList},
};

pub mod add_game;
pub mod class_scan;
pub mod curseforge;
pub mod data_res;
pub mod game_arg;
pub mod game_check;
pub mod game_download;
pub mod game_export;
pub mod game_lan;
pub mod game_launch;
pub mod game_libraries;
pub mod game_log;
pub mod game_mods;
pub mod game_options;
pub mod game_resourcepacks;
pub mod game_saves;
pub mod game_schematics;
pub mod game_screenshots;
pub mod game_server;
pub mod game_shaderpacks;
pub mod gui_hook;
pub mod launcher;
pub mod launcher_path;
pub mod loader;
pub mod modpack;
pub mod modrinth;
pub mod mojang;
pub mod path_watch;

pub type GameInstance = Arc<RwLock<InstanceSettingObj>>;

/// 实例结束运行事件
pub struct InstanceExit {
    pub uuid: Uuid,
    pub code: i32,
}

pub enum InstanceChange {
    AddInstance(Uuid),
    RemoveInstance(Uuid),
    MoveGroup(Uuid, Option<String>),
}

pub enum LogType {
    AddLog(GameLogItemObj),
    ClearLog,
}

pub struct InstanceLog {
    pub uuid: Uuid,
    pub log: LogType,
}

pub struct InstanceData {
    pub instance: GameInstance,
    pub online: OnlineInfoList,
    pub custom: CustomGameArgList,
}

static RUNTIME_LOGS: LazyLock<RwLock<HashMap<Uuid, InstanceRuntimeLog>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static HANDELS: LazyLock<RwLock<HashMap<Uuid, InstanceHandle>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static GROUPS: LazyLock<RwLock<HashMap<String, Vec<Uuid>>>> = LazyLock::new(|| {
    let mut group = HashMap::new();
    group.insert(names::DEFAULT_GROUP.to_string(), Vec::new());
    RwLock::new(group)
});

static INSTANCES: LazyLock<RwLock<HashMap<Uuid, GameInstance>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static EXIT_EVENT: LazyLock<EventArgHandler<InstanceExit>> =
    LazyLock::new(|| EventArgHandler::new());
static CHANGE_EVENT: LazyLock<EventArgHandler<InstanceChange>> =
    LazyLock::new(|| EventArgHandler::new());
static LOG_EVENT: LazyLock<EventArgHandler<InstanceLog>> = LazyLock::new(|| EventArgHandler::new());

pub fn add_exit<F>(handler: F) -> u64
where
    F: Fn(&InstanceExit) + Send + Sync + 'static,
{
    EXIT_EVENT.add_handler(handler)
}

pub fn add_change<F>(handler: F) -> u64
where
    F: Fn(&InstanceChange) + Send + Sync + 'static,
{
    CHANGE_EVENT.add_handler(handler)
}

pub fn add_run_log<F>(handler: F) -> u64
where
    F: Fn(&InstanceLog) + Send + Sync + 'static,
{
    LOG_EVENT.add_handler(handler)
}

pub fn remove_exit(id: u64) {
    EXIT_EVENT.remove_handel(id);
}

pub fn remove_change(id: u64) {
    CHANGE_EVENT.remove_handel(id);
}

pub fn remove_run_log(id: u64) {
    LOG_EVENT.remove_handel(id);
}

pub(crate) fn invoke_exit(uuid: Uuid, code: i32) {
    EXIT_EVENT.emit(InstanceExit { uuid, code });
}

pub(crate) fn invoke_change(change: InstanceChange) {
    CHANGE_EVENT.emit(change);
}

pub(crate) fn invoke_run_log(uuid: Uuid, log: LogType) {
    LOG_EVENT.emit(InstanceLog { uuid, log });
}

/// 初始化
/// - `dir`: 运行路径
pub fn init<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    launcher_path::init(dir)?;
    path_watch::init_watch()?;

    thread::spawn(|| {
        loop {
            let to_remove: Vec<(Uuid, i32)> = {
                let handels = HANDELS.read().unwrap();
                handels
                    .iter()
                    .filter_map(|(k, v)| {
                        v.tick();
                        if v.is_exit() {
                            Some((*k, v.code()))
                        } else {
                            None
                        }
                    })
                    .collect()
            };

            if !to_remove.is_empty() {
                {
                    let mut handels = HANDELS.write().unwrap();
                    for (uuid, _) in &to_remove {
                        handels.remove(uuid);
                    }
                }
                for (uuid, code) in &to_remove {
                    invoke_exit(*uuid, *code);
                }
            }

            thread::sleep(Duration::from_secs(1));
        }
    });

    Ok(())
}

/// 开始加载数据
pub fn load() -> CoreResult<()> {
    let installs = instance_path::load_instance_dir()?;

    for item in installs {
        add_to_group(item);
    }

    Ok(())
}

/// 获取所有实例
pub fn get_instances() -> Vec<GameInstance> {
    let mut list = Vec::new();

    for (_, value) in INSTANCES.read().unwrap().iter() {
        list.push(value.clone());
    }

    list
}

/// 从uuid获取实例
pub fn get_instance(uuid: &Uuid) -> Option<GameInstance> {
    let list = INSTANCES.read().unwrap();

    Some(list.get(uuid)?.clone())
}

/// 获取所有分组名字
pub fn get_group_keys() -> Vec<String> {
    let mut list = Vec::new();

    for (key, _) in GROUPS.read().unwrap().iter() {
        list.push(key.clone());
    }

    list
}

/// 从分组名字获取对应的实例
/// - `key`: 分组名字
pub fn get_group(key: &str) -> Vec<GameInstance> {
    let mut list = Vec::new();

    let group = GROUPS.read().unwrap();
    if let Some(group) = group.get(key) {
        for item in group {
            if let Some(instance) = get_instance(item) {
                list.push(instance.clone());
            }
        }
    }

    list
}

/// 添加分组
/// - `name`: 分组名
pub fn add_group(name: &str) -> bool {
    let mut groups = GROUPS.write().unwrap();
    if groups.contains_key(name) {
        false
    } else {
        groups.insert(name.to_string(), Vec::new());
        true
    }
}

/// 删除分组
/// - `name`: 分组名
pub fn remove_group(name: &str) -> bool {
    let mut groups = GROUPS.write().unwrap();
    let items = groups.remove(name);
    match items {
        Some(items) => {
            let def = groups.get_mut(names::DEFAULT_GROUP).unwrap();
            def.extend(items);
            true
        }
        None => false,
    }
}

/// 移动分组
/// - `list`: 需要移动的列表
/// - `new`: 新分组名字
pub fn move_group(list: Vec<Uuid>, new: Option<String>) {
    let mut changes: Vec<(Uuid, Option<String>)> = Vec::new();

    {
        let mut groups = GROUPS.write().unwrap();
        let instances = INSTANCES.read().unwrap();

        for item in list.iter() {
            let game_arc = match instances.get(item) {
                Some(game) => game.clone(),
                None => continue,
            };

            let mut game = game_arc.write().unwrap();

            if let Some(name) = &game.group {
                let group = groups.get_mut(name);
                if let Some(group) = group {
                    group.retain(|g| g != &game.uuid);
                }
            } else {
                let group = groups.get_mut(names::DEFAULT_GROUP).unwrap();
                group.retain(|g| g != &game.uuid);
            }

            match new {
                Some(ref name) => {
                    if !groups.contains_key(name) {
                        groups.insert(name.to_string(), Vec::new());
                    }
                    let group = groups.get_mut(name).unwrap();
                    group.push(game.uuid);
                }
                None => {
                    let group = groups.get_mut(names::DEFAULT_GROUP).unwrap();
                    group.push(game.uuid);
                }
            }

            game.group = new.clone();
            changes.push((game.uuid, game.group.clone()));
        }
    }

    for (uuid, group) in changes {
        if let Some(instance) = get_instance(&uuid) {
            instance.write().unwrap().save();
        }
        invoke_change(InstanceChange::MoveGroup(uuid, group));
    }
}

/// 从实例名字获取实例
/// - `name`: 实例名字
pub fn get_instance_by_name(name: &str) -> Option<GameInstance> {
    let list = INSTANCES.read().unwrap();
    let temp = list
        .iter()
        .filter(|(_, value)| value.read().unwrap().name.eq_ignore_ascii_case(name))
        .next()?;

    Some(temp.1.clone())
}

/// 是否存在这个名字的实例
/// - `name`: 实例名字
pub fn have_instance_name(name: &str) -> bool {
    let list = INSTANCES.read().unwrap();
    let temp = list
        .iter()
        .filter(|(_, value)| value.read().unwrap().name.eq_ignore_ascii_case(name))
        .next();
    !temp.is_none()
}

/// 将实例添加到分组中
fn add_to_group(mut obj: InstanceSettingObj) -> GameInstance {
    while obj.uuid.is_nil() || matches!(get_instance(&obj.uuid), Some(_)) {
        obj.uuid = Uuid::new_v4();
    }

    obj.save();
    let key = obj.uuid.clone();
    let group = obj.group.clone();
    let game: Arc<RwLock<InstanceSettingObj>> = Arc::new(RwLock::new(obj));

    let mut groups = GROUPS.write().unwrap();
    if let Some(ref g) = group {
        if !groups.contains_key(g) {
            groups.insert(g.clone(), Vec::new());
        }
    } else {
        if !groups.contains_key(names::DEFAULT_GROUP) {
            groups.insert(names::DEFAULT_GROUP.to_string(), Vec::new());
        }
    }

    let mut instances = INSTANCES.write().unwrap();
    instances.insert(key, game.clone());

    // ---- 更新分组列表 ----
    if let Some(ref g) = group {
        if let Some(list) = groups.get_mut(g) {
            list.push(key);
        }
    } else {
        if let Some(list) = groups.get_mut(names::DEFAULT_GROUP) {
            list.push(key);
        }
    }

    drop(groups);
    drop(instances);

    invoke_change(InstanceChange::AddInstance(key));
    game
}

/// 删除实例
pub fn delete_instance(uuid: &Uuid) -> CoreResult<()> {
    let instance_opt = {
        let mut groups = GROUPS.write().unwrap();
        let mut instances = INSTANCES.write().unwrap();

        // 从所有分组中移除 uuid
        for (_, list) in groups.iter_mut() {
            list.retain(|&u| u != *uuid);
        }
        groups.retain(|_, list| !list.is_empty());
        instances.remove(uuid)
    };

    if let Some(instance) = instance_opt {
        invoke_change(InstanceChange::RemoveInstance(*uuid));
        instance.read().unwrap().delete_files()?;
        Ok(())
    } else {
        Err(ErrorType::ArgEmpty(ArgEmptyData {
            arg: "uuid".to_string(),
        }))
    }
}

/// 添加运行日志
pub(crate) fn add_game_log(uuid: &Uuid, data: &str) {
    let mut logs = RUNTIME_LOGS.write().unwrap();
    if let Some(log) = logs.get_mut(uuid) {
        log.add_game_log(&data);
    } else {
        let mut log = InstanceRuntimeLog::new();
        let item = log.add_game_log(&data);

        logs.insert(uuid.clone(), log);

        invoke_run_log(uuid.clone(), LogType::AddLog(item));
    }
}

/// 添加运行日志
pub(crate) fn add_game_log_item(uuid: &Uuid, data: GameLog) {
    let mut logs = RUNTIME_LOGS.write().unwrap();
    if let Some(log) = logs.get_mut(uuid) {
        log.add_log_item(data);
    } else {
        let mut log = InstanceRuntimeLog::new();
        let item = log.add_log_item(data);

        logs.insert(uuid.clone(), log);
        invoke_run_log(uuid.clone(), LogType::AddLog(item));
    }
}

/// 清理日志
pub(crate) fn clear_game_log(uuid: &Uuid) {
    let mut logs = RUNTIME_LOGS.write().unwrap();
    if let Some(log) = logs.get_mut(uuid) {
        log.clear();
    } else {
        let log = InstanceRuntimeLog::new();
        logs.insert(uuid.clone(), log);
    }

    invoke_run_log(uuid.clone(), LogType::ClearLog);
}

/// 添加启动的游戏实例
pub(crate) fn add_run_game(handel: InstanceHandle) {
    let mut games = HANDELS.write().unwrap();
    games.insert(handel.uuid, handel);
}

impl InstanceSettingObj {
    /// 创建实例
    pub async fn create_instance(
        mut self,
        gui: &Option<Arc<dyn IAddInstanceGui>>,
    ) -> CoreResult<GameInstance> {
        path_watch::stop_watch();

        let old = get_instance_by_name(&self.name);
        if let Some(instance) = &old {
            if let Some(gui) = gui {
                let over = gui.overwrite(instance.clone()).await;
                if !over && !gui.name_replace(&self.name).await {
                    return Err(ErrorType::TaskCancel);
                }
            }

            let mut a = 1;
            let mut name = format!("{}{a}", self.name);
            while have_instance_name(&name) {
                a += 1;
                name = format!("{}{a}", self.name);
            }

            self.name = name;
        }

        if self.name.is_empty() {
            return Err(ErrorType::ArgEmpty(ArgEmptyData {
                arg: "name".to_string(),
            }));
        }

        if let Some(instance) = old {
            let uuid = {
                let r = instance.read().unwrap();
                r.uuid
            };

            delete_instance(&uuid)?;
        }

        self.dir = path_helper::replace_path_name(&self.name);

        let dir = self.get_base_path();
        if dir.exists() {
            path_helper::move_to_trash(&dir)?
        }

        path_helper::create_dir_all(dir)?;
        path_helper::create_dir_all(self.get_game_path())?;
        path_helper::create_dir_all(self.get_mods_path())?;
        path_helper::create_dir_all(self.get_config_path())?;
        path_helper::create_dir_all(self.get_logs_path())?;
        path_helper::create_dir_all(self.get_saves_path())?;
        path_helper::create_dir_all(self.get_resourcepacks_path())?;

        self.save_online_info(&HashMap::new());
        self.save_launch_count_data(&GameTimeObj::new());

        if mcml_base::get_system_info().os == Os::Windows {
            self.encoding = LogEncoding::GBK;
        }

        self.save();

        path_watch::start_watch();

        Ok(add_to_group(self))
    }

    /// 删除实例文件
    pub fn delete_files(&self) -> CoreResult<()> {
        path_helper::move_to_trash(self.get_base_path())
    }

    /// 复制数据到新的实例
    /// - `name`: 新的实例名字
    pub async fn copy_to_other(
        &self,
        name: &str,
        gui: &Option<Arc<dyn IAddInstanceGui>>,
    ) -> CoreResult<GameInstance> {
        let mut instance = self.clone();
        instance.name = name.to_string();
        let instance = instance.create_instance(gui).await?;

        let online = self.read_online_info();
        let custom = self.read_custom_json();

        let read = instance.read().unwrap();
        read.save_custom_json(&custom)?;
        read.save_online_info(&online);

        Ok(instance.clone())
    }

    /// 更新在线文件信息
    pub async fn update_online(&self) {
        let mut online = self.read_online_info();
        let dir = self.get_game_path();
        online.retain(|_, value| {
            let file = dir.join(&value.path).join(&value.file);
            if file.exists() {
                false
            } else {
                if let Some(ext) = file.extension()
                    && ext.eq_ignore_ascii_case(names::JAR_EXT)
                {
                    let disabled_file =
                        PathBuf::from(format!("{}{}", file.display(), names::DISABLED_DOT_EXT));
                    if disabled_file.exists() {
                        return false;
                    }
                }
                true
            }
        });

        self.save_online_info(&online);
    }

    /// 将文件复制到其他地方
    pub async fn copy_files<P: AsRef<Path>>(
        &self,
        path: P,
        skip: Option<Vec<PathBuf>>,
        is_base: bool,
        gui: &Option<impl ICopyGui>,
    ) -> CoreResult<()> {
        let dir = if is_base {
            self.get_base_path()
        } else {
            self.get_game_path()
        };

        path_helper::create_dir_all(path.as_ref())?;

        let mut index = 0usize;
        let list = path_helper::get_all_files(&dir);
        if let Some(gui) = gui {
            gui.update(index, list.len());
        }
        for item in list.iter() {
            let file = item.strip_prefix(&dir).map_err(|err| {
                ErrorType::FileSystemError(FileSystemErrorData {
                    path: item.clone(),
                    error: err.to_string(),
                })
            })?;

            if let Some(skip) = &skip {
                if skip.contains(&file.to_path_buf()) {
                    index += 1;
                    if let Some(gui) = gui {
                        gui.update(index, list.len());
                    }
                    continue;
                }
            }

            if let Some(gui) = gui {
                gui.file(item.clone());
            }

            let now = path.as_ref().join(file);
            path_helper::copy_file_async(item, &now).await?;
        }

        Ok(())
    }

    /// 设置图标
    pub async fn set_icon(&mut self, icon: InputFile) -> CoreResult<()> {
        let file = self.get_icon_file();
        icon.save_file(file).await
    }

    /// 获取运行中的日志
    pub fn get_runtime_log(&self) -> Option<Arc<VecDeque<GameLogItemObj>>> {
        let logs = RUNTIME_LOGS.read().unwrap();
        logs.get(&self.uuid)
            .map(|log| log.logs.read().unwrap().clone())
    }
}
