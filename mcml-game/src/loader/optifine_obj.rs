use mcml_config::config_obj::SourceLocal;
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
    pub mcversion: String,
    /// Forge加载器信息
    #[serde(rename = "Forge")]
    pub forge: String,
    /// 文件名
    #[serde(rename = "FileName")]
    pub file_name: String,
    /// 日期
    #[serde(rename = "Date")]
    pub date: String,
    #[serde(rename = "Url1")]
    pub url1: Option<String>,
    #[serde(rename = "Url2")]
    pub url2: Option<String>,
    /// 下载源
    #[serde(rename = "Local")]
    pub local: SourceLocal,
}

impl Default for OptifineObj {
    fn default() -> Self {
        Self {
            version: Default::default(),
            mcversion: Default::default(),
            forge: Default::default(),
            file_name: Default::default(),
            date: Default::default(),
            url1: Default::default(),
            url2: Default::default(),
            local: Default::default(),
        }
    }
}
