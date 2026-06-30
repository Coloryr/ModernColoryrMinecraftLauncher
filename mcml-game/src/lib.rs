use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, LazyLock, RwLock},
    thread,
    time::Duration,
};

use mcml_base::events::Events;
use mcml_names::{i18_items::error_type::CoreResult, names};

use uuid::Uuid;

use crate::{
    game_launch::InstanceHandle,
    game_log::{GameLog, InstanceRuntimeLog},
    launcher::game_setting_obj::GameSettingObj,
    launcher_path::instance_path,
};

pub mod game_arg;
pub mod game_check;
pub mod game_download;
pub mod game_launch;
pub mod game_libraries;
pub mod game_log;
pub mod game_saves;
pub mod game_server;
pub mod launcher;
pub mod launcher_path;
pub mod loader;
pub mod mojang;
pub mod path_watch;

type InstanceExitHandler = Box<dyn Fn(Uuid, i32) + Send + Sync + 'static>;
type InstanceChangeHandler = Box<dyn Fn(&InstanceChange) + Send + Sync + 'static>;

static RUNTIME_LOGS: LazyLock<RwLock<HashMap<Uuid, InstanceRuntimeLog>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static HANDELS: LazyLock<RwLock<HashMap<Uuid, InstanceHandle>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static GROUPS: LazyLock<RwLock<HashMap<String, Vec<Uuid>>>> = LazyLock::new(|| {
    let mut group = HashMap::new();
    group.insert(names::DEFAULT_GROUP.to_string(), Vec::new());
    RwLock::new(group)
});

static INSTANCES: LazyLock<RwLock<HashMap<Uuid, Arc<GameSettingObj>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static EXIT_EVENT: Events<InstanceExitHandler> = Events::new();
static CHANGE_EVENT: Events<InstanceChangeHandler> = Events::new();

pub enum InstanceChange {
    AddInstance(Uuid),
    RemoveInstance(Uuid),
}

pub fn add_game_exit_handler<F>(handler: F)
where
    F: Fn(Uuid, i32) + Send + Sync + 'static,
{
    EXIT_EVENT.add(Box::new(handler));
}

pub fn add_game_change_handler<F>(handler: F)
where
    F: Fn(&InstanceChange) + Send + Sync + 'static,
{
    CHANGE_EVENT.add(Box::new(handler));
}

pub fn invoke_game_exit(uuid: Uuid, code: i32) {
    EXIT_EVENT.for_each(|handler| handler(uuid, code));
}

pub fn invoke_game_change(change: InstanceChange) {
    CHANGE_EVENT.for_each(|handler| handler(&change));
}

/// 初始化
/// - `dir`: 运行路径
pub fn init<P: AsRef<Path>>(dir: P) -> CoreResult<()> {
    launcher_path::init(dir)?;
    path_watch::init_watch()?;

    thread::spawn(|| {
        loop {
            let mut write = HANDELS.write().unwrap();

            let removed: Vec<_> = write
                .extract_if(|_key, value| {
                    value.tick();
                    value.is_exit()
                })
                .collect();

            for (uuid, handel) in removed {
                invoke_game_exit(uuid, handel.code());
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

/// 添加运行日志
pub(crate) fn add_game_log(uuid: &Uuid, data: &str) {
    let mut logs = RUNTIME_LOGS.write().unwrap();
    if let Some(log) = logs.get_mut(uuid) {
        log.add_game_log(&data);
    } else {
        let mut log = InstanceRuntimeLog::new();
        log.add_game_log(&data);

        logs.insert(uuid.clone(), log);
    }
}

/// 添加运行日志
pub(crate) fn add_game_log_item(uuid: &Uuid, data: GameLog) {
    let mut logs = RUNTIME_LOGS.write().unwrap();
    if let Some(log) = logs.get_mut(uuid) {
        log.add_log_item(data);
    } else {
        let mut log = InstanceRuntimeLog::new();
        log.add_log_item(data);

        logs.insert(uuid.clone(), log);
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
}

/// 添加启动的游戏实例
pub(crate) fn add_run_game(handel: InstanceHandle) {
    let mut games = HANDELS.write().unwrap();
    games.insert(handel.uuid, handel);
}

/// 获取所有实例
pub fn get_instances() -> Vec<Arc<GameSettingObj>> {
    let mut list = Vec::new();

    for (_, value) in INSTANCES.read().unwrap().iter() {
        list.push(value.clone());
    }

    list
}

/// 从uuid获取实例
pub fn get_instance(uuid: &Uuid) -> Option<Arc<GameSettingObj>> {
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
pub fn get_group(key: &str) -> Vec<Arc<GameSettingObj>> {
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

/// 将实例添加到分组中
fn add_to_group(mut obj: GameSettingObj) {
    while obj.uuid.is_nil() || matches!(get_instance(&obj.uuid), Some(_)) {
        obj.uuid = Uuid::new_v4();
    }

    obj.save();
    let game = Arc::new(obj);
    INSTANCES.write().unwrap().insert(game.uuid, game.clone());
    if let Some(group) = &game.group {
        let mut groups = GROUPS.write().unwrap();
        if let Some(group) = groups.get_mut(group) {
            group.push(game.uuid);
        } else {
            let mut list = Vec::new();
            list.push(game.uuid);
            groups.insert(group.clone(), list);
        }
    } else {
        let mut groups = GROUPS.write().unwrap();
        if let Some(group) = groups.get_mut(names::DEFAULT_GROUP) {
            group.push(game.uuid);
        }
    }

    invoke_game_change(InstanceChange::AddInstance(game.uuid));
}

/// 将实例从分组中删除
fn remove_from_group(uuid: &Uuid) {
    let mut group = GROUPS.write().unwrap();
    for (_, value) in group.iter_mut() {
        value.retain(|item| item.eq(uuid));
    }

    group.retain(|_, value| value.is_empty());

    let mut games = INSTANCES.write().unwrap();
    games.remove(uuid);

    invoke_game_change(InstanceChange::RemoveInstance(uuid.clone()));
}

impl GameSettingObj {
    pub fn create_instance(self) -> Arc<GameSettingObj> {
        todo!()
    }
}

//
// pub fn create_instance(obj: GameSettingObj) -> Arc<GameSettingObj> {
//     Arc::new(data)
// }
