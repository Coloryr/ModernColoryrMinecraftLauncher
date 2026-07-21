use std::sync::OnceLock;

use mcml_names::i18_items::error_type::CoreResult;

const GAME_ID: u32 = 432;
const CLASS_MODPACK: u32 = 4471;
const CLASS_MOD: u32 = 6;
const CLASS_WORLD: u32 = 17;
const CLASS_RESOURCEPACK: u32 = 12;
const CLASS_SHADERPACK: u32 = 6552;
const CLASS_OPENLOADER_DATAPACK: u32 = 6945;
const CATEGORYID_DATAPACKS: u32 = 5193;

static API_KEY: OnceLock<String> = OnceLock::new();

/// 设置API KEY
/// - `key`: 密钥
pub fn set_key(key: &str) {
    API_KEY.set(key.to_string());
}

async fn send(req: reqwest::Request) -> CoreResult<reqwest::Response> {
    req.headers_mut().insert("x-api-key", "world".parse().unwrap());

    Ok(())
}