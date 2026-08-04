//! 添加游戏实例操作
//! 导入文件夹，导入压缩包
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use mcml_base::{archives::BaseArchive, path_helper, serialize_tools};
use mcml_names::{
    i18_items::error_type::{ArgEmptyData, CoreResult, ErrorType},
    names,
};
use tokio_util::sync::CancellationToken;

use crate::{
    GameInstance, game_options, gui_hook::{AddModPackState, IAddGui, IAddInstanceGui, ICopyGui}, launcher::{SourceType, instance_setting_obj::InstanceSettingObj}, launcher_path::version_path, modpack::{
        BaseModPackWorker, ModPackWorker, curseforge_worker::CurseForgeWorker, modrinth_worker::ModrinthPackWorker,
    }, other_launcher::{self, mmc_obj::MMCObj, official_obj::OfficialObj},
};

/// 压缩包类型
pub enum PackType {
    ColorMC,
    CurseForge,
    Modrinth,
    MMC,
    HMCL,
    HMCLServer,
    ArchivePack,
    LauncherPack,
}

/// 导入文件夹
pub async fn add_game_folder<P: AsRef<Path>>(
    dir: P,
    name: Option<String>,
    group: Option<String>,
    unselect: Option<Vec<PathBuf>>,
    gui: Option<Arc<dyn IAddInstanceGui>>,
    copy_gui: Option<Arc<dyn ICopyGui>>,
) -> CoreResult<GameInstance> {
    if !dir.as_ref().exists() || !dir.as_ref().is_dir() {
        return Err(ErrorType::ArgEmpty(ArgEmptyData {
            arg: "dir".to_string(),
        }));
    }

    let mut instance: Option<InstanceSettingObj> = None;
    let mut is_mmc = false;
    //是否为MMC实例
    if other_launcher::is_mmc_version(&dir)
        && let Ok(mmc) =
            serialize_tools::json_from_file::<MMCObj>(dir.as_ref().join(names::MMCJSON_FILE))
    {
        let cfg =
            game_options::read_options_from_file(dir.as_ref().join(names::MMCCFG_FILE), Some('='))?;

        instance = Some(mmc.to_instance(cfg));
        is_mmc = true;
    } else {
        //是否为官启版本
        let files = path_helper::get_files(&dir);
        for item in files {
            if let Some(ext) = item.extension()
                && ext == names::JSON_EXT
            {
                if let Ok(data) = OfficialObj::read_from_file(item) {
                    instance = Some(data.to_instance());
                }
            }
        }
    }

    let mut instance = match instance {
        Some(data) => data,
        None => InstanceSettingObj {
            version: version_path::get_latest_version(),
            group,
            ..Default::default()
        },
    };

    //没有名字使用输入名字，已有名字同时有输入名字则覆盖
    if let Some(name) = &name
        && !name.is_empty()
    {
        instance.name = name.clone();
    }

    if instance.name.is_empty() && name.is_none() {
        return Err(ErrorType::ArgEmpty(ArgEmptyData {
            arg: "name".to_string(),
        }));
    }

    let res = instance.create_instance(&gui).await?;

    res.read()
        .unwrap()
        .copy_files(dir, unselect, is_mmc, &copy_gui)
        .await?;

    Ok(res)
}

/// 从整合包添加实例
async fn modpack<P: AsRef<Path>>(
    source: SourceType,
    file: P,
    group: Option<String>,
    unselect: Option<Vec<String>>,
    gui: Option<Arc<dyn IAddInstanceGui>>,
    pack_gui: Option<Arc<dyn IAddGui>>,
    cancel: CancellationToken,
) -> CoreResult<GameInstance> {
    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::ReadInfo);
        pack_gui.set_now(1, Some(5));
        pack_gui.set_sub_now(0, Some(1));
    }

    let mut work: Box<dyn ModPackWorker> = if source == SourceType::CurseForge {
        Box::new(CurseForgeWorker::new(BaseModPackWorker::new(
            BaseArchive::open(file)?,
            gui,
            pack_gui.clone(),
            Some(cancel.clone()),
        )))
    } else {
        Box::new(ModrinthPackWorker::new(BaseModPackWorker::new(
            BaseArchive::open(file)?,
            gui,
            pack_gui.clone(),
            Some(cancel.clone()),
        )))
    };

    if !work.read_info() || !work.read_version().await {
        return Err(ErrorType::InfoNotFound("info".to_string()));
    }

    if cancel.is_cancelled() {
        return Err(ErrorType::TaskCancel);
    }

    let game = work.create_instance(group).await?;

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Extract);
        pack_gui.set_now(2, Some(5));
        pack_gui.set_sub_now(0, Some(1));
    }

    if !work.extract(unselect).await {

    }

    todo!()
}
