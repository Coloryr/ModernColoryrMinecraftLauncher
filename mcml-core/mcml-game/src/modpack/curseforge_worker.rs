use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use mcml_base::{
    file_item::{FileHash, FileItemObj, LaterRun},
    path_helper, serialize_tools,
};
use mcml_names::{
    i18_items::error_type::{CoreResult, DataNotFoundData, ErrorType},
    names,
};
use mcml_net::curseforge_api::{self, file_obj::CurseForgeFileDataObj};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    GameInstance,
    curseforge::{
        self,
        pack_obj::{CurseForgePackObj, FilesObj},
    },
    gui_hook::IAddGui,
    launcher::{
        SourceType, file_online_info_obj::OnlineInfoObj, instance_setting_obj::InstanceSettingObj,
    },
    launcher_path::{instance_path::OnlineInfoList, version_path},
    loader::LoaderType,
    modpack::{BaseModPackWorker, ModPackWorker},
};

/// CurseForge整合包安装器
pub struct CurseForgeWorker {
    /// 整合包信息
    info: Option<CurseForgePackObj>,
    /// 基础安装器
    base: BaseModPackWorker,
}

impl CurseForgeWorker {
    /// 创建 CurseForge 整合包安装器
    pub fn new(base: BaseModPackWorker) -> Self {
        Self { info: None, base }
    }
}

/// 批量解析文件 ID 为 `CurseForgeFileDataObj`。
/// 优先批量接口，失败则逐文件查询。
async fn resolve_files(
    file_ids: &[u64],
    cancel: &Option<CancellationToken>,
) -> Vec<CurseForgeFileDataObj> {
    // 批量
    if let Ok(items) = curseforge_api::get_files(file_ids.to_vec()).await {
        return items;
    }

    // 逐文件降级
    let mut items = Vec::new();
    for &fid in file_ids {
        if let Some(cancel) = cancel
            && cancel.is_cancelled()
        {
            break;
        }
        let fid_str = fid.to_string();
        if let Ok(data) = curseforge_api::get_mod(&fid_str, &fid_str).await {
            items.push(data.data);
        }
    }
    items
}

#[async_trait]
impl ModPackWorker for CurseForgeWorker {
    /// 获取主信息
    fn read_info(&mut self) -> CoreResult<()> {
        let data = self
            .base
            .archive
            .entries()
            .iter()
            .filter(|item| item.name.eq_ignore_ascii_case(names::MANIFEST_FILE))
            .next();

        if let Some(item) = data {
            let data1 =
                self.base.archive.read(&item.name).and_then(|data| {
                    serialize_tools::json_from_bytes::<CurseForgePackObj>(&data)
                })?;

            self.info = Some(data1);
            Ok(())
        } else {
            Err(ErrorType::DataNotFound(DataNotFoundData::Info))
        }
    }

    /// 获取版本数据
    async fn read_version(&mut self) -> CoreResult<()> {
        if self.info.is_none() {
            return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
        }

        let info = self.info.as_ref().unwrap();

        for item in info.minecraft.mod_loaders.iter() {
            if item.id.starts_with(names::FORGE_KEY) {
                self.base.loader = LoaderType::Forge;
                self.base.loader_version = item.id.replace(&format!("{}-", names::FORGE_KEY), "");
            } else if item.id.starts_with(names::FABRIC_KEY) {
                self.base.loader = LoaderType::Fabric;
                self.base.loader_version = item.id.replace(&format!("{}-", names::FABRIC_KEY), "");
            } else if item.id.starts_with(names::NEOFORGE_KEY) {
                self.base.loader = LoaderType::NeoForge;
                self.base.loader_version =
                    item.id.replace(&format!("{}-", names::NEOFORGE_KEY), "");
            } else if item.id.starts_with(names::QUILT_KEY) {
                self.base.loader = LoaderType::Quilt;
                self.base.loader_version = item.id.replace(&format!("{}-", names::QUILT_KEY), "");
            }
        }

        let minecraft = &self.info.as_ref().unwrap().minecraft.version;
        let version = &self.base.loader_version;

        if version.starts_with(&format!("{}-", minecraft)) && version.len() > minecraft.len() + 1 {
            self.base.loader_version = version[(minecraft.len() + 1)..].to_string();
        }

        self.base.game_version = minecraft.clone();

        version_path::check_update(minecraft).await?;

        Ok(())
    }

