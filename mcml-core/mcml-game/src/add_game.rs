//! 添加游戏实例操作
//! 导入文件夹，导入压缩包
use std::path::{Path, PathBuf};

use mcml_base::{archives::BaseArchive, path_helper, serialize_tools};
use mcml_names::{
    i18_items::error_type::{
        ArgEmptyData, CoreResult, DataNotFoundData, ErrorType, PathNotExistsData,
    },
    names,
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    GameInstance,
    curseforge::pack_obj::CurseForgePackObj,
    game_options,
    gui_hook::{AddInstanceGui, AddModPackGui, AddModPackState, BaseArchiveGui, ProgressGui},
    launcher::{
        ModPackType,
        instance_setting_obj::{CustomLoaderObj, InstanceSettingObj},
    },
    launcher_path::version_path,
    modpack::{
        BaseModPackWorker, ModPackWorker, curseforge_worker::CurseForgeWorker,
        modrinth_worker::ModrinthPackWorker,
    },
    other_launcher::{
        self,
        hmcl_obj::{HMCLObj, HMCLServerObj},
        mmc_obj::MMCObj,
        official_obj::OfficialObj,
    },
};

/// 压缩包类型
pub enum PackType {
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
    instance_gui: AddInstanceGui,
    progress_gui: ProgressGui,
    cancel: CancellationToken,
) -> CoreResult<GameInstance> {
    if !dir.as_ref().exists() || !dir.as_ref().is_dir() {
        return Err(ErrorType::DirNotExists(PathNotExistsData {
            path: dir.as_ref().to_path_buf(),
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
        return Err(ErrorType::ArgEmpty(ArgEmptyData::Name));
    }

    if cancel.is_cancelled() {
        return Err(ErrorType::TaskCancel);
    }

    let res = instance.create_instance(instance_gui.clone()).await?;

    res.read()
        .unwrap()
        .copy_files(dir, unselect, is_mmc, progress_gui.clone())
        .await?;

    Ok(res)
}

/// 从整合包添加实例
async fn modpack<P: AsRef<Path>>(
    file: P,
    source: ModPackType,
    group: Option<String>,
    unselect: Option<Vec<String>>,
    instance_gui: AddInstanceGui,
    pack_gui: AddModPackGui,
    archive_gui: BaseArchiveGui,
    cancel: CancellationToken,
) -> CoreResult<Uuid> {
    if !file.as_ref().exists() || file.as_ref().is_dir() {
        return Err(ErrorType::FileNotExists(PathNotExistsData {
            path: file.as_ref().to_path_buf(),
        }));
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::ReadInfo);
        pack_gui.set_now(1, Some(5));
        pack_gui.set_sub_now(0, Some(1));
    }

    let mut work: Box<dyn ModPackWorker> = if source == ModPackType::CurseForge {
        Box::new(CurseForgeWorker::new(BaseModPackWorker::new(
            BaseArchive::open(file)?,
            instance_gui,
            pack_gui.clone(),
            archive_gui,
            cancel.clone(),
        )))
    } else {
        Box::new(ModrinthPackWorker::new(BaseModPackWorker::new(
            BaseArchive::open(file)?,
            instance_gui,
            pack_gui.clone(),
            archive_gui,
            cancel.clone(),
        )))
    };

    work.read_info()?;
    work.read_version().await?;

    if cancel.is_cancelled() {
        return Err(ErrorType::TaskCancel);
    }

    let uuid = work.create_instance(group).await?;

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Extract);
        pack_gui.set_now(2, Some(5));
        pack_gui.set_sub_now(0, Some(1));
    }

    work.extract(unselect).await?;

    if cancel.is_cancelled() {
        crate::delete_instance(&uuid)?;
        return Err(ErrorType::TaskCancel);
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::GetInfo);
        pack_gui.set_sub_text(None);
        pack_gui.set_sub_now(0, None);
        pack_gui.set_now(3, Some(5));
        pack_gui.set_sub_now(0, None);
    }

    work.get_info().await?;

    if cancel.is_cancelled() {
        crate::delete_instance(&uuid)?;
        return Err(ErrorType::TaskCancel);
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::DownloadFile);
        pack_gui.set_sub_text(None);
        pack_gui.set_now(4, Some(5));
        pack_gui.set_sub_now(0, None);
    }

    work.download().await;

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Done);
        pack_gui.set_now(5, Some(5));
    }

    Ok(uuid)
}

