use mcml_config::config_obj::SourceLocal;
use mcml_names::i18_items::error_type::CoreResult;
use mcml_net::url_helper;

/// 直接下载资源
pub async fn get_assets(url: &String) -> CoreResult<Vec<u8>> {
    mcml_http::WORK_CLIENT.get().unwrap().get_bytes(url).await
}

/// 获取主版本列表
pub async fn get_versions(source: Option<SourceLocal>) -> CoreResult<Vec<u8>> {
    let url = url_helper::game_version(source);
    mcml_http::WORK_CLIENT.get().unwrap().get_bytes(&url).await
}
