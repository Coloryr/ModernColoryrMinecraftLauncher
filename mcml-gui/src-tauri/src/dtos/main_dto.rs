//! 主窗口 DTO：事件负载 + 实例更新补丁（前端 wire，camelCase）
//!
//! 从 `../windows/main.rs` 挪出：这些类型只用于跨 Tauri IPC（事件 / 命令入参），
//! 无业务方法、不参与磁盘持久化，归入 DTO 层。

use serde::{Deserialize, Serialize};

/// 双层 Option 反序列化：JSON `null` -> `Some(None)`（区分“没传”和“清空”）
///
/// 配合 `#[serde(default)]`：字段缺失 -> `None`（不改），
/// `null` -> `Some(None)`（清空），有值 -> `Some(Some(v))`（更新）。
fn double_option<'de, T, D>(de: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// Minecraft 新闻条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadState {
    pub ok: bool,
    pub error: Option<String>,
}

/// Minecraft 新闻条目
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NewsItem {
    pub id: i64,
    pub title: String,
    pub date: String,
    pub tag: String,
    pub image: String,
    /// 原文链接（点击卡片用系统浏览器打开）
    pub url: String,
}

/// 游戏日志事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEvent {
    pub uuid: String,
    pub time: String,
    pub text: String,
    pub clear: bool,
}

/// 启动状态事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StateEvent {
    pub uuid: String,
    pub state: String,
}

/// 游戏退出事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitEvent {
    pub uuid: String,
    pub code: i32,
}

/// 启动错误事件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEvent {
    pub uuid: Option<String>,
    pub message: String,
}

/// 实例变更事件（instance-change）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceChangeEvent {
    pub r#type: String,
}

/// 实例更新补丁（前端 Partial<InstanceInfo> 的 IPC 形态）
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstancePatch {
    pub group: Option<Option<String>>,
    pub name: Option<String>,
    pub version: Option<String>,
    pub version_type: Option<String>,
    pub loader: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub loader_version: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub modpack_type: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub pid: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub fid: Option<Option<String>>,
    #[serde(default, deserialize_with = "double_option")]
    pub server_url: Option<Option<String>>,
    pub lang: Option<String>,
    pub log_encoding: Option<String>,
}
