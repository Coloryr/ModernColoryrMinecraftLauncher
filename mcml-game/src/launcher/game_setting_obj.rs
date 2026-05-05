use std::collections::HashMap;

use chrono::{DateTime, Duration, FixedOffset};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

/// 加入服务器设置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ServerObj {
    /// 服务器地址
    #[serde(rename = "IP")]
    pub ip: Option<String>,
    /// 服务器端口
    #[serde(rename = "Port")]
    pub port: Option<u16>,
}

impl Default for ServerObj {
    fn default() -> Self {
        Self {
            ip: Default::default(),
            port: Default::default(),
        }
    }
}

/// 端口代理设置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ProxyHostObj {
    /// 服务器地址
    #[serde(rename = "IP")]
    pub ip: Option<String>,
    /// 服务器端口
    #[serde(rename = "Port")]
    pub port: Option<u16>,

    /// 服务器地址
    #[serde(rename = "User")]
    pub user: Option<String>,
    /// 服务器地址
    #[serde(rename = "Password")]
    pub password: Option<String>,
}

impl Default for ProxyHostObj {
    fn default() -> Self {
        Self {
            ip: Default::default(),
            port: Default::default(),
            user: Default::default(),
            password: Default::default(),
        }
    }
}

/// 高级Jvm启动参数
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AdvanceJvmObj {
    /// 自定义mainclass
    #[serde(rename = "MainClass")]
    pub main_class: Option<String>,
    /// 附加classpath
    #[serde(rename = "ClassPath")]
    pub class_path: Option<String>,
}

impl Default for AdvanceJvmObj {
    fn default() -> Self {
        Self {
            main_class: Default::default(),
            class_path: Default::default(),
        }
    }
}

// 序列化：将 Duration 转换为统一格式（支持天）
fn serialize_duration<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let total_seconds = duration.num_seconds();
    let nanos = duration.subsec_nanos();

    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    // 使用完整的纳秒值，保留所有有效数字
    let fractional_seconds = nanos as f64 / 1_000_000_000.0;
    let seconds_with_frac = seconds as f64 + fractional_seconds;

    let formatted = if days > 0 {
        // 格式: days.HH:MM:SS.fffffff (保留7位小数，但不截断有效数字)
        format!(
            "{}.{:02}:{:02}:{:09.7}",
            days, hours, minutes, seconds_with_frac
        )
    } else {
        format!("{:02}:{:02}:{:09.7}", hours, minutes, seconds_with_frac)
    };

    // 移除末尾多余的0，但保持至少1位小数
    let formatted = trim_trailing_zeros(&formatted);
    serializer.serialize_str(&formatted)
}

fn trim_trailing_zeros(s: &str) -> String {
    if let Some(dot_pos) = s.find('.') {
        let integer_part = &s[..dot_pos];
        let fractional_part = &s[dot_pos + 1..];

        // 移除末尾的0
        let trimmed = fractional_part.trim_end_matches('0');

        if trimmed.is_empty() {
            format!("{}.0", integer_part)
        } else {
            format!("{}.{}", integer_part, trimmed)
        }
    } else {
        s.to_string()
    }
}

// 反序列化：支持多种格式
fn deserialize_duration<'de, D>(deserializer: D) -> Result<Duration, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    parse_duration_flexible(&s).map_err(serde::de::Error::custom)
}

fn parse_duration_flexible(time_str: &str) -> Result<Duration, String> {
    let time_str = time_str.trim();

    // 格式1: "6.04:35:49.3989028" (带天数)
    if let Some(dot_pos) = time_str.find('.') {
        let days_part = &time_str[..dot_pos];
        let time_part = &time_str[dot_pos + 1..];

        if let Ok(days) = days_part.parse::<i64>() {
            if time_part.contains(':') {
                let time_duration = parse_time_part(time_part)?;
                return Ok(Duration::days(days) + time_duration);
            }
        }
    }

    // 格式2: "00:16:49.9106414" (标准格式)
    if time_str.contains(':') {
        return parse_time_part(time_str);
    }

    Err(format!("Unsupported time format: {}", time_str))
}

