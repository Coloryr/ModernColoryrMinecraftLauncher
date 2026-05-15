use mcml_names::i18_items::error_type::ErrorType;
use mcml_net::url_helper;

pub async fn get_loader(mc: &String, version: &String) -> Result<Vec<u8>, ErrorType> {
    let url = format!(
        "{}/loader/{mc}/{version}/profile/json",
        url_helper::get_fabric_meta()
    );

    mcml_http::WORK_CLIENT.get().unwrap().get_bytes(&url).await
}
