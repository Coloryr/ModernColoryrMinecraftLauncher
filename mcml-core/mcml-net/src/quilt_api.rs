//! Quilt 模组加载器 API
//!
//! 提供从 Quilt Meta API 获取加载器版本信息和下载配置的功能。
//! Quilt 是 Fabric 的一个分支，API 结构类似但不兼容。

use mcml_names::i18_items::error_type::CoreResult;

use crate::{WORK_CLIENT, url_helper};

/// 获取 Quilt 加载器安装配置（profile JSON）
///
/// # 参数
///
/// - `mc`: Minecraft 游戏版本
/// - `version`: Quilt Loader 版本
pub async fn get_loader(mc: &str, version: &str) -> CoreResult<Vec<u8>> {
    let url = format!(
        "{}/loader/{mc}/{version}/profile/json",
        url_helper::get_quilt_meta()
    );

    WORK_CLIENT.get().unwrap().get_bytes(&url).await
}

/// 获取 Quilt 元数据（可用版本列表）
pub async fn get_meta() -> CoreResult<Vec<u8>> {
    let url = url_helper::get_quilt_meta();

    WORK_CLIENT.get().unwrap().get_bytes(&url).await
}