    /// 创建游戏实例
    async fn create_instance(&self, group: Option<String>) -> CoreResult<Uuid> {
        match &self.info {
            Some(info) => {
                let name = format!("{}-{}", info.name, info.version);
                let game = InstanceSettingObj {
                    group,
                    name,
                    version: self.base.game_version.clone(),
                    is_modpack: true,
                    loader: self.base.loader,
                    source_type: SourceType::CurseForge,
                    loader_version: Some(self.base.loader_version.clone()),
                    ..Default::default()
                };
                Ok(game.create_instance(&self.base.gui).await?.read().unwrap().uuid)
            }
            None => Err(ErrorType::DataNotFound(DataNotFoundData::Info)),
        }
    }

    /// 解压整合包覆盖文件到游戏目录。
    ///
    /// `overrides/` 下的文件去除前缀后写入游戏根目录；其余文件直接
    /// 写入游戏路径。
    async fn extract(&self, unselect: Option<Vec<String>>) -> CoreResult<()> {
        let Some(game) = &self.base.game else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::GameInstance));
        };
        let Some(info) = &self.info else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
        };

        let game = game.read().unwrap();
        let base_path = game.get_base_path();
        let game_path = game.get_game_path();
        let prefix = format!("{}/", info.overrides);

        let entries: Vec<_> = self.base.archive.entries().iter().collect();
        let total = entries.len();
        let mut index = 0usize;

        if let Some(pgui) = &self.base.pack_gui {
            pgui.set_sub_now(0, Some(total));
        }

        for entry in entries {
            if let Some(cancel) = &self.base.cancel
                && cancel.is_cancelled()
            {
                return Err(ErrorType::TaskCancel);
            }
            if entry.is_dir {
                index += 1;
                if let Some(pgui) = &self.base.pack_gui {
                    pgui.set_sub_now(index, Some(total));
                }
                continue;
            }
            // 跳过不需要解压的条目
            if let Some(ref unsel) = unselect {
                if unsel.iter().any(|u| u == &entry.name) {
                    index += 1;
                    if let Some(pgui) = &self.base.pack_gui {
                        pgui.set_sub_now(index, Some(total));
                    }
                    continue;
                }
            }

            if let Some(pgui) = &self.base.pack_gui {
                pgui.set_sub_text(Some(entry.name.clone()));
            }
            index += 1;

            let output = if let Some(rel) = entry.name.strip_prefix(&prefix) {
                // 覆盖文件：去除 overrides 前缀后放到游戏根目录
                game_path.join(rel)
            } else {
                base_path.join(&entry.name)
            };

            self.base.archive.extract_file(&entry.name, &output, None)?;
            if let Some(pgui) = &self.base.pack_gui {
                pgui.set_sub_now(index, Some(total));
            }
        }

        Ok(())
    }

    /// 获取模组下载信息。
    ///
    /// 批量解析 manifest 中的文件列表，构建下载项并存入
    /// `base.downloads`，后续由 [`download`] 统一下载。
    async fn get_info(&self) -> CoreResult<bool> {
        let Some(game) = &self.base.game else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::GameInstance));
        };
        let Some(info) = &self.info else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
        };

        let list = curseforge::get_modpack_info(game, info, &self.base.pack_gui).await?;

        let downloads = list.list;
        let mods = list.online;

        game.read().unwrap().save_online_info(&mods);

        let mut guard = self.base.downloads.lock().unwrap();
        *guard = downloads;

        Ok(!guard.is_empty() || info.files.is_empty())
    }

    /// 统一下载所有模组文件。
    async fn download(&self) {
        // 取出下载列表（在 .await 前释放 MutexGuard），取走后列表为空，
        // 重复调用不会重复下载
        let items = {
            let Ok(mut guard) = self.base.downloads.lock() else {
                return;
            };
            std::mem::take(&mut *guard)
        };
        if items.is_empty() {
            return;
        }
        mcml_downloader::start_download_task(items).await;
    }

    /// 更新游戏实例版本信息
    fn update_game(&mut self, game: &GameInstance) {
        self.base.game = Some(game.clone());

        let mut game = game.write().unwrap();
        game.loader = self.base.loader;
        game.loader_version = Some(self.base.loader_version.clone());
        game.version = self.base.game_version.clone();

        game.save();
    }

    /// 检查整合包更新。
    ///
    /// 对比当前整合包 manifest 与上一次安装的 manifest：
    /// - 存在旧 manifest：用 fileID 快速比对，找出新增/变更/删除的模组
    /// - 无旧 manifest：通过 API 获取文件信息后比对 SHA1
    ///
    /// 最终将需要下载的文件存入 `base.downloads`。
    async fn check_upgrade(&self) -> CoreResult<()> {
        let Some(game) = &self.base.game else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::GameInstance));
        };
        let Some(info) = &self.info else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
        };

        let old_info = {
            let game = game.read().unwrap();
            let base_path = game.get_base_path();
            let old_manifest_path = base_path.join(names::MANIFEST_FILE);
            path_helper::open_read(&old_manifest_path)
                .ok()
                .and_then(|stream| serialize_tools::json_from_stream(&stream).ok())
        };

        if let Some(old_info) = old_info {
            // ── 有旧 manifest：用 fileID 比对 ──
            check_upgrade_with_old_manifest(
                info,
                &old_info,
                game,
                &self.base.downloads,
                &self.base.pack_gui,
                &self.base.cancel,
            )
            .await?;
        } else {
            // ── 无旧 manifest：通过 SHA1 比对 ──
            check_upgrade_sha1(
                info,
                game,
                &self.base.downloads,
                &self.base.pack_gui,
                &self.base.cancel,
            )
            .await?;
        }

        // 写入当前整合包 manifest，作为下一次升级比对用的旧清单
        serialize_tools::json_to_file(
            info,
            game.read()
                .unwrap()
                .get_base_path()
                .join(names::MANIFEST_FILE),
        )?;

        Ok(())
    }
}

