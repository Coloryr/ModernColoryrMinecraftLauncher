use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    GameInstance,
    curseforge::pack_obj::CurseForgePackObj,
    data_res::{DownloadItemRes, ItemPathRes},
    gui_hook::{AddModPackState, IAddGui, IAddModPackGui, IProgressGui},
    launcher::{
        FileType, file_online_info_obj::OnlineInfoObj, instance_setting_obj::InstanceSettingObj,
    },
    loader::LoaderType,
    modpack::{BaseModPackWorker, ModPackWorker, curseforge_worker::CurseForgeWorker},
};
use mcml_base::{
    archives::BaseArchive,
    file_item::{FileHash, FileItemObj, LaterRun},
};
use mcml_names::{i18_items::error_type::CoreResult, names};
use mcml_net::curseforge_api::{
    self, CurseFogreArg,
    categories_obj::CurseForgeCategoriesObj,
    file_obj::{CurseForgeFileDataObj, CurseForgeFileObj, DependenciesObj},
};

pub mod pack_obj;

static CATEGORIES: OnceLock<CurseForgeCategoriesObj> = OnceLock::new();
static VERSIONS: OnceLock<Vec<String>> = OnceLock::new();

/// 排序编号
pub enum CurseForgeSortField {
    Featured,
    Popularity,
    LastUpdated,
    Name,
    Author,
    TotalDownloads,
    Category,
    GameVersion,
}

impl CurseForgeSortField {
    pub fn get_id(&self) -> u32 {
        match self {
            CurseForgeSortField::Featured => 1,
            CurseForgeSortField::Popularity => 2,
            CurseForgeSortField::LastUpdated => 3,
            CurseForgeSortField::Name => 4,
            CurseForgeSortField::Author => 5,
            CurseForgeSortField::TotalDownloads => 6,
            CurseForgeSortField::Category => 7,
            CurseForgeSortField::GameVersion => 8,
        }
    }
}

impl LoaderType {
    fn get_id(&self) -> u32 {
        match self {
            LoaderType::Forge => 1,
            LoaderType::Fabric => 4,
            LoaderType::Quilt => 5,
            LoaderType::NeoForge => 6,
            _ => 0,
        }
    }
}

/// 创建下载项目
pub fn make_file_item_obj<P: AsRef<Path>>(obj: &mut CurseForgeFileDataObj, path: P) -> FileItemObj {
    obj.fix_download_url();

    FileItemObj {
        url: obj.download_url.clone().unwrap(),
        name: obj.display_name.clone(),
        file: path.as_ref().join(&obj.file_name),
        hash: FileHash::Sha1(obj.sha1_hash()),
        later: LaterRun::None,
    }
}

/// 创建在线文件信息
pub fn make_file_online_info_obj(obj: &mut CurseForgeFileDataObj, path: &str) -> OnlineInfoObj {
    obj.fix_download_url();

    OnlineInfoObj {
        path: path.to_string(),
        name: obj.display_name.clone(),
        file: obj.file_name.clone(),
        sha1: obj.sha1_hash(),
        url: obj.download_url.clone().unwrap_or_default(),
        modid: obj.mod_id.to_string(),
        fileid: obj.id.to_string(),
    }
}

/// 模组依赖列表
pub struct CurseForgeModDependenciesRes {
    /// 名字
    pub name: String,
    /// 项目编号
    pub mod_id: u64,
    /// 是否可选
    pub opt: bool,
    /// 文件列表
    pub list: Vec<CurseForgeFileDataObj>,
}

