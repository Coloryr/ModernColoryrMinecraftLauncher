use std::{
    collections::VecDeque, path::{Path, PathBuf}, sync::{LazyLock, RwLock}, time::Duration,
};

use chrono::{DateTime, FixedOffset, Local};
use mcml_base::path_helper;
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use regex::Regex;
use tokio::io::{AsyncBufReadExt, BufReader};

static REGEX_LOG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[(.*?)\] \[(.*?)(?:\/(.*?))?\]:? \[(.*?)\](?: (.*))?").unwrap());
static REGEX_LOG_OLD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[(.*?)\] \[(.*?)(?:\/(.*?))?\]:?").unwrap());

#[derive(Clone)]
pub enum LogLevel {
    None,
    Info,
    Warn,
    Error,
    Debug,
}

#[derive(Clone)]
pub struct GameLogObj {
    pub log: String,
    pub time: String,
    pub thread: String,
    pub level: LogLevel,
    pub category: String,
}

/// 游戏日志启动器消息
#[derive(Clone)]
pub enum GameLog {
    /// 没有分类的日志输出
    Text(String),
    /// 游戏日志
    GameLog(GameLogObj),
    /// 运行库输出
    RuntimeLib(PathBuf),
    /// 日志重定向
    JavaRedirect,
    /// Java切换回启动查找
    JavaLocalRedirect,
    /// 登录用时
    LoginTime(Duration),
    /// 服务器包检查用时
    ServerPackCheckTime(Duration),
    /// 检查游戏文件用时
    CheckGameFileTime(Duration),
    /// 文件下载用时
    DownloadFileTime(Duration),
    /// 启动参数
    LaunchArgs(String),
    /// Java路径
    JavaPath(PathBuf),
    /// 启动用时
    LaunchTime(Duration),
    /// 启动前执行用时
    CmdPreTime(Duration),
    /// 启动后执行用时
    CmdPostTime(Duration),
}

/// 游戏运行日志
#[derive(Clone)]
pub struct GameLogItemObj {
    /// 获取时间
    pub time: DateTime<FixedOffset>,
    /// 日志内容
    pub log: GameLog,
}

/// 游戏运行日志处理
pub struct GameRuntimeLog {
    /// 日志列表
    pub logs: RwLock<VecDeque<GameLogItemObj>>,
    /// 日志文件，如果是打开日志文件时
    pub file: Option<PathBuf>,
}

impl GameRuntimeLog {
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(VecDeque::new()),
            file: None,
        }
    }

    /// 从文件读取日志
    pub async fn from_file<P: AsRef<Path>>(file: P) -> CoreResult<Self> {
        let stream = path_helper::open_read_async(&file).await?;

        let mut log = Self {
            logs: RwLock::new(VecDeque::new()),
            file: Some(file.as_ref().to_path_buf()),
        };

        let reader = BufReader::new(stream);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await.map_err(|err| {
            ErrorType::StreamError(ErrorData {
                error: err.to_string(),
            })
        })? {
            log.add_game_log(&line);
        }

        Ok(log)
    }

    /// 清理所有日志
    pub fn clear(&mut self) {
        let mut logs = self.logs.write().unwrap();
        logs.clear();
    }

    /// 添加游戏输出的日志
    pub fn add_game_log(&mut self, log: &str) -> GameLogItemObj {
        let temp = log.trim();
        let obj = if let Some(captures) = REGEX_LOG.captures(temp)
            && captures.len() == 6
        {
            GameLog::GameLog(GameLogObj {
                level: get_level(&captures[3]),
                log: String::from(log),
                thread: String::from(&captures[2]),
                category: String::from(&captures[4]),
                time: String::from(&captures[1]),
            })
        } else if let Some(captures) = REGEX_LOG_OLD.captures(temp)
            && captures.len() == 4
        {
            GameLog::GameLog(GameLogObj {
                level: get_level(&captures[3]),
                log: String::from(log),
                thread: String::from(&captures[2]),
                category: Default::default(),
                time: String::from(&captures[1]),
            })
        } else {
            GameLog::Text(String::from(log))
        };

        self.add_log_item(obj)
    }

    /// 添加日志
    pub fn add_log_item(&mut self, log: GameLog) -> GameLogItemObj {
        let mut logs = self.logs.write().unwrap();
        let item = GameLogItemObj {
            time: Local::now().fixed_offset(),
            log: log,
        };

        if logs.len() > 10000 {
            logs.pop_front();
        }
        logs.push_back(item.clone());

        item
    }
}

fn get_level(level: &str) -> LogLevel {
    match level.to_lowercase().as_str() {
        "info" => LogLevel::Info,
        "warn" => LogLevel::Warn,
        "error" => LogLevel::Error,
        "fatal" => LogLevel::Error,
        "debug" => LogLevel::Debug,
        _ => LogLevel::None,
    }
}
