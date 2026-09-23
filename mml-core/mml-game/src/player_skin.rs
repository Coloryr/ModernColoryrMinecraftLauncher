//! 获取玩家皮肤
//!
//! 按账户认证类型向对应的会话服务器查询档案，解析 profile 属性里的
//! 皮肤 / 披风地址，并缓存到本地皮肤目录。

use std::path::PathBuf;

use mml_auth::{AuthType, LoginObj};
use mml_base::hash_helper;
use mml_net::{mojang_api, urls};
use mml_sys::path_helper;
use serde::Deserialize;

use crate::launcher_path::assets_path;

/// 皮肤数据（profile 属性 base64 解码后的 JSON）
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct MinecraftTexturesObj {
    textures: TexturesObj,
}

/// 皮肤与披风地址
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct TexturesObj {
    #[serde(rename = "SKIN")]
    skin: Option<TextureObj>,
    #[serde(rename = "CAPE")]
    cape: Option<TextureObj>,
}

/// 单个皮肤 / 披风
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct TextureObj {
    url: String,
    metadata: Option<TextureMetadataObj>,
}

/// 皮肤元数据，`model` 为 `slim` 表示纤细皮肤
#[derive(Debug, Deserialize, Default)]
#[serde(default)]
struct TextureMetadataObj {
    model: String,
}

/// 皮肤下载结果
#[derive(Debug, Default)]
pub struct DownloadSkinRes {
    /// 皮肤文件位置
    pub skin: Option<PathBuf>,
    /// 披风文件位置
    pub cape: Option<PathBuf>,
    /// 是否为纤细（slim）皮肤
    pub is_new_slim: bool,
}

/// 下载皮肤与披风
///
/// 按认证类型查询对应的会话服务器，取回皮肤 / 披风文件并缓存到本地皮肤目录。
pub async fn download_skin(obj: &LoginObj) -> DownloadSkinRes {
    let url = match obj.auth_type {
        AuthType::Offline | AuthType::OAuth => None,
        AuthType::Nide8 => Some(format!(
            "{}{}/sessionserver/session/minecraft/profile/{}",
            urls::NIDE8_URL,
            obj.text1.clone().unwrap_or_default(),
            obj.uuid
        )),
        AuthType::AuthlibInjector => Some(format!(
            "{}/sessionserver/session/minecraft/profile/{}",
            obj.text1.clone().unwrap_or_default(),
            obj.uuid
        )),
        AuthType::LittleSkin => Some(format!(
            "{}api/yggdrasil/sessionserver/session/minecraft/profile/{}",
            urls::LITTLE_SKIN_URL,
            obj.uuid
        )),
        AuthType::SelfLittleSkin => Some(format!(
            "{}/api/yggdrasil/sessionserver/session/minecraft/profile/{}",
            obj.text1.clone().unwrap_or_default(),
            obj.uuid
        )),
    };

    let Some(textures) = load_textures(obj, url.as_deref()).await else {
        return DownloadSkinRes::default();
    };

    let is_new_slim = textures
        .textures
        .skin
        .as_ref()
        .and_then(|skin| skin.metadata.as_ref())
        .map(|metadata| metadata.model == "slim")
        .unwrap_or(false);

    let skin = match &textures.textures.skin {
        Some(data) => load_texture(&data.url).await,
        None => None,
    };
    let cape = match &textures.textures.cape {
        Some(data) => load_texture(&data.url).await,
        None => None,
    };

    DownloadSkinRes {
        skin,
        cape,
        is_new_slim,
    }
}

/// 查询档案并解析皮肤数据
async fn load_textures(obj: &LoginObj, url: Option<&str>) -> Option<MinecraftTexturesObj> {
    let res = mojang_api::get_user_profile(&obj.uuid, url).await.ok()?;
    let value = res.properties.first()?.value.clone();

    parse_textures(&value)
}

/// 解析 profile 属性（base64 后的 JSON）里的皮肤数据
fn parse_textures(value: &str) -> Option<MinecraftTexturesObj> {
    let data = hash_helper::de_base64(value).ok()?;

    serde_json::from_str::<MinecraftTexturesObj>(&data).ok()
}

/// 取回单个皮肤 / 披风文件，已缓存则直接返回位置
async fn load_texture(url: &str) -> Option<PathBuf> {
    if url.trim().is_empty() {
        return None;
    }

    let file = assets_path::get_skin_from_url(url.to_string());
    if file.exists() {
        return Some(file);
    }

    let data = mojang_api::get_assets(&url.to_string()).await.ok()?;
    path_helper::write_bytes(&file, &data).ok()?;

    Some(file)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 带纤细皮肤与披风的 textures 属性
    const SLIM: &str = "eyJ0aW1lc3RhbXAiOjE2MDAwMDAwMDAwMDAsInByb2ZpbGVJZCI6IjA2OWE3OWY0NDRlOTQ3MjZhNWJlZmNhOTBlMzhhYWY1IiwicHJvZmlsZU5hbWUiOiJOb3RjaCIsInRleHR1cmVzIjp7IlNLSU4iOnsidXJsIjoiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9hYmMiLCJtZXRhZGF0YSI6eyJtb2RlbCI6InNsaW0ifX0sIkNBUEUiOnsidXJsIjoiaHR0cDovL3RleHR1cmVzLm1pbmVjcmFmdC5uZXQvdGV4dHVyZS9jYXBlIn19fQ==";
    /// 只有 timestamp 等字段、没有任何纹理的 textures 属性
    const EMPTY: &str = "eyJ0aW1lc3RhbXAiOjE2MDAwMDAwMDAwMDAsInByb2ZpbGVJZCI6IjA2OWE3OWY0NDRlOTQ3MjZhNWJlZmNhOTBlMzhhYWY1IiwicHJvZmlsZU5hbWUiOiJOb3RjaCIsInRleHR1cmVzIjp7fX0=";

    /// 应解析出皮肤（含 slim 标记）与披风地址
    #[test]
    fn test_parse_textures_slim() {
        let obj = parse_textures(SLIM).expect("应能解析");

        let skin = obj.textures.skin.expect("应有皮肤");
        assert_eq!(skin.url, "http://textures.minecraft.net/texture/abc");
        assert_eq!(skin.metadata.expect("应有元数据").model, "slim");

        let cape = obj.textures.cape.expect("应有披风");
        assert_eq!(cape.url, "http://textures.minecraft.net/texture/cape");
    }

    /// 没有纹理时皮肤与披风都应为 `None`
    #[test]
    fn test_parse_textures_empty() {
        let obj = parse_textures(EMPTY).expect("应能解析");

        assert!(obj.textures.skin.is_none());
        assert!(obj.textures.cape.is_none());
    }

    /// 非法 base64 / 非法 JSON 应返回 `None`
    #[test]
    fn test_parse_textures_invalid() {
        assert!(parse_textures("!!!").is_none());
        assert!(parse_textures("aGVsbG8=").is_none());
    }
}