/// 获取模组依赖
///
/// - `obj`: CurseForge文件信息
/// - `version`: 游戏版本
/// - `loader`: 加载器类型
pub async fn get_mod_dependencies(
    obj: &CurseForgeFileDataObj,
    version: &str,
    loader: LoaderType,
) -> Vec<CurseForgeModDependenciesRes> {
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
    dependencies: &Option<Vec<DependenciesObj>>,
    version: &str,
    loader: &LoaderType,
    ids: &Mutex<HashSet<u64>>,
    handle: &tokio::runtime::Handle,
) -> Vec<CurseForgeModDependenciesRes> {
    let dep = match dependencies {
        Some(dep) if !dep.is_empty() => dep,
        _ => return Vec::new(),
    };

    let list: Mutex<Vec<CurseForgeModDependenciesRes>> = Mutex::new(Vec::new());

    dep.par_iter().for_each(|item| {
        // Atomic check-and-insert: HashSet::insert returns false if already present
        {
            let mut ids_guard = ids.lock().unwrap();
            if !ids_guard.insert(item.mod_id) {
                return;
            }
        }

        let id = item.mod_id.to_string();
        let opt = item.relation_type != 2;

        let (res1, res2) = handle.block_on(async {
            let res1 = curseforge_api::get_files_page(CurseFogreArg {
                id: Some(id.clone()),
                version: Some(version.to_string()),
                loader: Some(loader.get_id()),
                ..Default::default()
            })
            .await;

            if res1.is_err() {
                return (None, None);
            }

            let data = res1.unwrap();
            if data.data.is_empty() {
                return (None, None);
            }

            let res2 = curseforge_api::get_mod_info(&id).await;
            (Some(data), res2.ok())
        });

        let Some(data) = res1 else { return };
        let Some(data1) = res2 else { return };

        // 在移动 data.data 之前，先递归获取第一个文件的依赖
        let sub_deps =
            get_mod_dependencies_inner(&data.data[0].dependencies, version, loader, ids, handle);

        list.lock().unwrap().push(CurseForgeModDependenciesRes {
            name: data1.data.name.clone(),
            mod_id: data1.data.id,
            opt: !opt,
            list: data.data,
        });

        // 添加未被标记的子依赖（避免同时持有 ids 和 list 锁）
        for item5 in sub_deps {
            let mut ids_guard = ids.lock().unwrap();
            if ids_guard.contains(&item5.mod_id) {
                continue;
            }
            ids_guard.insert(item5.mod_id);
            drop(ids_guard);
            list.lock().unwrap().push(item5);
        }
    });

    list.into_inner().unwrap()
}

impl FileType {
    fn get_classid(&self) -> u32 {
        match self {
            FileType::Mod => curseforge_api::CLASS_MOD,
            FileType::Save => curseforge_api::CLASS_SAVES,
            FileType::Shaderpack => curseforge_api::CLASS_SHADERPACKS,
            FileType::Resourcepack => curseforge_api::CLASS_RESOURCEPACKS,
            _ => 0,
        }
    }

    /// 获取分组数据
    pub async fn get_categories(&self) -> CoreResult<HashMap<String, String>> {
        let temp = match CATEGORIES.get() {
            Some(data) => data,
            None => {
                let list = curseforge_api::get_categories().await?;
                CATEGORIES.get_or_init(|| list)
            }
        };

        let mut list: Vec<_> = temp
            .data
            .iter()
            .filter(|item| item.class_id == self.get_classid())
            .collect();

        list.sort_by(|a, b| a.name.cmp(&b.name));

        Ok(list
            .into_iter()
            .map(|item| (item.id.to_string(), item.name.clone()))
            .collect())
    }

    /// 获取支持的游戏版本
    pub async fn get_game_version() -> CoreResult<Vec<String>> {
        match VERSIONS.get() {
            Some(version) => Ok(version.clone()),
            None => {
                let mut list = curseforge_api::get_version_type().await?;

                list.data.retain(|item| item.name.starts_with("Minecraft "));

                // Sort: ID > 17 desc first, ID < 18 asc after.
                // Use cached key to parse each id only once.
                list.data.sort_by_cached_key(|item| {
                    let id: i32 = item.id.parse().unwrap_or(0);
                    // Primary: new versions (id > 17) before old
                    // Secondary: new versions descending, old versions ascending
                    (id <= 17, if id > 17 { -id } else { id })
                });

                let version_list = curseforge_api::get_version().await?;

                // Build lookup map for O(1) version-type matching
                let version_map: HashMap<u32, &_> = version_list
                    .data
                    .iter()
                    .map(|v| (v.verion_type, v))
                    .collect();

                let mut result = vec![String::new()];
                for vtype in &list.data {
                    let vtype_id: u32 = vtype.id.parse().unwrap_or(0);
                    if let Some(version_data) = version_map.get(&vtype_id) {
                        result.extend(version_data.versions.clone());
                    }
                }

                VERSIONS.get_or_init(|| result.clone());
                Ok(result)
            }
        }
    }
}