fn parse_time_part(time_str: &str) -> Result<Duration, String> {
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return Err(format!(
            "Invalid time format, expected HH:MM:SS, got: {}",
            time_str
        ));
    }

    // 解析小时
    let hours: i64 = parts[0]
        .parse()
        .map_err(|_| format!("Invalid hours: {}", parts[0]))?;

    // 解析分钟
    let minutes: i64 = parts[1]
        .parse()
        .map_err(|_| format!("Invalid minutes: {}", parts[1]))?;

    // 验证分钟范围
    if minutes < 0 || minutes >= 60 {
        return Err(format!("Minutes must be between 0-59, got: {}", minutes));
    }

    // 解析秒和纳秒
    let sec_part = parts[2];
    let (seconds, nanos) = if let Some(dot_pos) = sec_part.find('.') {
        let secs_str = &sec_part[..dot_pos];
        let frac_str = &sec_part[dot_pos + 1..];

        let seconds: i64 = secs_str
            .parse()
            .map_err(|_| format!("Invalid seconds: {}", secs_str))?;

        // 验证秒范围
        if seconds < 0 || seconds >= 60 {
            return Err(format!("Seconds must be between 0-59, got: {}", seconds));
        }

        // 将小数部分转换为纳秒（支持1-9位）
        let nano_str = format!("{:0<9}", frac_str);
        let nano_str = &nano_str[..9.min(nano_str.len())];
        let nanos: i64 = nano_str
            .parse()
            .map_err(|_| format!("Invalid fractional seconds: {}", frac_str))?;

        (seconds, nanos)
    } else {
        let seconds: i64 = sec_part
            .parse()
            .map_err(|_| format!("Invalid seconds: {}", sec_part))?;

        if seconds < 0 || seconds >= 60 {
            return Err(format!("Seconds must be between 0-59, got: {}", seconds));
        }

        (seconds, 0)
    };

    Ok(Duration::hours(hours)
        + Duration::minutes(minutes)
        + Duration::seconds(seconds)
        + Duration::nanoseconds(nanos))
}

/// 游戏统计
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameTimeObj {
    /// 实例添加时间
    #[serde(rename = "AddTime")]
    pub add_time: DateTime<FixedOffset>,
    /// 上次启动时间
    #[serde(rename = "LastTime")]
    pub last_time: DateTime<FixedOffset>,
    /// 游戏时间
    #[serde(rename = "GameTime")]
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub game_time: Duration,
    /// 游戏统计
    #[serde(rename = "LastPlay")]
    #[serde(
        serialize_with = "serialize_duration",
        deserialize_with = "deserialize_duration"
    )]
    pub last_play: Duration,
}

impl Default for GameTimeObj {
    fn default() -> Self {
        Self {
            add_time: Default::default(),
            last_time: Default::default(),
            game_time: Default::default(),
            last_play: Default::default(),
        }
    }
}

/// 自定义模组加载器设置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CustomLoaderObj {
    /// 后加载原版运行库
    #[serde(rename = "OffLib")]
    pub off_list: bool,
    /// 删除原版运行库
    #[serde(rename = "RemoveLib")]
    pub remove_lib: bool,
    /// 是否启用自定义启动配置
    #[serde(rename = "CustomJson")]
    pub custom_json: bool,
    /// 删除原有启动配置
    #[serde(rename = "RemoveJson")]
    pub remove_json: bool,
}

impl Default for CustomLoaderObj {
    fn default() -> Self {
        Self {
            off_list: Default::default(),
            remove_lib: Default::default(),
            custom_json: Default::default(),
            remove_json: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameSettingObj {
    #[serde(rename = "UUID")]
    pub uuid: Uuid,

}
