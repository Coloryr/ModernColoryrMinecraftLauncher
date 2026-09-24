//! OptiFine 安装信息（实例目录内保存的版本）

use mml_config::config_obj::SourceLocal;
use mml_net::optifine_api::GetOptifineObj;
use serde::{Deserialize, Serialize};

/// 高清修复信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OptifineObj {
    /// 版本号
    #[serde(rename = "Version")]
    pub version: String,
    /// 游戏版本号
    #[serde(rename = "MCVersion")]
    pub mc_version: String,
    /// Forge加载器信息
    #[serde(rename = "Forge")]
    pub forge: String,
    /// 文件名
    #[serde(rename = "FileName")]
    pub file_name: String,
    /// 日期
    #[serde(rename = "Date")]
    pub date: String,
    /// 下载地址（官方源为下载页 / 镜像源为 JAR 地址）
    #[serde(rename = "Url1")]
    pub url1: Option<String>,
    /// 镜像下载地址（仅官方源有）
    #[serde(rename = "Url2")]
    pub url2: Option<String>,
    /// 下载源
    #[serde(rename = "Local")]
    pub source: SourceLocal,
}

impl From<GetOptifineObj> for OptifineObj {
    fn from(value: GetOptifineObj) -> Self {
        OptifineObj {
            version: value.version,
            mc_version: value.mc_version,
            forge: value.forge,
            file_name: value.file_name,
            date: value.date,
            url1: value.url1,
            url2: value.url2,
            source: value.source,
        }
    }
}

impl Default for OptifineObj {
    fn default() -> Self {
        Self {
            version: Default::default(),
            mc_version: Default::default(),
            forge: Default::default(),
            file_name: Default::default(),
            date: Default::default(),
            url1: Default::default(),
            url2: Default::default(),
            source: Default::default(),
        }
    }
}
