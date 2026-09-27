use std::sync::OnceLock;

use mml_names::i18_items::error_type::CoreResult;
use serde::{Deserialize, Serialize};

use crate::urls;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ZuluObj {
    pub arch: String,
    /// 包类型（jdk / jre）
    pub bundle_type: String,
    pub hw_bitness: String,
    pub java_version: Vec<i32>,
    pub name: String,
    pub os: String,
    pub sha256_hash: String,
    pub url: String,
    pub zulu_version: Vec<i32>,
}

impl Default for ZuluObj {
    fn default() -> Self {
        Self {
            arch: Default::default(),
            bundle_type: Default::default(),
            hw_bitness: Default::default(),
            java_version: Default::default(),
            name: Default::default(),
            os: Default::default(),
            sha256_hash: Default::default(),
            url: Default::default(),
            zulu_version: Default::default(),
        }
    }
}

static ZULU: OnceLock<Vec<ZuluObj>> = OnceLock::new();

async fn init() -> CoreResult<Vec<ZuluObj>> {
    let list: Vec<ZuluObj> = crate::get_work_client().get_json(urls::ZULU).await?;
    Ok(ZULU.get_or_init(|| list).clone())
}

pub async fn get_java_list() -> CoreResult<Vec<ZuluObj>> {
    match ZULU.get() {
        Some(data) => Ok(data.clone()),
        None => init().await,
    }
}
