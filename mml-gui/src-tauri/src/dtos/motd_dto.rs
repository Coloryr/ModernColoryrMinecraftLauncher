//! 服务器 MOTD 查询 DTO（前端 wire：camelCase）
//!
//! 内核 `mml_game::game_motd` 的查询结果转成前端可直接渲染的形态：
//! 描述文字已展平成带颜色的段落，玩家数拆成 online / max。

use serde::{Deserialize, Serialize};

use mml_game::game_motd::{chat_to_segments, ChatSegment, MotdState, ServerMotdObj};

/// 展平后的一段 MOTD 文字（color 为 #RRGGBB，前端直接上 style）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotdSegmentDto {
    pub text: String,
    pub color: String,
    pub bold: bool,
    pub italic: bool,
    pub underlined: bool,
    pub strikethrough: bool,
}

impl From<&ChatSegment> for MotdSegmentDto {
    fn from(s: &ChatSegment) -> Self {
        Self {
            text: s.text.clone(),
            color: s.color.clone(),
            bold: s.bold,
            italic: s.italic,
            underlined: s.underlined,
            strikethrough: s.strikethrough,
        }
    }
}

/// 服务器 MOTD 查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MotdDto {
    /// 查询状态：ok / noData / connectFail / error
    pub state: String,
    /// 出错时的错误信息
    pub message: String,
    /// 实际查询的地址（SRV 重定向后与传入值不同）
    pub ip: String,
    pub port: u16,
    /// 延迟（毫秒）
    pub ping: u64,
    /// 服务器版本名
    pub version: Option<String>,
    /// 协议版本号
    pub protocol: Option<i32>,
    /// 在线玩家数
    pub players_online: Option<i32>,
    /// 最大玩家数
    pub players_max: Option<i32>,
    /// 服务器图标（data:image/png;base64, 前缀的 Base64 PNG）
    pub favicon: Option<String>,
    /// MOTD 文字段（已展平）
    pub segments: Vec<MotdSegmentDto>,
}

impl From<ServerMotdObj> for MotdDto {
    fn from(m: ServerMotdObj) -> Self {
        let (version, protocol) = m
            .version
            .map_or((None, None), |v| (Some(v.name), Some(v.protocol)));
        let (players_online, players_max) = m
            .players
            .map_or((None, None), |p| (Some(p.online), Some(p.max)));
        Self {
            state: match m.state {
                MotdState::Ok => "ok",
                MotdState::NoData => "noData",
                MotdState::ConnectFail => "connectFail",
                MotdState::Error => "error",
            }
            .to_string(),
            message: m.message,
            ip: m.ip,
            port: m.port,
            ping: m.ping,
            version,
            protocol,
            players_online,
            players_max,
            favicon: m.favicon,
            segments: m
                .description
                .as_ref()
                .map_or(Vec::new(), chat_to_segments)
                .iter()
                .map(MotdSegmentDto::from)
                .collect(),
        }
    }
}
