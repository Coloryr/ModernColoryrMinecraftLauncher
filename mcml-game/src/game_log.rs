/// 游戏实例日志相关
/// 包括运行日志和过往日志处理
use std::{
    collections::VecDeque,
    fs,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, RwLock},
    time::{Duration, SystemTime},
};

use chrono::{DateTime, FixedOffset, Local};
use encoding_rs_io::DecodeReaderBytesBuilder;
use flate2::write::GzDecoder;
use mcml_base::path_helper;
use mcml_names::names;
use regex::Regex;

use crate::launcher::{LogEncoding, instance_setting_obj::InstanceSettingObj};

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
pub struct InstanceRuntimeLog {
    /// 日志列表
    pub logs: RwLock<Arc<VecDeque<GameLogItemObj>>>,
    /// 日志文件，如果是打开日志文件时
    pub file: Option<PathBuf>,
}

impl InstanceRuntimeLog {
    pub fn new() -> Self {
        Self {
            logs: RwLock::new(Arc::new(VecDeque::new())),
            file: None,
        }
    }

    /// 从文件读取日志
    pub fn from_file<P: AsRef<Path>>(file: P, encoding: LogEncoding) -> Self {
        let mut log = Self {
            logs: RwLock::new(Arc::new(VecDeque::new())),
            file: Some(file.as_ref().to_path_buf()),
        };

        if let Ok(stream) = path_helper::open_read(&file) {
            let is_gz = file
                .as_ref()
                .file_name()
                .map(|n| n.to_string_lossy().ends_with(names::LOG_GZ_DOT_EXT))
                .unwrap_or(false);

            let reader: Box<BufReader<dyn Read>> = if is_gz {
                let gz = GzDecoder::new(stream);
                Box::new(BufReader::new(gz))
            } else {
                Box::new(BufReader::new(stream))
            };

            match encoding {
                LogEncoding::UTF8 => {
                    let reader = BufReader::new(reader);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            log.add_game_log(&line);
                        }
                    }
                }
                LogEncoding::GBK => {
                    let transcoder = DecodeReaderBytesBuilder::new()
                        .encoding(Some(encoding_rs::GBK))
                        .build(reader);
                    let reader = BufReader::new(transcoder);
                    for line in reader.lines() {
                        if let Ok(line) = line {
                            log.add_game_log(&line);
                        }
                    }
                }
            }
        }

        log
    }

    /// 清理所有日志
    pub fn clear(&mut self) {
        let mut logs = self.logs.write().unwrap();
        *logs = Arc::new(VecDeque::new());
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
        let logs = Arc::make_mut(&mut logs);
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

impl InstanceSettingObj {
    /// 获取游戏日志文件列表
    pub fn get_log_files(&self) -> Vec<PathBuf> {
        fn is_log_file(path: &Path) -> bool {
            if let Some(ext) = path.extension() {
                if ext.eq_ignore_ascii_case(names::LOG_EXT)
                    || ext.eq_ignore_ascii_case(names::TXT_EXT)
                {
                    return true;
                }
            }
            path.file_name()
                .map(|n| n.to_string_lossy().ends_with(names::LOG_GZ_DOT_EXT))
                .unwrap_or(false)
        }

        let mut list = Vec::new();
        let dir = self.get_logs_path();
        if dir.exists() && dir.is_dir() {
            for item in path_helper::get_all_files(dir).iter() {
                if is_log_file(item) {
                    list.push(item.clone());
                }
            }
        }

        let dir = self.get_crash_path();
        if dir.exists() && dir.is_dir() {
            for item in path_helper::get_all_files(dir).iter() {
                if is_log_file(item) {
                    list.push(item.clone());
                }
            }
        }

        list
    }

    /// 获取最新的崩溃日志
    /// - `sec`: 和最新时间最大差值
    pub fn get_last_crash_report(&self, sec: Option<u32>) -> Option<PathBuf> {
        let dir = self.get_crash_path();
        if !dir.exists() || !dir.is_dir() {
            return None;
        }

        let sec = sec.unwrap_or(5);
        let files = path_helper::get_last_written_file(dir);
        if let Ok(Some(path)) = files
            && let Ok(meta) = fs::metadata(&path)
            && let Ok(time) = meta.modified()
        {
            let now = SystemTime::now();

            let duration = now.duration_since(time);
            if let Ok(duration) = duration {
                if duration.as_secs_f64() > sec as f64 {
                    return None;
                } else {
                    return Some(path.clone());
                }
            }
        }

        None
    }

    /// 读取日志
    pub fn read_log<P: AsRef<Path>>(&self, path: P) -> Option<InstanceRuntimeLog> {
        let path = self.get_logs_path().join(path.as_ref());

        if path.exists() && path.is_file() {
            Some(InstanceRuntimeLog::from_file(path, self.encoding))
        } else {
            None
        }
    }
}