/// 构建下载结果（串行处理）。
/// 只有 `get_modpack_info` 中的 `spawn_fetch_task` 才需要并行处理。
async fn build_results(
    game: &InstanceSettingObj,
    items: Vec<CurseForgeFileDataObj>,
    gui: &Option<Arc<dyn IAddGui>>,
    size: usize,
) -> DownloadItemRes {
    let mut list = Vec::with_capacity(size);
    let mut online = HashMap::with_capacity(size);
    let mut now = 0usize;

    for mut item in items {
        let Some(path) = game.get_item_path(&item.file_name, item.mod_id).await.ok() else {
            continue;
        };

        // Fix download URL and compute SHA1 once (avoid duplicate
        // work from calling both make_file_item_obj and
        // make_file_online_info_obj)
        item.fix_download_url();
        let url = item.download_url.clone().unwrap_or_default();
        let sha1 = item.sha1_hash();

        let modid_str = item.mod_id.to_string();

        let mut file_item = FileItemObj {
            url: url.clone(),
            name: item.display_name.clone(),
            file: path.file_path.join(&item.file_name),
            hash: FileHash::Sha1(sha1.clone()),
            later: LaterRun::None,
        };

        let online_item = if matches!(path.file_type, FileType::Save) {
            file_item.later = LaterRun::UnpackSave(game.get_saves_path());
            None
        } else {
            Some(OnlineInfoObj {
                path: path.path.clone(),
                name: item.display_name.clone(),
                file: item.file_name.clone(),
                sha1,
                url,
                modid: modid_str.clone(),
                fileid: item.id.to_string(),
            })
        };

        let now_val = now;
        now += 1;
        if let Some(gui) = gui {
            gui.set_sub_now(now_val, Some(size));
        }

        // Assemble: last-write-wins deduplication
        list.push(file_item);
        online.remove(&modid_str);
        if let Some(oi) = online_item {
            online.insert(modid_str, oi);
        }
    }

    DownloadItemRes { list, online }
}

/// 获取整合包模组信息
pub async fn get_modpack_info<P: AsRef<Path>>(
    path: P,
    obj: &CurseForgePackObj,
    gui: &Option<Arc<dyn IAddGui>>,
) -> CoreResult<DownloadItemRes> {
    let size = obj.files.len();

    let file_ids: Vec<_> = obj.files.iter().map(|f| f.file_id).collect();

    // ── Batch path: get_files succeeds → process all at once ──
    if let Ok(items) = curseforge_api::get_files(file_ids).await {
        return Ok(build_results(path, items, gui, size).await);
    }

    // ── Fallback path: fetch each file individually, any failure → empty ──
    const CONCURRENCY: usize = 20;
    let failed = Arc::new(AtomicBool::new(false));
    let mut fetched: Vec<CurseForgeFileDataObj> = Vec::with_capacity(size);

    {
        let mut tasks = tokio::task::JoinSet::new();
        let mut iter = obj.files.iter();

        // Fill initial batch
        for file_ref in (&mut iter).take(CONCURRENCY) {
            spawn_fetch_task(&mut tasks, file_ref, &failed);
        }

        // Drain + refill
        while let Some(result) = tasks.join_next().await {
            if let Ok(Some(data)) = result {
                fetched.push(data.data);
            }
            // Once any item fails, stop spawning but keep draining remaining tasks
            if failed.load(Ordering::Relaxed) {
                continue;
            }
            if let Some(file_ref) = iter.next() {
                spawn_fetch_task(&mut tasks, file_ref, &failed);
            }
        }
    }

    if failed.load(Ordering::Relaxed) {
        return Ok(DownloadItemRes {
            list: Vec::new(),
            online: HashMap::new(),
        });
    }

    Ok(build_results(path, fetched, gui, size).await)
}