/// 解压整合包压缩包到实例目录，统一处理压缩包整体套一层文件夹的情况。
///
/// * `output_dir` — 解压目标目录（实例基础目录）。
/// * `unselect` — 按压缩包内完整条目名排除的文件列表。
/// * `strip_dir` — 压缩包整体套的顶层目录名（不剥时传 `None`）。
/// * `gui` — 可选的进度回调。
fn extract_pack<P: AsRef<Path>>(
    archive: &BaseArchive,
    output_dir: P,
    unselect: Vec<String>,
    strip_dir: Option<String>,
    archive_gui: BaseArchiveGui,
) -> CoreResult<()> {
    archive.extract_all(output_dir, Some(unselect), strip_dir, archive_gui)
}

/// 直接解压
async fn archive<P: AsRef<Path>>(
    file: P,
    name: Option<String>,
    group: Option<String>,
    unselect: Vec<String>,
    instance_gui: AddInstanceGui,
    pack_gui: AddModPackGui,
    archive_gui: BaseArchiveGui,
    cancel: CancellationToken,
) -> CoreResult<Uuid> {
    if !file.as_ref().exists() || file.as_ref().is_dir() {
        return Err(ErrorType::FileNotExists(PathNotExistsData {
            path: file.as_ref().to_path_buf(),
        }));
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::ReadInfo);
        pack_gui.set_now(1, Some(3));
    }

    let archive = BaseArchive::open(file)?;
    // 找到 game.json 所在条目，兼容压缩包整体套一层文件夹的情况
    let game_entry = archive
        .entries()
        .iter()
        .find(|e| !e.is_dir && e.name.ends_with(names::GAME_FILE))
        .ok_or_else(|| ErrorType::DataNotFound(DataNotFoundData::Info))?;
    let data = archive.read(&game_entry.name)?;
    let mut obj = serialize_tools::json_from_bytes::<InstanceSettingObj>(&data)?;

    if let Some(name) = name {
        obj.name = name;
    }

    if let Some(group) = group {
        obj.group = Some(group);
    }

    let game = obj.create_instance(instance_gui).await?;
    let uuid = game.read().unwrap().uuid;
    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Extract);
        pack_gui.set_now(2, Some(3));
    }

    if cancel.is_cancelled() {
        crate::delete_instance(&uuid)?;
        return Err(ErrorType::TaskCancel);
    }

    // game.json 所在目录即实例包裹目录，解压时去掉该层，直接放进实例目录
    let strip_dir = game_entry
        .name
        .strip_suffix(names::GAME_FILE)
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches(['/', '\\']).to_string());

    extract_pack(
        &archive,
        game.read().unwrap().get_base_path(),
        unselect,
        strip_dir,
        archive_gui,
    )?;

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Done);
        pack_gui.set_now(3, Some(3));
    }

    Ok(uuid)
}

