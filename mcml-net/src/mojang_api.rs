use mcml_config::config_obj::SourceLocal;
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};

use reqwest::{Method, Request, Url, header::HeaderValue};
use serde::{Deserialize, Serialize};

use crate::{WORK_CLIENT, url_helper, urls};

/// 直接下载资源
pub async fn get_assets(url: &String) -> CoreResult<Vec<u8>> {
    WORK_CLIENT.get().unwrap().get_bytes(url).await
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
