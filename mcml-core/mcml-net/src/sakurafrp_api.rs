use mcml_base::file_item::{FileHash, FileItemObj, LaterRun};
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use mcml_sys::Os;
use serde::{Deserialize, Serialize};

use crate::urls;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SakuraFrpChannelObj {
    pub id: i32,
    pub name: String,
    #[serde(rename = "type")]
    pub c_type: String,
    pub remote: String,
}

impl Default for SakuraFrpChannelObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: Default::default(),
            c_type: Default::default(),
            remote: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SakuraFrpGetChannelObj {
    pub query: i32,
}

impl Default for SakuraFrpGetChannelObj {
    fn default() -> Self {
        Self {
            query: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SakuraFrpDownloadObj {
    pub frpc: SakuraFrpDownloadItemObj,
}

impl Default for SakuraFrpDownloadObj {
    fn default() -> Self {
        Self {
            frpc: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SakuraFrpDownloadItemObj {
    pub archs: ArchsObj,
    pub ver: String,
}

impl Default for SakuraFrpDownloadItemObj {
    fn default() -> Self {
        Self {
            archs: Default::default(),
            ver: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ArchsObj {
    pub windows_amd64: ArchItemObj,
    pub windows_arm64: ArchItemObj,
    pub linux_amd64: ArchItemObj,
    pub linux_arm64: ArchItemObj,
    pub darwin_amd64: ArchItemObj,
    pub darwin_arm64: ArchItemObj,
}

impl Default for ArchsObj {
    fn default() -> Self {
        Self {
            windows_amd64: Default::default(),
            windows_arm64: Default::default(),
            linux_amd64: Default::default(),
            linux_arm64: Default::default(),
            darwin_amd64: Default::default(),
            darwin_arm64: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ArchItemObj {
    pub title: String,
    pub url: String,
    pub hash: String,
}

impl Default for ArchItemObj {
    fn default() -> Self {
        Self {
            title: Default::default(),
            url: Default::default(),
            hash: Default::default(),
        }
    }
}

/// 获取通道列表
///
/// - `key`: 账户密钥
pub async fn get_channel(key: &str) -> CoreResult<Vec<SakuraFrpChannelObj>> {
    let client = crate::get_work_client();
    let url = format!("{}tunnels?token={key}", urls::SAKURA_FRP);

    client.get_json(&url).await
}

/// 获取通道配置
///
/// - `key`: 账户密钥
/// - `id`: 通道ID
/// - `version`: 版本号
pub async fn get_channel_config(key: &str, id: i32, version: &str) -> CoreResult<String> {
    let client = crate::get_work_client();
    let url = format!(
        "{}tunnel/config?token={key}&frpc={version}",
        urls::SAKURA_FRP
    );

    client
        .post_json_get_text(&url, &SakuraFrpGetChannelObj { query: id })
        .await
}

/// 获取下载列表
pub async fn get_download() -> CoreResult<SakuraFrpDownloadObj> {
    let client = crate::get_work_client();
    let url = format!("{}system/clients", urls::SAKURA_FRP);

    client.get_json(&url).await
}

/// 创建Frp下载项目
pub async fn build_download_item() -> CoreResult<FileItemObj> {
    let obj = get_download().await?;

    let sys = mcml_sys::get_system_info();
    let obj = if sys.os == Os::Windows {
        if sys.is_arm {
            obj.frpc.archs.windows_arm64
        } else {
            obj.frpc.archs.windows_amd64
        }
    } else if sys.os == Os::Linux {
        if sys.is_arm {
            obj.frpc.archs.linux_arm64
        } else {
            obj.frpc.archs.linux_amd64
        }
    } else if sys.os == Os::MacOS {
        if sys.is_arm {
            obj.frpc.archs.darwin_arm64
        } else {
            obj.frpc.archs.darwin_amd64
        }
    } else {
        return Err(ErrorType::InvalidOperation);
    };

    Ok(FileItemObj {
        name: format!("SakuraFrp {}", obj.title),
        file: Default::default(),
        url: obj.url,
        hash: FileHash::Sha1(obj.hash),
        later: LaterRun::None,
    })
}
