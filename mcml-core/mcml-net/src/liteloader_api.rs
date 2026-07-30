//! LiteLoader 模组加载器 API
//!
//! LiteLoader 是一个轻量级的 Minecraft 模组加载器，与 Forge 共存。

use mcml_names::i18_items::error_type::CoreResult;

use crate::{WORK_CLIENT, urls};

/// 获取 LiteLoader 版本元数据（可用版本列表）
pub async fn get_meta() -> CoreResult<Vec<u8>> {
    let url = format!("{}versions/versions.json", urls::LITELOADER);

    WORK_CLIENT.get().unwrap().get_bytes(&url).await
}