/// 导入MMC压缩包
async fn mmc_archive<P: AsRef<Path>>(
    file: P,
    name: Option<String>,
    group: Option<String>,
    unselect: Vec<String>,
    instance_gui: AddInstanceGui,
    pack_gui: AddModPackGui,
    archive_gui: BaseArchiveGui,
    cancel: CancellationToken,
) -> CoreResult<Uuid> {
    if !file.as_ref().exists() || file.as_ref().is_dir() {
        return Err(ErrorType::FileNotExists(PathNotExistsData {
            path: file.as_ref().to_path_buf(),
        }));
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::ReadInfo);
        pack_gui.set_now(1, Some(3));
    }

    let archive = BaseArchive::open(&file)?;

    let mut path = String::new();
    let mut mmc = None;
    let mut cfg = None;

    for item in archive.entries() {
        if item.is_dir {
            continue;
        } else if mmc.is_none() && item.name.ends_with(names::MMCJSON_FILE) {
            path = item.name.replace(names::MMCJSON_FILE, "");
            let data = archive.read(&item.name)?;
            let obj = serialize_tools::json_from_bytes::<MMCObj>(&data)?;
            mmc = Some(obj);
        } else if cfg.is_none() && item.name.ends_with(names::MMCCFG_FILE) {
            let data = archive.read_stream(&item.name)?;
            cfg = Some(game_options::read_options(data, Some('='))?);
        }

        if mmc.is_some() && cfg.is_some() {
            break;
        }
    }

    if mmc.is_none() || cfg.is_none() {
        return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
    }

    let mut obj = mmc.unwrap().to_instance(cfg.unwrap());
    if let Some(name) = name {
        obj.name = name;
    }

    if let Some(group) = group {
        obj.group = Some(group);
    }

    if obj.name.is_empty() {
        obj.name = file
            .as_ref()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
    }

    if cancel.is_cancelled() {
        return Err(ErrorType::TaskCancel);
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Extract);
        pack_gui.set_now(2, Some(3));
    }

    let game = obj.create_instance(instance_gui).await?;
    let uuid = game.read().unwrap().uuid;

    if cancel.is_cancelled() {
        crate::delete_instance(&uuid)?;
        return Err(ErrorType::TaskCancel);
    }

    // MMC 元数据按完整条目名加入排除列表
    let mut unselect = unselect;
    unselect.push(format!("{path}{}", names::MMCJSON_FILE));
    unselect.push(format!("{path}{}", names::MMCCFG_FILE));

    // 解压到实例基础目录，`.minecraft/` 下的内容自然落到游戏目录（base/.minecraft）。
    // mmc-pack.json 所在目录即包裹层，解压时去掉
    let strip_dir = path.trim_end_matches(['/', '\\']).to_string();
    extract_pack(
        &archive,
        game.read().unwrap().get_base_path(),
        unselect,
        (!strip_dir.is_empty()).then_some(strip_dir),
        archive_gui.clone(),
    )?;

    let json = game.read().unwrap().read_custom_json();
    if !json.is_empty() {
        game.write().unwrap().custom_loader = Some(CustomLoaderObj {
            custom_json: true,
            ..Default::default()
        });
        game.read().unwrap().save();
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Done);
        pack_gui.set_now(3, Some(3));
    }

    Ok(uuid)
}

/// HMCL压缩包
async fn hmcl_archive<P: AsRef<Path>>(
    file: P,
    name: Option<String>,
    group: Option<String>,
    unselect: Vec<String>,
    instance_gui: AddInstanceGui,
    pack_gui: AddModPackGui,
    archive_gui: BaseArchiveGui,
    cancel: CancellationToken,
) -> CoreResult<Uuid> {
    if !file.as_ref().exists() || file.as_ref().is_dir() {
        return Err(ErrorType::FileNotExists(PathNotExistsData {
            path: file.as_ref().to_path_buf(),
        }));
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::ReadInfo);
        pack_gui.set_now(1, Some(3));
    }

    let archive = BaseArchive::open(&file)?;

    let mut path = String::new();
    let mut hmcl = None;
    let mut cfg = None;

    for item in archive.entries() {
        if item.is_dir {
            continue;
        } else if hmcl.is_none() && item.name.ends_with(names::HMCLFILE) {
            path = item.name.replace(names::MMCJSON_FILE, "");
            let data = archive.read(&item.name)?;
            let obj = serialize_tools::json_from_bytes::<HMCLObj>(&data)?;
            hmcl = Some(obj);
        } else if cfg.is_none() && item.name.ends_with(names::MANIFEST_FILE) {
            let data = archive.read(&item.name)?;
            let obj = serialize_tools::json_from_bytes::<CurseForgePackObj>(&data)?;
            cfg = Some(obj);
        }

        if hmcl.is_some() && cfg.is_some() {
            break;
        }
    }

    if hmcl.is_none() || cfg.is_none() {
        return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
    }

    let mut obj = hmcl.unwrap().to_instance();
    if let Some(name) = name {
        obj.name = name;
    }

    if let Some(group) = group {
        obj.group = Some(group);
    }

    if obj.name.is_empty() {
        obj.name = file
            .as_ref()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
    }

    if cancel.is_cancelled() {
        return Err(ErrorType::TaskCancel);
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Extract);
        pack_gui.set_now(2, Some(3));
    }

    let game = obj.create_instance(instance_gui).await?;
    let uuid = game.read().unwrap().uuid;

    if cancel.is_cancelled() {
        crate::delete_instance(&uuid)?;
        return Err(ErrorType::TaskCancel);
    }

    // HMCL 元数据按完整条目名加入排除列表
    let mut unselect = unselect;
    unselect.push(format!("{path}{}", names::HMCLFILE));

    let over = if let Some(cfg) = cfg {
        cfg.overrides.clone()
    } else {
        names::OVERRIDE_DIR.to_string()
    };

    let mut strip_dir = path.trim_end_matches(['/', '\\']).to_string();
    let dir = if !strip_dir.is_empty() {
        over
    } else {
        strip_dir.push_str(&over);
        strip_dir
    };

    extract_pack(
        &archive,
        game.read().unwrap().get_base_path(),
        unselect,
        Some(dir),
        archive_gui.clone(),
    )?;

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Done);
        pack_gui.set_now(3, Some(3));
    }

    Ok(uuid)
}

