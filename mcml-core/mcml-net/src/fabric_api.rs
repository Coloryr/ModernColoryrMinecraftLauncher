use mcml_names::i18_items::error_type::CoreResult;

use crate::{WORK_CLIENT, url_helper};

/// 获取加载器信息
/// - `mc`: 游戏版本
/// - `version`: 加载器版本
pub async fn get_loader(mc: &str, version: &str) -> CoreResult<Vec<u8>> {
    let url = format!(
        "{}/loader/{mc}/{version}/profile/json",
        url_helper::get_fabric_meta()
    );

    WORK_CLIENT.get().unwrap().get_bytes(&url).await
}

/// 获取元数据
pub async fn get_meta() -> CoreResult<Vec<u8>> {
    let url = url_helper::get_fabric_meta();

    WORK_CLIENT.get().unwrap().get_bytes(&url).await
}