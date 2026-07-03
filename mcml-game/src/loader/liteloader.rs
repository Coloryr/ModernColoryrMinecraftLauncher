use mcml_base::file_item::FileItemObj;
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use mcml_net::liteloader_api;

use crate::{
    launcher::instance_setting_obj::InstanceSettingObj, loader::liteloader_meta_obj::LiteloaderMetaObj,
};

pub async fn get_liteloader_meta() -> CoreResult<LiteloaderMetaObj> {
    let data = liteloader_api::get_meta().await?;
    let obj = serde_json::from_slice::<LiteloaderMetaObj>(&data).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?;

    Ok(obj)
}

/// 获取liteloader运行库列表
fn get_liteloader_lib(mc: &str, version: &str) -> Vec<FileItemObj> {
    todo!()
}

impl InstanceSettingObj {
    /// 获取liteloader运行库列表
    pub fn get_liteloader_lib(&self) -> Vec<FileItemObj> {
        get_liteloader_lib(&self.version, self.loader_version.as_ref().unwrap())
    }
}
