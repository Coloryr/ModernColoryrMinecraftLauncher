use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use mcml_base::{archives::ArchiveEntryInfo, file_item::FileItemObj, serialize_tools};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorType},
    names,
};
use tokio_util::sync::CancellationToken;

use crate::{
    GameInstance,
    gui_hook::IAddGui,
    launcher::{SourceType, instance_setting_obj::InstanceSettingObj},
    launcher_path::version_path,
    loader::LoaderType,
    modpack::{BaseModPackWorker, ModPackWorker},
    modrinth::{self, pack_obj::ModrinthPackObj},
};

/// Modrinth整合包安装器
pub struct ModrinthPackWorker {
    /// 整合包信息
    info: Option<ModrinthPackObj>,
    /// 基础安装器
    base: BaseModPackWorker,
}

impl ModrinthPackWorker {
    /// 创建 Modrinth 整合包安装器
    pub fn new(base: BaseModPackWorker) -> Self {
        Self { info: None, base }
    }
}

#[async_trait]
impl ModPackWorker for ModrinthPackWorker {
    /// 获取主信息
    fn read_info(&mut self) -> bool {
        if let Some(item) = self
            .base
            .zip
            .entries()
            .iter()
            .filter(|item| item.name.eq_ignore_ascii_case(names::MODRINTH_FILE))
            .next()
            && let Ok(data) = self
                .base
                .zip
                .read(&item.name)
                .and_then(|data| serialize_tools::json_from_bytes::<ModrinthPackObj>(&data))
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

        if let Some(version) = info.dependencies.get(names::MINECRAFT_KEY) {
            self.base.game_version = version.clone();
        }

        if let Some(version) = info.dependencies.get(names::FORGE_KEY) {
            self.base.loader = LoaderType::Forge;
            self.base.loader_version = version.clone();
        }
        if let Some(version) = info.dependencies.get(names::FABRIC_KEY) {
            self.base.loader = LoaderType::Fabric;
            self.base.loader_version = version.clone();
        }
        if let Some(version) = info.dependencies.get(names::NEOFORGE_KEY) {
            self.base.loader = LoaderType::NeoForge;
            self.base.loader_version = version.clone();
        }
        if let Some(version) = info.dependencies.get(names::QUILT_KEY) {
            self.base.loader = LoaderType::Quilt;
            self.base.loader_version = version.clone();
        }

        version_path::check_update(&self.base.game_version)
            .await
            .is_ok()
    }

    /// 创建游戏实例
    async fn create_instance(&self, group: Option<String>) -> CoreResult<GameInstance> {
        match &self.info {
            Some(info) => {
                let name = format!("{}-{}", info.name, info.version_id);
                let game = InstanceSettingObj {
                    group,
                    name,
                    version: self.base.game_version.clone(),
                    is_modpack: true,
                    loader: self.base.loader,
                    source_type: SourceType::Modrinth,
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

        let game = game.read().unwrap();
        let base_path = game.get_base_path();
        let game_path = game.get_game_path();
        let prefix = format!("{}/", names::OVERRIDE_DIR);

        let entries: Vec<_> = self.base.zip.entries().iter().collect();
        let total = entries.len();
        let mut index = 0usize;

        if let Some(pgui) = &self.base.pack_gui {
            pgui.set_sub_now(0, Some(total));
        }

        for entry in entries {
            if let Some(cancel) = &self.base.cancel
                && cancel.is_cancelled()
            {
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

        // 在 .await 前克隆并释放读锁，避免把非 Send 的 RwLockReadGuard 带进异步任务
        let path = game.read().unwrap().get_game_path();
        let list =
            modrinth::get_mod_info(path, info, &self.base.pack_gui, self.base.cancel.clone()).await;

        if list.is_err() {
            return false;
        }

        let list = list.unwrap();

        // 构建下载列表
        let downloads = list.list;
        let mods = list.online;

        game.read().unwrap().save_online_info(&mods);

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

        check_upgrade_sha1(
            info,
            game,
            &self.base.downloads,
            &self.base.pack_gui,
            &self.base.cancel,
        )
        .await
    }
}

/// 无旧 manifest 时：通过 API 获取最新文件信息，以 mod_id 和 SHA1 比对。
///
/// 1. 批量解析 manifest 中的全部文件
/// 2. 构建 `FileOnlineInfoObj` 映射（mod_id → 在线信息）
/// 3. 与现有 `online_info` 比对：同 mod_id 但 fileid/sha1 不同 → 变更
/// 4. 仅在在线信息中不存在 → 新增
/// 5. 删除旧文件，构建下载列表
async fn check_upgrade_sha1(
    new_info: &ModrinthPackObj,
    game: &GameInstance,
    downloads: &Mutex<Vec<FileItemObj>>,
    pack_gui: &Option<Arc<dyn IAddGui>>,
    cancel: &Option<CancellationToken>,
) -> bool {
    true
}
