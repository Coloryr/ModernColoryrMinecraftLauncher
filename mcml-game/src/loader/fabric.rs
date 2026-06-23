use mcml_base::file_item::{FileHash, FileItemObj, LaterRun};
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use mcml_net::{maven_utils::version_name_to_path, fabric_api, url_helper};

use crate::{
    launcher::game_setting_obj::GameSettingObj,
    launcher_path::{libraries_path, version_path},
    loader::{fabric_loader_obj::FabricLoaderObj, fabric_meta_obj::FabricMetaObj},
};

/// 获取Fabric下载项目
/// - `mc`: 游戏版本号
/// - `version`: fabric版本号
pub async fn get_fabric_libs(mc: &str, version: Option<&str>) -> CoreResult<Vec<FileItemObj>> {
    let meta = fabric_api::get_meta().await?;

    let obj = serde_json::from_slice::<FabricMetaObj>(&meta).map_err(|err| {
        ErrorType::JsonError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let fabric = match version {
        Some(version) => obj
            .loader
            .iter()
            .filter(|item| item.version.eq_ignore_ascii_case(version))
            .next(),
        None => obj.loader.iter().filter(|item| item.stable).next(),
    };

    if let Some(fabric) = fabric {
        let data = fabric_api::get_loader(mc, &fabric.version).await?;
        let obj = serde_json::from_slice::<FabricLoaderObj>(&data).map_err(|err| {
            ErrorType::JsonError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let obj = version_path::add_fabric(obj, &data, mc, &fabric.version);

        let mut list = Vec::new();

        for item in &obj.libraries {
            let name = version_name_to_path(&item.name);
            list.push(FileItemObj {
                name: item.name.clone(),
                file: libraries_path::get_lib_dir().join(&name),
                url: url_helper::replace_fabric_libraries(&item.url) + &name,
                hash: FileHash::Sha256(item.sha256.clone()),
                later: LaterRun::None,
            });
        }

        Ok(list)
    } else {
        Err(ErrorType::InfoNotFound)
    }
}

impl GameSettingObj {
    pub async fn get_fabric_libs(&self) -> CoreResult<Vec<FileItemObj>> {
        get_fabric_libs(
            &self.version,
            self.loader_version.as_ref().map(|x| x.as_str()),
        )
        .await
    }
}