/// 有旧 manifest 时：对比两边的 `Files` 列表，找出新增、删除、变更。
///
/// 1. 以 `project_id` 为键比对新旧文件列表
/// 2. 同 project 但不同 file_id → 变更（下载新版本，删除旧版本）
/// 3. 仅在新 manifest 中 → 新增
/// 4. 仅在旧 manifest 中 → 删除
/// 5. 对每个需要下载的文件调用 API 解析，构建下载项并更新在线信息
async fn check_upgrade_with_old_manifest(
    new_info: &CurseForgePackObj,
    old_info: &CurseForgePackObj,
    game: &GameInstance,
    downloads: &Mutex<Vec<FileItemObj>>,
    pack_gui: &Option<Arc<dyn IAddGui>>,
    cancel: &Option<CancellationToken>,
) -> CoreResult<()> {
    let mut add_list: Vec<&FilesObj> = Vec::new();
    let mut remove_list: Vec<&FilesObj> = Vec::new();

    let mut old_matched = vec![false; old_info.files.len()];

    // 第一遍：匹配新旧列表中 project_id 相同的文件
    for new_file in &new_info.files {
        let mut found = false;
        for (j, old_file) in old_info.files.iter().enumerate() {
            if new_file.project_id == old_file.project_id {
                found = true;
                old_matched[j] = true;
                if new_file.file_id != old_file.file_id {
                    // 同一项目但文件 ID 不同 → 需要更新
                    add_list.push(new_file);
                    remove_list.push(old_file);
                }
                break;
            }
        }
        if !found {
            // 仅在新的 manifest 中出现 → 新增
            add_list.push(new_file);
        }
    }

    // 仅在旧的 manifest 中出现 → 删除
    for (j, old_file) in old_info.files.iter().enumerate() {
        if !old_matched[j] {
            remove_list.push(old_file);
        }
    }

    // 检查取消
    if let Some(cancel) = cancel
        && cancel.is_cancelled()
    {
        return Err(ErrorType::TaskCancel);
    }

    // 删除已移除的模组
    {
        let game = game.read().unwrap();
        let game_path = game.get_game_path();
        let mut online_info = game.read_online_info();

        for item in &remove_list {
            let project_id_str = item.project_id.to_string();
            if let Some(mod_info) = online_info.remove(&project_id_str) {
                let local = game_path.join(&mod_info.path).join(&mod_info.file);
                delete_with_disabled(local);
            }
        }

        game.save_online_info(&online_info);
    }

    // 检查取消
    if let Some(cancel) = cancel
        && cancel.is_cancelled()
    {
        return Err(ErrorType::TaskCancel);
    }

    // 无需下载 → 完成
    if add_list.is_empty() {
        downloads.lock().unwrap().clear();
        return Ok(());
    }

    // 逐个解析新增/变更的文件并构建下载列表
    let (mods_path, mut online_info) = {
        let game = game.read().unwrap();
        (game.get_mods_path(), game.read_online_info())
    };

    let total = add_list.len();
    if let Some(pgui) = pack_gui {
        pgui.set_sub_now(0, Some(total));
    }

    let mut new_downloads: Vec<FileItemObj> = Vec::with_capacity(total);

    for (b, item) in add_list.iter().enumerate() {
        // 检查取消
        if let Some(cancel) = cancel
            && cancel.is_cancelled()
        {
            return Err(ErrorType::TaskCancel);
        }

        let pid = item.project_id.to_string();
        let fid = item.file_id.to_string();

        let res = curseforge_api::get_mod(&pid, &fid).await?;

        let mut data = res.data;
        let mod_id_str = data.mod_id.to_string();
        let sha1 = data.sha1_hash();

        data.fix_download_url();

        let download = FileItemObj {
            url: data.download_url.clone().unwrap_or_default(),
            name: data.display_name.clone(),
            file: mods_path.join(&data.file_name),
            hash: FileHash::Sha1(sha1.clone()),
            later: LaterRun::None,
        };
        new_downloads.push(download);

        // 更新在线信息：移除旧条目后插入新条目
        online_info.remove(&mod_id_str);
        online_info.insert(
            mod_id_str,
            OnlineInfoObj {
                path: names::GAME_MODS_DIR.to_string(),
                name: data.display_name.clone(),
                file: data.file_name.clone(),
                sha1,
                url: data.download_url.unwrap_or_default(),
                modid: data.mod_id.to_string(),
                fileid: data.id.to_string(),
            },
        );

        if let Some(pgui) = pack_gui {
            pgui.set_sub_now(b + 1, Some(total));
        }
    }

    // 保存更新后的在线信息
    game.read().unwrap().save_online_info(&online_info);
    *downloads.lock().unwrap() = new_downloads;

    Ok(())
}

