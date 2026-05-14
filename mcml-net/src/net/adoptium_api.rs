use std::sync::OnceLock;

use mcml_names::{i18_items::error_type::ErrorType, urls::ADOPTIUM_URL};
use serde::{Deserialize, Serialize};

static JAVA_VERSION: OnceLock<Vec<String>> = OnceLock::new();

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct PackageObj {
    pub checksum: String,
    pub link: String,
    pub name: String,
    pub size: i64,
}

impl Default for PackageObj {
    fn default() -> Self {
        Self {
            checksum: String::new(),
            link: String::new(),
            name: String::new(),
            size: 0,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct BinaryObj {
    pub architecture: String,
    pub image_type: String,
    pub os: String,
    pub package: PackageObj,
    pub scm_ref: String,
}

impl Default for BinaryObj {
    fn default() -> Self {
        Self {
            architecture: String::new(),
            image_type: String::new(),
            os: String::new(),
            package: PackageObj::default(),
            scm_ref: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AdoptiumVersionObj {
    pub openjdk_version: String,
}

impl Default for AdoptiumVersionObj {
    fn default() -> Self {
        Self {
            openjdk_version: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AdoptiumObj {
    pub binary: BinaryObj,
    pub version: AdoptiumVersionObj,
}

impl Default for AdoptiumObj {
    fn default() -> Self {
        Self {
            binary: BinaryObj::default(),
            version: AdoptiumVersionObj::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AdoptiumJavaVersionObj {
    pub available_releases: Vec<i32>,
}

impl Default for AdoptiumJavaVersionObj {
    fn default() -> Self {
        Self {
            available_releases: Vec::new(),
        }
    }
}

/// 获取API系统对应的字符串
fn get_os(ostype: Os) -> &'static str {
    match ostype {
        Os::Windows => "windows",
        Os::Linux => "linux",
        Os::AlpineLinux => "alpine-linux",
        Os::MacOS => "mac",
        Os::AIX => "aix",
        Os::Solaris => "solaris",
        _ => "",
    }
}

/// 获取支持的Java主版本
pub async fn get_java_version() -> Result<Vec<String>, ErrorType> {
    if let Some(list) = JAVA_VERSION.get() {
        return Ok(list.to_vec());
    }

    let url = String::from(ADOPTIUM_URL) + "v3/info/available_releases";

    let res = mcml_http::WORK_CLIENT
        .get()
        .unwrap()
        .get_json::<AdoptiumJavaVersionObj>(&url)
        .await?;

    let mut list = Vec::new();
    for item in res.available_releases.iter() {
        list.push(item.to_string());
    }

    Ok(list)
}

/// 获取Java文件列表
///
/// - `version`: Java主版本
/// - `os`: 系统类型
pub async fn get_java_list(version: String, os: Os) -> Result<Vec<AdoptiumObj>, ErrorType> {
    let mut url = String::from(ADOPTIUM_URL);
    if os == Os::None {
        url += &format!("v3/assets/latest/{}/hotspot", version);
    } else {
        url += &format!("v3/assets/latest/{}/hotspot?os={}", version, get_os(os));
    }

    let res = mcml_http::WORK_CLIENT
        .get()
        .unwrap()
        .get_json::<Vec<AdoptiumObj>>(&url)
        .await?;

    Ok(res)
}