fn spawn_fetch_task(
    tasks: &mut tokio::task::JoinSet<Option<CurseForgeFileObj>>,
    file_ref: &pack_obj::FilesObj,
    failed: &Arc<AtomicBool>,
) {
    let pid = file_ref.project_id.to_string();
    let fid = file_ref.file_id.to_string();
    let failed = failed.clone();
    tasks.spawn(async move {
        match curseforge_api::get_mod(&pid, &fid).await {
            Ok(data) => Some(data),
            Err(_) => {
                failed.store(true, Ordering::Relaxed);
                None
            }
        }
    });
}

impl InstanceSettingObj {
    /// Resolve the item path and file type. Takes individual fields to enable
    /// cheap concurrent dispatch without cloning the full `CurseForgeFileDataObj`.
    async fn get_item_path(&self, file_name: &str, mod_id: u64) -> CoreResult<ItemPathRes> {
        let mut item1 = ItemPathRes {
            file_path: self.get_mods_path(),
            path: names::GAME_MODS_DIR.to_string(),
            file_type: FileType::Mod,
        };

        if !file_name.ends_with(names::JAR_DOT_EXT) {
            let info1 = curseforge_api::get_mod_info(&mod_id.to_string()).await?;

            // Categories list: first match wins
            for item2 in &info1.data.categories {
                if apply_class_id(item2.class_id, &mut item1, self) {
                    break;
                }
            }

            // Fallback: data.class_id may override category result
            apply_class_id(info1.data.class_id, &mut item1, self);
        }

        Ok(item1)
    }
}

/// Apply a CurseForge class_id to the path resolver. Returns `true` when
/// the id matched one of the known file-type classes.
#[inline]
fn apply_class_id(class_id: u32, item: &mut ItemPathRes, instance: &InstanceSettingObj) -> bool {
    match class_id {
        curseforge_api::CLASS_RESOURCEPACKS => {
            item.change_to_resourcepacks(instance);
            true
        }
        curseforge_api::CLASS_SHADERPACKS => {
            item.change_to_shaderpacks(instance);
            true
        }
        curseforge_api::CLASS_SAVES => {
            item.change_to_saves(instance);
            true
        }
        curseforge_api::CLASS_OPENLOADER_DATAPACK => {
            item.change_to_openloader_datapack(instance);
            true
        }
        _ => false,
    }
}

/// 升级整合包
pub async fn upgrade_modpack(
    game: &GameInstance,
    data: &mut CurseForgeFileDataObj,
    gui: Option<Arc<dyn IAddGui>>,
) -> bool {
    data.fix_download_url();

    let obj = make_file_item_obj(data, mcml_downloader::get_download_path());
    let file = obj.file.clone();

    if let Some(ref gui) = gui {
        gui.set_state(AddModPackState::DownloadPack);
        gui.set_now(1, Some(6));
    }

    let res = mcml_downloader::start_download_task(vec![obj]).await;
    if !res {
        return false;
    }

    if let Some(ref gui) = gui {
        gui.set_state(AddModPackState::ReadInfo);
        gui.set_now(2, Some(6));
    }

    let zip = BaseArchive::open(file);
    if let Err(err) = zip {
        mcml_log::error_type(err);
        return false;
    }
    let mut worker = CurseForgeWorker::new(BaseModPackWorker::new(
        zip.unwrap(),
        None,
        gui.as_ref().cloned(),
        None,
    ));

    if !worker.read_info() || !worker.read_version().await {
        return false;
    }

    worker.update_game(game);

    if let Some(ref gui) = gui {
        gui.set_state(AddModPackState::Unzip);
        gui.set_now(3, Some(6));
    }

    if !worker.unzip(None).await {
        return false;
    }

    if let Some(ref gui) = gui {
        gui.set_sub_text(None);
        gui.set_sub_now(0, None);
        gui.set_state(AddModPackState::GetInfo);
        gui.set_now(4, Some(6));
    }

    if !worker.check_upgrade().await {
        return false;
    }

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

    true
}
