use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{Arc, Mutex, OnceLock},
};

use mcml_base::{
    archives::BaseArchive,
    file_item::{FileHash, FileItemObj, LaterRun},
    tools,
};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorType},
    names,
};
use mcml_net::{
    modrinth_api::{
        self, ModrinthCategoriesObj,
        version_obj::{DependencieObj, ModrinthVersionObj},
    },
    urls,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use tokio_util::sync::CancellationToken;

use crate::{
    GameInstance,
    data_res::DownloadItemRes,
    gui_hook::{AddModPackState, IAddGui},
    launcher::{
        FileType, file_online_info_obj::OnlineInfoObj, instance_setting_obj::InstanceSettingObj,
    },
    loader::LoaderType,
    modpack::{BaseModPackWorker, ModPackWorker, modrinth_worker::ModrinthPackWorker},
    modrinth::pack_obj::{ModrinthPackFileObj, ModrinthPackObj},
};

pub mod pack_obj;

static CATEGORISE: OnceLock<Vec<ModrinthCategoriesObj>> = OnceLock::new();
static GAME_VERSIONS: OnceLock<Vec<String>> = OnceLock::new();

/// 创建下载项目
pub fn make_download_obj<P: AsRef<Path>>(obj: &ModrinthVersionObj, path: P) -> FileItemObj {
    let file = obj
        .files
        .iter()
        .filter(|item| item.primary)
        .next()
        .unwrap_or(&obj.files[0]);
    FileItemObj {
        name: obj.name.clone(),
        file: path.as_ref().join(&file.filename),
        url: file.url.clone(),
        hash: FileHash::Sha1Sha512(file.hashes.sha1.clone(), file.hashes.sha512.clone()),
        later: LaterRun::None,
    }
}

/// 创建下载项目
pub fn make_pack_download_obj<P: AsRef<Path>>(obj: &ModrinthPackFileObj, path: P) -> FileItemObj {
    FileItemObj {
        url: obj.downloads[0].clone(),
        name: obj.path.clone(),
        file: path.as_ref().join(&obj.path),
        hash: FileHash::Sha1Sha512(obj.hashes.sha1.clone(), obj.hashes.sha512.clone()),
        ..Default::default()
    }
}

/// 创建在线文件信息
pub fn make_file_online_obj(obj: &ModrinthVersionObj, path: &str) -> OnlineInfoObj {
    let file = obj
        .files
        .iter()
        .filter(|item| item.primary)
        .next()
        .unwrap_or(&obj.files[0]);
    OnlineInfoObj {
        path: path.to_string(),
        name: obj.name.clone(),
        file: file.filename.clone(),
        sha1: file.hashes.sha1.clone(),
        url: file.url.clone(),
        modid: obj.project_id.clone(),
        fileid: obj.id.clone(),
    }
}

/// 获取分组
pub async fn get_categories(file_type: FileType) -> CoreResult<HashMap<String, String>> {
    let cap = match CATEGORISE.get() {
        Some(cap) => cap,
        None => {
            let list = modrinth_api::get_categories().await?;
            CATEGORISE.get_or_init(|| list)
        }
    };

    let project_type = match file_type {
        FileType::Shaderpack => modrinth_api::CLASS_SHADERPACK,
        FileType::Resourcepack => modrinth_api::CLASS_RESOURCEPACK,
        FileType::Modpack => modrinth_api::CLASS_MODPACK,
        _ => modrinth_api::CLASS_MOD,
    };

    let map = cap
        .iter()
        .filter(|item| item.project_type == project_type && item.header == "categories")
        .map(|item| (item.name.clone(), item.name.clone()))
        .collect();

    Ok(map)
}

/// 获取所有游戏版本
pub async fn get_game_versions() -> CoreResult<Vec<String>> {
    match GAME_VERSIONS.get() {
        Some(versions) => Ok(versions.clone()),
        None => {
            let list = modrinth_api::get_game_versions().await?;
            let mut list1: Vec<String> = list.into_iter().map(|item| item.version).collect();

            list1.insert(0, String::new());

            Ok(GAME_VERSIONS.get_or_init(|| list1).clone())
        }
    }
}

/// 获取整合包模组信息
pub async fn get_mod_info<P: AsRef<Path>>(
    path: P,
    info: &ModrinthPackObj,
    gui: &Option<Arc<dyn IAddGui>>,
    cancel: Option<CancellationToken>,
) -> CoreResult<DownloadItemRes> {
    let mut list = Vec::new();
    let mut mods = HashMap::new();

    let size = info.files.len();
    let mut now = 0usize;

    let mut hash = HashMap::new();

    for item in info.files.iter() {
        if let Some(cancel) = &cancel
            && cancel.is_cancelled()
        {
            return Err(ErrorType::TaskCancel);
        }

        let item1 = make_pack_download_obj(item, &path);
        let url = item
            .downloads
            .iter()
            .filter(|item| item.starts_with(&format!("{}data/", urls::MODRINTH_DOWNLOAD)))
            .next();

        let sha1 = item.hashes.sha1.clone();
        let path = tools::get_path_part(&item.path);
        // 有隐藏信息
        if let Some(data) = &item.project {
            mods.remove(&data.pid);
            mods.insert(
                data.pid.clone(),
                OnlineInfoObj {
                    path: path.parent,
                    name: path.file.clone(),
                    file: path.file,
                    sha1,
                    url: item1.url.clone(),
                    modid: data.pid.clone(),
                    fileid: data.fid.clone(),
                },
            );
        } else if let Some(url) = url {
            // 是modr的标准下载地址
            let modid = tools::get_string(url, "data/", "/ver");
            let fileid = tools::get_string(url, "versions/", "/");

            mods.remove(&modid);
            mods.insert(
                modid.clone(),
                OnlineInfoObj {
                    path: path.parent,
                    name: path.file.clone(),
                    file: path.file,
                    sha1,
                    url: item1.url.clone(),
                    modid,
                    fileid,
                },
            );
        } else {
            // 尝试从hash获取版本信息
            hash.insert(
                item.hashes.sha512.clone(),
                OnlineInfoObj {
                    path: path.parent,
                    name: path.file.clone(),
                    file: path.file,
                    sha1,
                    url: item1.url.clone(),
                    ..Default::default()
                },
            );
        }

        now += 1;

        if let Some(gui) = gui {
            gui.set_sub_now(now, Some(size));
        }

        list.push(item1);
    }

    if !hash.is_empty() {
        let ids = hash.keys().map(|item| item.clone()).collect();
        let data = modrinth_api::get_version_from_sha512(ids).await;
        if let Ok(data) = data {
            for (key, value) in data {
                let mut temp = hash.remove(&key).unwrap();
                temp.modid = value.project_id;
                temp.fileid = value.id;

                mods.remove(&temp.modid);
                mods.insert(temp.modid.clone(), temp);
            }
        }
    }

    Ok(DownloadItemRes { list, online: mods })
}

/// 模组依赖列表
pub struct ModrinthModDependenciesRes {
    /// 名字
    pub name: String,
    /// 项目编号
    pub mod_id: String,
    /// 是否可选
    pub opt: bool,
    /// 文件列表
    pub list: Vec<ModrinthVersionObj>,
}

/// 获取模组依赖
///
/// - `obj`: Modrinth文件信息
/// - `version`: 游戏版本
/// - `loader`: 加载器类型
pub async fn get_mod_dependencies(
    obj: &ModrinthVersionObj,
    version: &str,
    loader: LoaderType,
) -> Vec<ModrinthModDependenciesRes> {
    let ids = Mutex::new(HashSet::new());
    let handle = tokio::runtime::Handle::current();
    let dependencies = obj.dependencies.clone();
    let version = version.to_string();

    tokio::task::spawn_blocking(move || {
        get_mod_dependencies_inner(&dependencies, &version, &loader, &ids, &handle)
    })
    .await
    .unwrap()
}

fn get_mod_dependencies_inner(
    dependencies: &Vec<DependencieObj>,
    version: &str,
    loader: &LoaderType,
    ids: &Mutex<HashSet<String>>,
    handle: &tokio::runtime::Handle,
) -> Vec<ModrinthModDependenciesRes> {
    let list: Mutex<Vec<ModrinthModDependenciesRes>> = Mutex::new(Vec::new());

    dependencies.par_iter().for_each(|item| {
        // 原子检查并插入：HashSet::insert 在元素已存在时返回 false
        {
            let mut ids_guard = ids.lock().unwrap();
            if !ids_guard.insert(item.project_id.clone()) {
                return;
            }
        }

        let id = item.project_id.clone();
        let opt = !item.dependency_type.eq_ignore_ascii_case("required");

        let (res1, res2) = handle.block_on(async {
            let res1 = modrinth_api::get_project(&id).await;

            match res1 {
                Ok(data) => match &item.version_id {
                    Some(version) => (
                        Ok(data),
                        modrinth_api::get_version(&item.project_id, &version).await,
                    ),
                    None => {
                        let loader = if matches!(loader, LoaderType::Custom)
                            || matches!(loader, LoaderType::Normal)
                            || matches!(loader, LoaderType::OptiFine)
                        {
                            None
                        } else {
                            Some(loader.prefix())
                        };
                        let data1 = modrinth_api::get_file_versions(
                            &item.project_id,
                            Some(version),
                            loader,
                        )
                        .await;

                        match data1 {
                            Ok(mut data1) => (Ok(data), Ok(data1.remove(0))),
                            Err(err) => (Ok(data), Err(err)),
                        }
                    }
                },
                Err(err) => (Err(err), Err(ErrorType::InvalidOperation)),
            }
        });

        let Ok(data) = res1 else { return };
        let Ok(data1) = res2 else { return };

        let sub_deps =
            get_mod_dependencies_inner(&data1.dependencies, version, loader, ids, handle);

        list.lock().unwrap().push(ModrinthModDependenciesRes {
            name: data.title,
            mod_id: data.id.clone(),
            opt: !opt,
            list: vec![data1],
        });

        // 添加未被标记的子依赖（避免同时持有 ids 和 list 锁）
        for item5 in sub_deps {
            let mut ids_guard = ids.lock().unwrap();
            if ids_guard.contains(&item5.mod_id) {
                continue;
            }
            ids_guard.insert(item5.mod_id.clone());
            drop(ids_guard);
            list.lock().unwrap().push(item5);
        }
    });

    list.into_inner().unwrap()
}

/// 升级整合包
pub async fn upgrade_modpack(
    game: &GameInstance,
    data: &mut ModrinthVersionObj,
    gui: Option<Arc<dyn IAddGui>>,
) -> CoreResult<()> {
    let obj = make_download_obj(data, mcml_downloader::get_download_path());
    let file = obj.file.clone();

    if let Some(ref gui) = gui {
        gui.set_state(AddModPackState::DownloadPack);
        gui.set_now(1, Some(6));
    }

    let res = mcml_downloader::start_download_task(vec![obj]).await;
    if !res {
        return Err(ErrorType::DownloadFileFail);
    }

    if let Some(ref gui) = gui {
        gui.set_state(AddModPackState::ReadInfo);
        gui.set_now(2, Some(6));
    }

    let zip = BaseArchive::open(file)?;
    let mut worker = ModrinthPackWorker::new(BaseModPackWorker::new(
        zip,
        None,
        gui.as_ref().cloned(),
        None,
    ));

    worker.read_info()?;
    worker.read_version().await?;

    worker.update_game(game);

    if let Some(ref gui) = gui {
        gui.set_state(AddModPackState::Extract);
        gui.set_now(3, Some(6));
    }

    worker.extract(None).await?;

    if let Some(ref gui) = gui {
        gui.set_sub_text(None);
        gui.set_sub_now(0, None);
        gui.set_state(AddModPackState::GetInfo);
        gui.set_now(4, Some(6));
    }

    worker.check_upgrade().await?;

    if let Some(ref gui) = gui {
        gui.set_sub_text(None);
        gui.set_sub_now(0, None);
        gui.set_state(AddModPackState::DownloadFile);
        gui.set_now(5, Some(6));
    }

    worker.download().await;

    if let Some(ref gui) = gui {
        gui.set_state(AddModPackState::Done);
        gui.set_now(6, Some(6));
    }

    Ok(())
}

impl InstanceSettingObj {
    /// 自动标记模组
    ///
    /// - `over`: 是否覆盖已经标记的模组
    pub async fn auto_mark(&self, over: bool) -> CoreResult<()> {
        let list = self.read_mod_fast().await;
        let mut online = self.read_online_info();

        let hashs: Vec<String> = online
            .values()
            .map(|data| data.sha1.clone().to_ascii_lowercase())
            .collect();

        let mut check = Vec::new();
        for item in list.iter() {
            let sha1 = item.hash.get_sha1().unwrap();
            if hashs.contains(&sha1) && !over {
                continue;
            }

            check.push(sha1);
        }

        if !check.is_empty() {
            let list = modrinth_api::get_version_from_sha1(check).await?;

            for (_, value) in list {
                online.remove(&value.project_id);
                let file = &value.files[0];
                online.insert(
                    value.project_id.clone(),
                    OnlineInfoObj {
                        path: names::GAME_MODS_DIR.to_string(),
                        name: value.name,
                        file: file.filename.clone(),
                        sha1: file.hashes.sha1.clone(),
                        url: file.url.clone(),
                        modid: value.project_id,
                        fileid: value.id,
                    },
                );
            }

            self.save_online_info(&online);
        }

        Ok(())
    }
}
