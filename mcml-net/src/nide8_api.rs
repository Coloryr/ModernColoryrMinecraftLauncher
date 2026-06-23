use mcml_names::{i18_items::error_type::CoreResult};
use serde::{Deserialize, Serialize};

use crate::{WORK_CLIENT, urls};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Nide8Obj {
    #[serde(rename = "jarVersion")]
    pub jar_version: String,
    #[serde(rename = "jarHash")]
    pub jar_hash: String,
}

impl Default for Nide8Obj {
    fn default() -> Self {
        Self {
            jar_version: Default::default(),
            jar_hash: Default::default(),
        }
    }
}

/// 获取jar
pub async fn get_obj() -> CoreResult<Nide8Obj> {
    WORK_CLIENT
        .get()
        .unwrap()
        .get_json::<Nide8Obj>(&format!(
            "{}00000000000000000000000000000000/",
            urls::NIDE8_URL
        ))
        .await
}
