//! Modrinth 项目团队成员 DTO

use serde::{Deserialize, Serialize};

/// Modrinth 项目团队成员信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthTeamObj {
    /// 成员的用户信息
    pub user: TeamserObj,
}

impl Default for ModrinthTeamObj {
    fn default() -> Self {
        Self {
            user: Default::default(),
        }
    }
}

/// 团队成员的用户信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct TeamserObj {
    /// 用户名
    pub username: String,
    /// 头像 URL
    pub avatar_url: Option<String>,
}

impl Default for TeamserObj {
    fn default() -> Self {
        Self {
            username: Default::default(),
            avatar_url: Default::default(),
        }
    }
}
