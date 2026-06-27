use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use uuid::Uuid;

use crate::game_log::{GameLog, GameRuntimeLog};

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

static GAME_RUNTIME_LOG: LazyLock<RwLock<HashMap<Uuid, GameRuntimeLog>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 添加运行日志
pub(crate) fn add_game_log(uuid: Uuid, data: &str) {
    let mut logs = GAME_RUNTIME_LOG.write().unwrap();
    if let Some(log) = logs.get_mut(&uuid) {
        log.add_game_log(&data);
    } else {
        let mut log = GameRuntimeLog::new();
        log.add_game_log(&data);

        logs.insert(uuid.clone(), log);
    }
}

/// 添加运行日志
pub(crate) fn add_game_log_item(uuid: &Uuid, data: GameLog) {
    let mut logs = GAME_RUNTIME_LOG.write().unwrap();
    if let Some(log) = logs.get_mut(&uuid) {
        log.add_log_item(data);
    } else {
        let mut log = GameRuntimeLog::new();
        log.add_log_item(data);

        logs.insert(uuid.clone(), log);
    }
}
