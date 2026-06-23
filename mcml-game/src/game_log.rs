use std::{
    collections::HashMap,
    sync::{LazyLock, RwLock},
};

use regex::Regex;
use uuid::Uuid;

static GAME_RUNTIME_LOG: LazyLock<RwLock<HashMap<Uuid, GameRuntimeLog>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

static REGEX_LOG: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[(.*?)\] \[(.*?)(?:\/(.*?))?\]:? \[(.*?)\](?: (.*))?").unwrap());
static REGEX_LOG_OLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[(.*?)\] \[(.*?)(?:\/(.*?))?\]:?").unwrap());

pub struct GameLogItemObj {
    
}

pub struct GameRuntimeLog {

}
