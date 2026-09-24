//! SakuraFrp（樱花内网穿透）API
//!
//! 提供通道列表查询、frpc 配置生成与 frpc 客户端下载信息。

use mml_base::file_item::{FileHash, FileItemObj, LaterRun};
use mml_names::i18_items::error_type::{CoreResult, ErrorType};
use mml_sys::Os;
use serde::{Deserialize, Serialize};

use crate::urls;

/// 单个通道
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SakuraFrpChannelObj {
    /// 通道 ID
    pub id: i32,
    /// 通道名称
    pub name: String,
    /// 通道协议类型
    #[serde(rename = "type")]
    pub c_type: String,
    /// 远程访问地址
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

/// 通道配置查询请求体
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SakuraFrpGetChannelObj {
    /// 查询的通道 ID
    pub query: i32,
}

impl Default for SakuraFrpGetChannelObj {
    fn default() -> Self {
        Self {
            query: Default::default(),
        }
    }
}

/// frpc 客户端下载信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SakuraFrpDownloadObj {
    /// 各架构下载项
    pub frpc: SakuraFrpDownloadItemObj,
}

impl Default for SakuraFrpDownloadObj {
    fn default() -> Self {
        Self {
            frpc: Default::default(),
        }
    }
}

/// frpc 各架构下载信息
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SakuraFrpDownloadItemObj {
    /// 各平台架构的下载项
    pub archs: ArchsObj,
    /// frpc 版本号
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

/// 各平台架构的下载项集合
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

/// 单个架构的下载项
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ArchItemObj {
    /// 架构标题
    pub title: String,
    /// 下载 URL
    pub url: String,
    /// 文件哈希
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
///
/// # 返回值
///
/// 返回账户下所有通道
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
///
/// # 返回值
///
/// 返回该通道的 frpc 配置文本
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
///
/// # 返回值
///
/// 返回 frpc 各架构的下载信息
pub async fn get_download() -> CoreResult<SakuraFrpDownloadObj> {
    let client = crate::get_work_client();
    let url = format!("{}system/clients", urls::SAKURA_FRP);

    client.get_json(&url).await
}

/// 创建Frp下载项目
///
/// # 返回值
///
/// 返回与当前系统架构匹配的 frpc 下载任务；不支持的系统返回 `InvalidOperation`
pub async fn build_download_item() -> CoreResult<FileItemObj> {
    let obj = get_download().await?;

    let sys = mml_sys::get_system_info();
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
