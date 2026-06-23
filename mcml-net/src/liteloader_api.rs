use mcml_names::i18_items::error_type::CoreResult;

use crate::{WORK_CLIENT, urls};

pub async fn get_meta() -> CoreResult<Vec<u8>> {
    let url = format!("{}versions/versions.json", urls::LITELOADER);

    WORK_CLIENT.get().unwrap().get_bytes(&url).await
}
