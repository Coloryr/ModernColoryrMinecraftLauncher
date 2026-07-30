//! 日志条目数据结构
//!
//! 定义日志系统中的日志级别枚举和单条日志条目结构。

use std::time::SystemTime;

use chrono::{DateTime, Datelike, Local, Timelike};

/// 日志级别
///
/// 从低到高依次为：Info → Warn → Error → Fault
pub(crate) enum LogLevel {
    /// 一般信息
    Info,
    /// 警告
    Warn,
    /// 错误
    Error,
    /// 严重错误/崩溃
    Fault,
}

/// 单条日志条目
///
/// 包含日志内容、级别和记录时间。
pub(crate) struct LogItem {
    /// 日志文本内容
    pub log: String,
    /// 日志级别
    level: LogLevel,
    /// 日志记录时间（系统时间）
    time: SystemTime,
}

impl LogItem {
    /// 创建一条日志条目
    ///
    /// # 参数
    ///
    /// - `text`: 日志内容
    /// - `level`: 日志级别
    pub fn new(text: String, level: LogLevel) -> Self {
        LogItem {
            log: text,
            level,
            time: SystemTime::now(),
        }
    }

    /// 获取格式化的时间字符串
    ///
    /// 格式：`YYYY-MM-DD HH:MM:SS`
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

    /// 获取日志级别的字符串表示
    pub fn get_level(&self) -> &str {
        match self.level {
            LogLevel::Info => "Info",
            LogLevel::Warn => "Warn",
            LogLevel::Error => "Error",
            LogLevel::Fault => "Fault",
        }
    }
}