/// HMCL服务器包
async fn hmcl_server_archive<P: AsRef<Path>>(
    file: P,
    name: Option<String>,
    group: Option<String>,
    unselect: Vec<String>,
    instance_gui: AddInstanceGui,
    pack_gui: AddModPackGui,
    archive_gui: BaseArchiveGui,
    cancel: CancellationToken,
) -> CoreResult<Uuid> {
    if !file.as_ref().exists() || file.as_ref().is_dir() {
        return Err(ErrorType::FileNotExists(PathNotExistsData {
            path: file.as_ref().to_path_buf(),
        }));
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::ReadInfo);
        pack_gui.set_now(1, Some(3));
    }

    let archive = BaseArchive::open(&file)?;

    let mut path = String::new();
    let mut hmcl = None;

    for item in archive.entries() {
        if item.is_dir {
            continue;
        } else if hmcl.is_none() && item.name.ends_with(names::SERVER_MANIFEST_FILE) {
            path = item.name.replace(names::SERVER_MANIFEST_FILE, "");
            let data = archive.read(&item.name)?;
            let obj = serialize_tools::json_from_bytes::<HMCLServerObj>(&data)?;
            hmcl = Some(obj);
            break;
        }
    }

    if hmcl.is_none() {
        return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
    }

    let hmcl = hmcl.unwrap();

    let mut obj = hmcl.to_instance();
    if let Some(name) = name {
        obj.name = name;
    }

    if let Some(group) = group {
        obj.group = Some(group);
    }

    if obj.name.is_empty() {
        obj.name = file
            .as_ref()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
    }

    if cancel.is_cancelled() {
        return Err(ErrorType::TaskCancel);
    }

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Extract);
        pack_gui.set_now(2, Some(3));
    }

    let game = obj.create_instance(instance_gui).await?;
    let uuid = game.read().unwrap().uuid;

    let mut online = game.read().unwrap().read_online_info();

    if !hmcl.files.is_empty() && !hmcl.file_api.is_empty() {
        let url = if hmcl.file_api.ends_with('/') {
            format!("{}/", hmcl.file_api)
        } else {
            hmcl.file_api
        };
        for item in hmcl.files {}
    }

    if cancel.is_cancelled() {
        crate::delete_instance(&uuid)?;
        return Err(ErrorType::TaskCancel);
    }

    // HMCL 元数据按完整条目名加入排除列表
    let mut unselect = unselect;
    unselect.push(format!("{path}{}", names::HMCLFILE));

    let over = names::OVERRIDE_DIR.to_string();
    let mut strip_dir = path.trim_end_matches(['/', '\\']).to_string();
    let dir = if !strip_dir.is_empty() {
        over
    } else {
        strip_dir.push_str(&over);
        strip_dir
    };

    extract_pack(
        &archive,
        game.read().unwrap().get_base_path(),
        unselect,
        Some(dir),
        archive_gui.clone(),
    )?;

    if let Some(pack_gui) = &pack_gui {
        pack_gui.set_state(AddModPackState::Done);
        pack_gui.set_now(3, Some(3));
    }

    Ok(uuid)
}
