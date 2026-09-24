//! Mojang 版本清单 DTO

use serde::{Deserialize, Serialize};

/// 最新版本信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LastestObj {
    /// 最新正式版
    pub release: String,
}

impl Default for LastestObj {
    fn default() -> Self {
        Self {
            release: Default::default(),
        }
    }
}

/// 单个版本条目
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct VersionsObj {
    /// 版本 ID
    pub id: String,
    /// 版本类型（release / snapshot / old_beta / old_alpha）
    #[serde(rename = "type")]
    pub version_type: String,
    /// 版本 JSON 下载地址
    pub url: String,
    /// 版本 JSON 校验值
    pub sha1: String,
}

impl Default for VersionsObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            version_type: Default::default(),
            url: Default::default(),
            sha1: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct VersionObj {
    /// 最新版本
    pub latest: LastestObj,
    /// 版本列表
    pub versions: Vec<VersionsObj>,
}

impl Default for VersionObj {
    fn default() -> Self {
        Self {
            latest: Default::default(),
            versions: Default::default(),
        }
    }
}
