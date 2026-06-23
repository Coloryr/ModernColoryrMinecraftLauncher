use std::sync::LazyLock;

use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use serde::{Deserialize, Serialize};

use crate::{WORK_CLIENT, url_helper};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AuthlibInjectorMetaObj {
    pub latest_build_number: i32,
    pub artifacts: Vec<ArtifactsObj>,
}

impl Default for AuthlibInjectorMetaObj {
    fn default() -> Self {
        Self {
            latest_build_number: Default::default(),
            artifacts: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ArtifactsObj {
    pub build_number: i32,
}

impl Default for ArtifactsObj {
    fn default() -> Self {
        Self {
            build_number: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AuthlibInjectorObj {
    pub build_number: i32,
    pub version: String,
    pub download_url: String,
    pub checksums: ChecksumsObj,
}

impl Default for AuthlibInjectorObj {
    fn default() -> Self {
        Self {
            build_number: Default::default(),
            version: Default::default(),
            download_url: Default::default(),
            checksums: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ChecksumsObj {
    pub sha256: String,
}

impl Default for ChecksumsObj {
    fn default() -> Self {
        Self {
            sha256: Default::default(),
        }
    }
}

pub static LOCAL_AUTHLIB: LazyLock<AuthlibInjectorObj> = LazyLock::new(|| AuthlibInjectorObj {
    build_number: 55,
    version: String::from("1.2.7"),
    download_url: String::from(
        "https://authlib-injector.yushi.moe/artifact/55/authlib-injector-1.2.7.jar",
    ),
    checksums: ChecksumsObj {
        sha256: String::from("eaf14bc5acffc7d885bd5bd5942b99f36d6299302beae356b2fc5807fe42652b"),
    },
});

/// 获取最新AuthlibInjector信息
pub async fn get_obj() -> CoreResult<AuthlibInjectorObj> {
    let url = url_helper::get_authlib_injector_meta();
    let meta = WORK_CLIENT
        .get()
        .unwrap()
        .get_json::<AuthlibInjectorMetaObj>(&url)
        .await?;

    let item = meta
        .artifacts
        .iter()
        .find(|item| item.build_number == meta.latest_build_number);

    match item {
        None => Err(ErrorType::DataNotFound),
        Some(data) => Ok(WORK_CLIENT
            .get()
            .unwrap()
            .get_json::<AuthlibInjectorObj>(&url_helper::get_authlib_injector(data))
            .await?),
    }
}
