use std::{collections::HashMap, path::PathBuf, sync::Arc};

use mcml_base::{
    archives::{self, ArchiveGui, ArchiveType, BaseArchive},
    file_item::FileHash,
    serialize_tools,
};
use mcml_names::{i18_items::error_type::CoreResult, names};

use crate::{
    curseforge::curseforge_pack_obj::{CurseForgePackObj, FilesObj, MinecraftObj, ModLoadersObj},
    launcher::{file_online_info_obj::FileOnlineInfoObj, instance_setting_obj::InstanceSettingObj},
    loader::LoaderType,
};

/// 导出压缩包类型
pub enum ExportPackType {
    ColorMC,
    CurseForge,
    Modrinth,
    Zip,
}

/// 导出的在线文件
pub struct OnlineFileExport {
    /// 文件路径
    pub file: PathBuf,
    /// 文件大小
    pub size: usize,
    /// 导出方式
    pub pack: ExportPackType,
    /// 下载地址
    pub url: String,
    /// 校验
    pub hash: FileHash,
    /// 文件在线信息
    pub info: Option<FileOnlineInfoObj>,
}

/// 导出需要的参数
pub struct ExportArg {
    /// 导出保存的位置
    pub file: PathBuf,
    /// 打包类型
    pub pack: ExportPackType,
    /// 压缩类型
    pub archive: ArchiveType,
    /// 在线模组信息
    pub mods: Vec<OnlineFileExport>,
    /// 在线文件信息
    pub files: Vec<OnlineFileExport>,
    /// 不打包的文件
    pub unselect: Vec<PathBuf>,
    /// 一起打包的文件
    pub select: Vec<PathBuf>,
    /// 名字
    pub name: String,
    /// 作者
    pub author: String,
    /// 版本
    pub version: String,
    /// 说明
    pub summary: String,
    /// 压缩进度条
    pub gui: Option<Arc<dyn ArchiveGui>>,
}

impl InstanceSettingObj {
    pub async fn export(&self, data: ExportArg) -> CoreResult<()> {
        match data.pack {
            ExportPackType::ColorMC => colormc(self, data),
            ExportPackType::CurseForge => curseforge(self, data),
            ExportPackType::Modrinth => todo!(),
            ExportPackType::Zip => todo!(),
        }
    }
}

fn colormc(game: &InstanceSettingObj, data: ExportArg) -> CoreResult<()> {
    let mut list = data.unselect;
    list.push(game.get_online_info_file());

    let list = list
        .iter()
        .map(|item| item.to_string_lossy().to_string())
        .collect();

    let mut list1 = HashMap::new();
    for item in data.mods.iter() {
        if let Some(info) = &item.info {
            list1.insert(info.modid.clone(), info.clone());
        }
    }

    archives::compress(
        data.archive,
        &data.file,
        &game.get_base_path(),
        None,
        &Some(list),
        data.gui.clone(),
    )?;

    let mut archive = BaseArchive::open(&data.file)?;
    archive.add_data(
        names::MOD_INFO_FILE,
        &serialize_tools::json_to_bytes(&list1)?,
        data.gui,
    )?;

    Ok(())
}

fn curseforge(game: &InstanceSettingObj, data: ExportArg) -> CoreResult<()> {
    let mut obj = CurseForgePackObj {
        name: data.name,
        author: data.author,
        version: data.version,
        manifest_type: "minecraftModpack".to_string(),
        manifest_version: 1,
        overrides: names::OVERRIDE_DIR.to_string(),
        minecraft: MinecraftObj {
            version: game.version.clone(),
            mod_loaders: Vec::new(),
        },
        ..Default::default()
    };

    if game.loader != LoaderType::Normal {
        obj.minecraft.mod_loaders.push(ModLoadersObj {
            id: format!(
                "{}-{}",
                game.loader.prefix(),
                game.loader_version.clone().unwrap_or_default()
            ),
            primary: true,
        });
    }

    for item in data.mods {
        if let Some(info) = item.info {
            obj.files.push(FilesObj {
                file_id: info.fileid.parse::<i64>().unwrap_or_default(),
                project_id: info.modid.parse::<i64>().unwrap_or_default(),
                required: true,
            });
        }
    }

    

    Ok(())
}
