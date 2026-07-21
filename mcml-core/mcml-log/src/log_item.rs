use std::time::SystemTime;

use chrono::{DateTime, Datelike, Local, Timelike};

/// 日志等级
pub enum LogLevel {
    /// 信息
    Info,
    /// 警告
    Warn,
    /// 错误
    Error,
    /// 严重错误
    Fault
}

pub struct LogItem {
    /// 日志内容
    pub log: String,
    /// 日志等级
    level: LogLevel,
    /// 日志时间
    time: SystemTime,
}

/// 一条日志
impl LogItem {
    /// 生成一条日志
    /// text 日志内容
    /// level 日志等级
    pub fn new(text: String, level: LogLevel) -> Self {
        LogItem {
            log: text,
            level,
            time: SystemTime::now(),
        }
    }

    /// 获取时间字符串
    pub fn get_time(&self) -> String {
        let time: DateTime<Local> = self.time.into();

        format!(
            "{}-{}-{} {}:{}:{}",
            time.year(),
            time.month(),
            time.day(),
            time.hour(),
            time.minute(),
            time.second()
        )
        .to_string()
    }

    /// 获取日志等级字符串
    pub fn get_level(&self) -> &str {
        match self.level {
            LogLevel::Info => "Info",
            LogLevel::Warn => "Warn",
            LogLevel::Error => "Error",
            LogLevel::Fault => "Fault",
        }
    }
}
