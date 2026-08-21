use std::collections::HashMap;

use mcml_base::file_item::{FileHash, FileItemObj, LaterRun};
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use mcml_sys::Os;
use serde::{Deserialize, Serialize};

use crate::urls;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OpenFrpChannelObj {
    pub data: Vec<OpenFrpChannelData>,
}

impl Default for OpenFrpChannelObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OpenFrpChannelData {
    pub name: String,
    pub proxies: Vec<ProxieObj>,
}

impl Default for OpenFrpChannelData {
    fn default() -> Self {
        Self {
            name: Default::default(),
            proxies: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ProxieObj {
    pub name: String,
    pub id: i32,
    #[serde(rename = "type")]
    pub p_type: String,
    pub remote: String,
}

impl Default for ProxieObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            id: Default::default(),
            p_type: Default::default(),
            remote: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OpenFrpChannelInfoObj {
    pub proxies: HashMap<String, String>,
}

impl Default for OpenFrpChannelInfoObj {
    fn default() -> Self {
        Self {
            proxies: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OpenFrpDownloadObj {
    pub data: OpenFrpDownloadItemObj,
}

impl Default for OpenFrpDownloadObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OpenFrpDownloadItemObj {
    pub latest: String,
    pub latest_full: String,
    pub source: Vec<SourceObj>,
}

impl Default for OpenFrpDownloadItemObj {
    fn default() -> Self {
        Self {
            latest: Default::default(),
            latest_full: Default::default(),
            source: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SourceObj {
    pub value: String,
}

impl Default for SourceObj {
    fn default() -> Self {
        Self {
            value: Default::default(),
        }
    }
}

/// 获取通道列表
///
/// - `key`: 账户密钥
pub async fn get_channel(key: &str) -> CoreResult<OpenFrpChannelObj> {
    let client = crate::get_work_client();
    let url = format!("{}?action=getallproxies&user={key}", urls::OPENFRP);

    client.get_json(&url).await
}

/// 获取通道配置
///
/// - `key`: 账户密钥
/// - `id`: 通道ID
pub async fn get_channel_config(key: &str, id: i32) -> CoreResult<OpenFrpChannelInfoObj> {
    let client = crate::get_work_client();
    let url = format!("{}?action=getproxy&proxy={id}&user={key}", urls::OPENFRP);

    client.get_json(&url).await
}

/// 获取下载列表
pub async fn get_download() -> CoreResult<OpenFrpDownloadObj> {
    let client = crate::get_work_client();

    client.get_json(urls::OPENFRP_DOWNLOAD).await
}

/// 创建Frp下载项目
pub async fn build_download_item() -> CoreResult<FileItemObj> {
    let data = get_download().await?;

    let sys = mcml_sys::get_system_info();
    let name = if sys.os == Os::Windows {
        if sys.is_arm {
            "frpc_windows_arm64.zip"
        } else {
            "frpc_windows_amd64.zip"
        }
    } else if sys.os == Os::Linux {
        if sys.is_arm {
            "frpc_linux_arm64.tar.gz"
        } else {
            "frpc_linux_amd64.tar.gz"
        }
    } else if sys.os == Os::MacOS {
        if sys.is_arm {
            "frpc_darwin_arm64.tar.gz"
        } else {
            "frpc_darwin_amd64.tar.gz"
        }
    } else {
        ""
    };

    if name.is_empty() {
        return Err(ErrorType::InvalidOperation);
    }

    Ok(FileItemObj {
        name: format!("OpenFrp {}", name),
        file: Default::default(),
        url: format!("{}{}{}", data.data.source[0].value, data.data.latest, name),
        hash: FileHash::None,
        later: LaterRun::None,
    })
}
