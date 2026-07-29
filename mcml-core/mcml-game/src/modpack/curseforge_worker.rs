use async_trait::async_trait;
use mcml_base::{
    archives::ArchiveEntryInfo,
    file_item::{FileHash, FileItemObj, LaterRun},
    path_helper, serialize_tools,
};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorType},
    names,
};
use mcml_net::curseforge_api;

use crate::{
    GameInstance,
    curseforge::{
        file_obj::{CurseForgeFileDataObj, HashesObj},
        pack_obj::{CurseForgePackObj, FilesObj},
    },
    launcher::{SourceType, instance_setting_obj::InstanceSettingObj},
    launcher_path::version_path,
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

// ── Helpers ────────────────────────────────────────────────────────────

/// 从 CurseForge 哈希列表中提取 SHA1 值（algo == 1）。
fn extract_sha1(hashes: &[HashesObj]) -> String {
    hashes
        .iter()
        .find(|h| h.algo == 1)
        .map(|h| h.value.clone())
        .unwrap_or_default()
}

/// 将 `CurseForgeFileDataObj` 转为下载项。
fn to_download_item(data: &mut CurseForgeFileDataObj, mods_path: &std::path::Path) -> FileItemObj {
    data.fix_download_url();
    FileItemObj {
        url: data.download_url.clone().unwrap_or_default(),
        name: data.display_name.clone(),
        file: mods_path.join(&data.file_name),
        hash: FileHash::Sha1(extract_sha1(&data.hashes)),
        later: LaterRun::None,
    }
}

/// 批量解析文件 ID 为 `CurseForgeFileDataObj`。
/// 优先批量接口，失败则逐文件查询。
async fn resolve_files(
    file_ids: &[u64],
    cancel: &tokio_util::sync::CancellationToken,
) -> Vec<CurseForgeFileDataObj> {
    // 批量
    if let Ok(items) =
        curseforge_api::get_files::<Vec<CurseForgeFileDataObj>>(file_ids.to_vec()).await
    {
        return items;
    }

    // 逐文件降级
    let mut items = Vec::new();
    for &fid in file_ids {
        if cancel.is_cancelled() {
            break;
        }
        let fid_str = fid.to_string();
        if let Ok(data) = curseforge_api::get_mod::<CurseForgeFileDataObj>(&fid_str, &fid_str).await
        {
            items.push(data);
        }
    }
    items
}

#[async_trait]
impl ModPackWorker for CurseForgeWorker {
    /// 获取主信息
    fn read_info(&mut self) -> bool {
        if let Some(item) = self
            .base
            .zip
            .entries()
            .iter()
            .filter(|item| item.name.eq_ignore_ascii_case(names::MANIFEST_FILE))
            .next()
            && let Ok(data) = self
                .base
                .zip
                .read(&item.name)
                .and_then(|data| serialize_tools::json_from_bytes::<CurseForgePackObj>(&data))
        {
            self.info = Some(data);
            true
        } else {
            false
        }
    }

    /// 获取版本数据
    async fn read_version(&mut self) -> bool {
        if self.info.is_none() {
            return false;
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

        version_path::check_update(minecraft).await.is_ok()
    }

    /// 创建游戏实例
    async fn create_instance(&self, group: Option<String>) -> CoreResult<GameInstance> {
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
                game.create_instance(&self.base.gui).await
            }
            None => Err(ErrorType::InfoNotFound("info".to_string())),
        }
    }

    /// 解压整合包覆盖文件到游戏目录。
    ///
    /// `overrides/` 下的文件去除前缀后写入游戏根目录；其余文件直接
    /// 写入游戏路径。
    async fn unzip(&self, unselect: Option<&Vec<&ArchiveEntryInfo>>) -> bool {
        let Some(game) = &self.base.game else {
            return false;
        };
        let Some(info) = &self.info else {
            return false;
        };

        let game = game.read().unwrap();
        let base_path = game.get_base_path();
        let game_path = game.get_game_path();
        let prefix = format!("{}/", info.overrides);

        let entries: Vec<_> = self.base.zip.entries().iter().collect();
        let total = entries.len();
        let mut index = 0usize;

        if let Some(pgui) = &self.base.pack_gui {
            pgui.set_sub_now(0, Some(total));
        }

        for entry in entries {
            if self.base.cancel.is_cancelled() {
                return false;
            }
            if entry.is_dir {
                index += 1;
                if let Some(pgui) = &self.base.pack_gui {
                    pgui.set_sub_now(index, Some(total));
                }
                continue;
            }
            // 跳过不需要解压的条目
            if let Some(unsel) = unselect {
                if unsel.iter().any(|u| u.name == entry.name) {
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

            if self
                .base
                .zip
                .extract_file(&entry.name, &output, None)
                .is_err()
            {
                return false;
            }
            if let Some(pgui) = &self.base.pack_gui {
                pgui.set_sub_now(index, Some(total));
            }
        }

        true
    }

    /// 获取模组下载信息。
    ///
    /// 批量解析 manifest 中的文件列表，构建下载项并存入
    /// `base.downloads`，后续由 [`download`] 统一下载。
    async fn get_info(&self) -> bool {
        let Some(info) = &self.info else {
            return false;
        };
        let Some(game) = &self.base.game else {
            return false;
        };

        let file_ids: Vec<u64> = info.files.iter().map(|f| f.file_id).collect();
        if file_ids.is_empty() {
            return true;
        }

        // 批量解析文件信息
        let mut files = resolve_files(&file_ids, &self.base.cancel).await;
        if self.base.cancel.is_cancelled() {
            return false;
        }

        let mods_path = {
            let game = game.read().unwrap();
            game.get_mods_path()
        };

        // 构建下载列表
        let downloads: Vec<FileItemObj> = files
            .iter_mut()
            .map(|data| to_download_item(data, &mods_path))
            .collect();

        let Ok(mut guard) = self.base.downloads.lock() else {
            return false;
        };
        *guard = downloads;

        !guard.is_empty() || info.files.is_empty()
    }

    /// 统一下载所有模组文件。
    async fn download(&self) {
        // 取出下载列表（在 .await 前释放 MutexGuard）
        let items = {
            let Ok(guard) = self.base.downloads.lock() else {
                return;
            };
            guard.clone()
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
    async fn check_upgrade(&self) -> bool {
        let Some(info) = &self.info else {
            return false;
        };
        let Some(game) = &self.base.game else {
            return false;
        };

        // 在 .await 前释放 RwLockReadGuard
        let (base_path, old_info) = {
            let game = game.read().unwrap();
            let base_path = game.get_base_path();
            let old_manifest_path = base_path.join(names::MANIFEST_FILE);
            let old_info: Option<CurseForgePackObj> = path_helper::open_read(&old_manifest_path)
                .ok()
                .and_then(|stream| serialize_tools::json_from_stream(&stream).ok());
            (base_path, old_info)
        };

        if let Some(old_info) = old_info {
            // ── 有旧 manifest：用 fileID 比对 ──
            check_upgrade_with_old_manifest(info, &old_info, &base_path).await
        } else {
            // ── 无旧 manifest：通过 SHA1 比对 ──
            check_upgrade_sha1(info, &base_path).await
        }
    }
}

// ── check_upgrade 内部逻辑 ──────────────────────────────────────────────

/// 有旧 manifest 时：对比两边的 `Files` 列表，找出新增、删除、变更。
async fn check_upgrade_with_old_manifest(
    new_info: &CurseForgePackObj,
    old_info: &CurseForgePackObj,
    _base_path: &std::path::Path,
) -> bool {
    // TODO: 需要访问 Game.Mods 来清理已删除的模组
    // 当前仅构建下载列表

    let mut add_list: Vec<&FilesObj> = Vec::new();
    let mut remove_list: Vec<&FilesObj> = Vec::new();

    // 交集：fileID 不同 → 变更（add 新 + remove 旧）
    // 只在 new 中 → 新增
    // 只在 old 中 → 删除

    let mut old_matched: Vec<bool> = vec![false; old_info.files.len()];

    for new_file in &new_info.files {
        let mut found = false;
        for (j, old_file) in old_info.files.iter().enumerate() {
            if new_file.project_id == old_file.project_id {
                found = true;
                old_matched[j] = true;
                if new_file.file_id != old_file.file_id {
                    add_list.push(new_file);
                    remove_list.push(old_file);
                }
                break;
            }
        }
        if !found {
            add_list.push(new_file);
        }
    }

    for (j, old_file) in old_info.files.iter().enumerate() {
        if !old_matched[j] {
            remove_list.push(old_file);
        }
    }

    // 构建下载列表（仅新增/变更）
    if !add_list.is_empty() {
        let file_ids: Vec<u64> = add_list.iter().map(|f| f.file_id).collect();
        // TODO: 获取 game mods_path 来设置正确的下载路径
        // 当前简化处理
        let cancel = tokio_util::sync::CancellationToken::new();
        let _files = resolve_files(&file_ids, &cancel).await;
    }

    let _ = remove_list;
    !add_list.is_empty()
}

/// 无旧 manifest 时：通过 API 获取最新文件信息，对比 SHA1。
async fn check_upgrade_sha1(new_info: &CurseForgePackObj, _base_path: &std::path::Path) -> bool {
    // TODO: 需要 Game.Mods 字典来比对 SHA1
    // 当前为占位实现

    let file_ids: Vec<u64> = new_info.files.iter().map(|f| f.file_id).collect();
    if file_ids.is_empty() {
        return true;
    }

    let cancel = tokio_util::sync::CancellationToken::new();
    let _files = resolve_files(&file_ids, &cancel).await;

    !_files.is_empty()
}
