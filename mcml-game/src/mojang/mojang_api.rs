use mcml_config::config_obj::SourceLocal;
use mcml_names::i18_items::error_type::ErrorType;
use mcml_net::url_helper;

pub async fn get_assets(url: &String) -> Result<Vec<u8>, ErrorType> {
    mcml_http::WORK_CLIENT
        .get()
        .unwrap()
        .get_bytes(url)
        .await
}

pub async fn get_versions(source: Option<SourceLocal>) -> Result<Vec<u8>, ErrorType> {
    let url = url_helper::game_version(source);
    mcml_http::WORK_CLIENT
        .get()
        .unwrap()
        .get_bytes(&url)
        .await
}