/// 无旧 manifest 时：通过 API 获取最新文件信息，以 mod_id 和 SHA1 比对。
///
/// 1. 批量解析 manifest 中的全部文件
/// 2. 构建 `FileOnlineInfoObj` 映射（mod_id → 在线信息）
/// 3. 与现有 `online_info` 比对：同 mod_id 但 fileid/sha1 不同 → 变更
/// 4. 仅在在线信息中不存在 → 新增
/// 5. 删除旧文件，构建下载列表
async fn check_upgrade_sha1(
    new_info: &CurseForgePackObj,
    game: &GameInstance,
    downloads: &Mutex<Vec<FileItemObj>>,
    pack_gui: &Option<Arc<dyn IAddGui>>,
    cancel: &Option<CancellationToken>,
) -> CoreResult<()> {
    let file_ids: Vec<u64> = new_info.files.iter().map(|f| f.file_id).collect();
    if file_ids.is_empty() {
        downloads.lock().unwrap().clear();
        return Ok(());
    }

    // 批量解析文件信息，含进度回调
    let resolved_files = {
        if let Some(pgui) = pack_gui {
            pgui.set_sub_now(0, Some(new_info.files.len()));
        }

        let mut files = Vec::new();

        // 批量获取
        if let Ok(items) = curseforge_api::get_files(file_ids.clone()).await {
            files = items;
        } else {
            // 逐文件降级
            let total = file_ids.len();
            for (idx, &fid) in file_ids.iter().enumerate() {
                if let Some(cancel) = cancel
                    && cancel.is_cancelled()
                {
                    return Err(ErrorType::TaskCancel);
                }
                let fid_str = fid.to_string();
                if let Ok(data) = curseforge_api::get_mod(&fid_str, &fid_str).await {
                    files.push(data.data);
                }
                if let Some(pgui) = pack_gui {
                    pgui.set_sub_now(idx + 1, Some(total));
                }
            }
        }

        if files.is_empty() {
            return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
        }
        files
    };

    // 检查取消
    if let Some(cancel) = cancel
        && cancel.is_cancelled()
    {
        return Err(ErrorType::TaskCancel);
    }

    // 从解析结果构建 mod_id → (FileOnlineInfoObj, FileItemObj) 映射
    let (game_path, mods_path) = {
        let game = game.read().unwrap();
        (game.get_game_path(), game.get_mods_path())
    };

    // new_mods: mod_id → 在线文件信息对象
    // new_download_map: mod_id → 下载项对象
    let mut new_online_map: OnlineInfoList = HashMap::new();
    let mut new_download_map: HashMap<String, FileItemObj> = HashMap::new();

    for file_data in &resolved_files {
        let mod_id_str = file_data.mod_id.to_string();
        let sha1 = file_data.sha1_hash();

        let download = FileItemObj {
            url: file_data.download_url.clone().unwrap_or_default(),
            name: file_data.display_name.clone(),
            file: mods_path.join(&file_data.file_name),
            hash: FileHash::Sha1(sha1.clone()),
            later: LaterRun::None,
        };

        let online = OnlineInfoObj {
            path: names::GAME_MODS_DIR.to_string(),
            name: file_data.display_name.clone(),
            file: file_data.file_name.clone(),
            sha1,
            url: file_data.download_url.clone().unwrap_or_default(),
            modid: mod_id_str.clone(),
            fileid: file_data.id.to_string(),
        };

        new_online_map.insert(mod_id_str.clone(), online);
        new_download_map.insert(mod_id_str, download);
    }

    // ── 比对现有在线信息 ──
    let mut online_info = {
        let game = game.read().unwrap();
        game.read_online_info()
    };

    let mut add_list: Vec<OnlineInfoObj> = Vec::new();
    let mut remove_list: Vec<OnlineInfoObj> = Vec::new();

    // 遍历现有模组
    for (mod_id, existing_mod) in online_info.iter() {
        if let Some(new_mod) = new_online_map.get(mod_id) {
            // 同 mod_id：检查是否需要更新
            if existing_mod.fileid != new_mod.fileid || existing_mod.sha1 != new_mod.sha1 {
                add_list.push(new_mod.clone());
                remove_list.push(existing_mod.clone());
            }
        }
        // 不在新列表中 → 不删除（SHA1 路径下不作删除，只更新和新增）
    }

    // 新增：在新列表中但不在现有列表中的模组
    for (mod_id, new_mod) in &new_online_map {
        if !online_info.contains_key(mod_id) {
            add_list.push(new_mod.clone());
        }
    }

    // 删除旧文件
    for item in &remove_list {
        delete_with_disabled(game_path.join(&item.path).join(&item.file));
        online_info.remove(&item.modid);
    }

    // 构建下载列表
    let mut new_downloads: Vec<FileItemObj> = Vec::new();

    for item in &add_list {
        if let Some(download) = new_download_map.get(&item.modid) {
            new_downloads.push(download.clone());
            online_info.insert(item.modid.clone(), item.clone());
        }
    }

    // 保存更新后的在线信息
    game.read().unwrap().save_online_info(&online_info);
    *downloads.lock().unwrap() = new_downloads;

    Ok(())
}

/// 删除文件，若文件已被禁用（追加了 `.disable`/`.disabled` 后缀）则一并删除。
fn delete_with_disabled<P: AsRef<Path>>(file: P) {
    let file = file.as_ref();
    // `delete` 在文件不存在时是无操作
    let _ = path_helper::delete(file);
    let _ = path_helper::delete(format!("{}{}", file.display(), names::DISABLE_DOT_EXT));
    let _ = path_helper::delete(format!("{}{}", file.display(), names::DISABLED_DOT_EXT));
}
