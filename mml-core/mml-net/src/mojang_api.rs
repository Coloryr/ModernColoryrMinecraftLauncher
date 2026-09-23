//! Mojang API
//!
//! 提供与 Mojang 官方 API 交互的功能，包括：
//!
//! - 获取 Minecraft 版本列表
//! - 获取玩家档案（Profile）和皮肤信息
//! - Xbox Live → Minecraft Token 认证
//! - Minecraft 官方新闻
//!
//! # 认证相关
//!
//! 本模块中的 [`get_minecraft_profile`] 和 [`get_minecraft_token`]
//! 是 Microsoft OAuth 认证流程的最后两步。

use mml_config::config_obj::SourceLocal;
use mml_names::i18_items::error_type::{CoreResult, ErrorType};

use reqwest::{Method, Request, StatusCode, Url, header::ETAG, header::HeaderValue};
use serde::{Deserialize, Serialize};

use crate::{WORK_CLIENT, url_helper, urls};

/// 直接下载资源
pub async fn get_assets(url: &String) -> CoreResult<Vec<u8>> {
    WORK_CLIENT.get().unwrap().get_bytes(url).await
}

/// 缓存校验下载的结果
pub struct AssetCheckResponse {
    /// 服务器返回 304：本地缓存的资源仍然有效
    pub not_modified: bool,
    /// ETag 响应头（S3 / R2 类存储在非分片上传时就是内容的 MD5）
    pub etag: Option<String>,
    /// 响应体（304 时为空）
    pub data: Vec<u8>,
}

/// 直接下载资源，可带上次记录的 ETag 做缓存校验（If-None-Match）
///
/// - 服务器 304 → `not_modified = true`，`data` 为空，本地缓存可以继续用
/// - 服务器 200 → 返回新内容与新的 ETag，本地缓存应更新
pub async fn get_assets_with_check(
    url: &String,
    etag: Option<&str>,
) -> CoreResult<AssetCheckResponse> {
    let resp = WORK_CLIENT
        .get()
        .unwrap()
        .get_if_none_match(url, etag)
        .await?;

    let not_modified = resp.status() == StatusCode::NOT_MODIFIED;
    let etag = resp
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_string());
    let data = if not_modified {
        Vec::new()
    } else {
        resp.bytes().await.map_err(crate::map_err)?.to_vec()
    };

    Ok(AssetCheckResponse {
        not_modified,
        etag,
        data,
    })
}

/// 获取主版本列表
pub async fn get_versions(source: Option<SourceLocal>) -> CoreResult<Vec<u8>> {
    let url = url_helper::game_version(source);
    WORK_CLIENT.get().unwrap().get_bytes(&url).await
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct MinecraftProfileObj {
    pub id: String,
    pub name: String,
}

impl Default for MinecraftProfileObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct UserProfilePropertiesObj {
    pub value: String,
}

impl Default for UserProfilePropertiesObj {
    fn default() -> Self {
        Self {
            value: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct UserProfileObj {
    pub properties: Vec<UserProfilePropertiesObj>,
}

impl Default for UserProfileObj {
    fn default() -> Self {
        Self {
            properties: Default::default(),
        }
    }
}

/// 获取账户信息
/// - `token`: 登陆Token
pub async fn get_minecraft_profile(token: &str) -> CoreResult<MinecraftProfileObj> {
    let client = crate::get_login_client();
    let mut req = Request::new(Method::GET, Url::parse(urls::MINECRAFT_SERVICES).unwrap());
    req.headers_mut().insert(
        "Authorization",
        HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
    );
    let res = client.send(req).await?;
    let data = crate::handle_response::<MinecraftProfileObj>(res).await?;

    Ok(data)
}

/// 获取皮肤信息
/// - `uuid`:
/// - `url`: 网址
pub async fn get_user_profile(uuid: &str, url: Option<&str>) -> CoreResult<UserProfileObj> {
    let url = match url {
        Some(data) => data.to_string(),
        None => format!("{}/{uuid}", urls::MINECRAFT_SESSION_SERVER),
    };
    crate::get_login_client()
        .get_json::<UserProfileObj>(&url)
        .await
}

/// 按玩家名查UUID档案（查不到玩家时接口返回204，这里同样报DataNotFound）
pub async fn get_profile_by_name(name: &str) -> CoreResult<MinecraftProfileObj> {
    let url = format!("{}/{name}", urls::MINECRAFT_PROFILE_API);
    crate::get_login_client()
        .get_json::<MinecraftProfileObj>(&url)
        .await
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct MinecraftTokenObj {
    #[serde(rename = "identityToken")]
    pub identity_token: String,
}

impl Default for MinecraftTokenObj {
    fn default() -> Self {
        Self {
            identity_token: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct MinecraftTokenResObj {
    pub access_token: String,
    pub expires_in: i64,
}

impl Default for MinecraftTokenResObj {
    fn default() -> Self {
        Self {
            access_token: Default::default(),
            expires_in: Default::default(),
        }
    }
}

/// 从XBOX登陆获取账户认证
pub async fn get_minecraft_token(uhs: &str, token: &str) -> CoreResult<String> {
    let obj = MinecraftTokenObj {
        identity_token: format!("XBL3.0 x={uhs};{token}"),
    };

    let res = crate::get_login_client()
        .post_json_get_json::<_, MinecraftTokenResObj>(urls::MINECRAFT_SERVICES_XBOX, &obj)
        .await?;

    if res.expires_in <= 0 || res.access_token.is_empty() {
        Err(ErrorType::AuthTokenTimeout)
    } else {
        Ok(res.access_token)
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ImageObj {
    pub content_type: String,
    #[serde(rename = "imageURL")]
    pub image_url: String,
    pub alt: String,
}

impl Default for ImageObj {
    fn default() -> Self {
        Self {
            content_type: Default::default(),
            image_url: Default::default(),
            alt: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct DefaultTileObj {
    pub title: String,
    pub sub_header: String,
    pub tile_size: String,
    pub image: ImageObj,
}

impl Default for DefaultTileObj {
    fn default() -> Self {
        Self {
            title: Default::default(),
            sub_header: Default::default(),
            tile_size: Default::default(),
            image: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ArticleGridObj {
    pub default_tile: DefaultTileObj,
    pub primary_category: String,
    pub article_url: String,
}

impl Default for ArticleGridObj {
    fn default() -> Self {
        Self {
            default_tile: Default::default(),
            primary_category: Default::default(),
            article_url: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct MinecraftNewsObj {
    pub article_grid: Vec<ArticleGridObj>,
}

impl Default for MinecraftNewsObj {
    fn default() -> Self {
        Self {
            article_grid: Default::default(),
        }
    }
}

/// 获取Minecraft新闻
pub async fn get_minecraft_news(page: u32) -> CoreResult<MinecraftNewsObj> {
    let url = format!("{}{page}.json", urls::MINECRAFT_NEWS);

    crate::get_work_client()
        .get_json::<MinecraftNewsObj>(&url)
        .await
}
