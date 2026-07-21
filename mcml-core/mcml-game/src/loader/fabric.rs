use mcml_base::{
    file_item::{FileHash, FileItemObj, LaterRun},
    serialize_tools,
};
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use mcml_net::{fabric_api, maven_utils::version_name_to_path, url_helper};

use crate::{
    launcher::instance_setting_obj::InstanceSettingObj,
    launcher_path::{libraries_path, version_path},
    loader::{fabric_loader_obj::FabricLoaderObj, fabric_meta_obj::FabricMetaObj},
};

/// 获取Fabric下载项目
/// - `mc`: 游戏版本号
/// - `version`: fabric版本号
pub async fn get_fabric_libs(mc: &str, version: Option<&str>) -> CoreResult<Vec<FileItemObj>> {
    let meta = fabric_api::get_meta().await?;

    let obj = serialize_tools::json_from_bytes::<FabricMetaObj>(&meta)?;

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
        let obj = serialize_tools::json_from_bytes::<FabricLoaderObj>(&data)?;
        let obj = version_path::add_fabric(obj, &data, mc, &fabric.version);
        Ok(obj.make_libs())
    } else {
        Err(ErrorType::InfoNotFound(mc.to_string()))
    }
}

impl FabricLoaderObj {
    /// 生成运行库列表
    pub fn make_libs(&self) -> Vec<FileItemObj> {
        let mut list = Vec::new();

        for item in &self.libraries {
            let name = version_name_to_path(&item.name);
            list.push(FileItemObj {
                name: item.name.clone(),
                file: libraries_path::get_lib_dir().join(&name),
                url: url_helper::replace_fabric_libraries(&item.url) + &name,
                hash: FileHash::Sha256(item.sha256.clone()),
                later: LaterRun::None,
            });
        }

        list
    }
}

impl InstanceSettingObj {
    /// 获取fabric的所有运行库
    pub async fn get_fabric_libs(&self) -> CoreResult<Vec<FileItemObj>> {
        let fabric =
            version_path::get_fabric(&self.version, &self.loader_version.as_ref().unwrap());
        match fabric {
            Some(fabric) => Ok(fabric.make_libs()),
            None => {
                get_fabric_libs(
                    &self.version,
                    self.loader_version.as_ref().map(|x| x.as_str()),
                )
                .await
            }
        }
    }
}
