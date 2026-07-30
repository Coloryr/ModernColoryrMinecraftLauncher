//! 统一通行证（Nide8）API
//!
//! 提供从 Nide8 服务器获取认证 JAR 版本信息和哈希值的功能。
//! 用于在游戏启动时注入 Nide8 认证模块。

use mcml_names::{i18_items::error_type::CoreResult};
use serde::{Deserialize, Serialize};

use crate::{WORK_CLIENT, urls};

/// Nide8 JAR 信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Nide8Obj {
    /// JAR 版本号
    #[serde(rename = "jarVersion")]
    pub jar_version: String,
    /// JAR 文件哈希
    #[serde(rename = "jarHash")]
    pub jar_hash: String,
}

impl Default for Nide8Obj {
    fn default() -> Self {
        Self {
            jar_version: Default::default(),
            jar_hash: Default::default(),
        }
    }
}

/// 获取 Nide8 认证 JAR 的最新版本信息和哈希
///
/// 向 Nide8 服务器查询最新 JAR 版本，用于下载和校验。
pub async fn get_obj() -> CoreResult<Nide8Obj> {
    WORK_CLIENT
        .get()
        .unwrap()
        .get_json::<Nide8Obj>(&format!(
            "{}00000000000000000000000000000000/",
            urls::NIDE8_URL
        ))
        .await
}
