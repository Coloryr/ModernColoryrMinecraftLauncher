use std::path::Path;

use async_trait::async_trait;
use mcml_base::{file_item::FileItemObj, path_helper, serialize_tools, tools};
use mcml_names::{
    i18_items::error_type::{CoreResult, DataNotFoundData, ErrorType},
    names,
};
use mcml_net::urls;
use uuid::Uuid;

use crate::{
    GameInstance,
    launcher::{
        SourceType, file_online_info_obj::OnlineInfoObj, instance_setting_obj::InstanceSettingObj,
    },
    launcher_path::version_path,
    loader::LoaderType,
    modpack::{BaseModPackWorker, ModPackWorker},
    modrinth::{
        self,
        pack_obj::{ModrinthPackFileObj, ModrinthPackObj},
    },
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
    fn read_info(&mut self) -> CoreResult<()> {
        if let Some(item) = self
            .base
            .archive
            .entries()
            .iter()
            .filter(|item| item.name.eq_ignore_ascii_case(names::MODRINTH_FILE))
            .next()
        {
            let data1 = self
                .base
                .archive
                .read(&item.name)
                .and_then(|data| serialize_tools::json_from_bytes::<ModrinthPackObj>(&data))?;

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

        version_path::check_update(&self.base.game_version).await?;

        Ok(())
    }

    /// 创建游戏实例
    async fn create_instance(&self, group: Option<String>) -> CoreResult<Uuid> {
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

        let game = game.read().unwrap();
        let base_path = game.get_base_path();
        let game_path = game.get_game_path();
        let prefix = format!("{}/", names::OVERRIDE_DIR);

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
        let Some(info) = &self.info else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
        };
        let Some(game) = &self.base.game else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::GameInstance));
        };

        let path = game.read().unwrap().get_game_path();
        let list =
            modrinth::get_mod_info(path, info, &self.base.pack_gui, self.base.cancel.clone())
                .await?;

        // 构建下载列表
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
    /// - 存在旧 manifest：按 SHA1 对比，找出新增/变更/删除的文件
    /// - 无旧 manifest：通过 API 获取文件信息后按 mod_id 比对
    ///
    /// 最终将需要下载的文件存入 `base.downloads`。
    async fn check_upgrade(&self) -> CoreResult<()> {
        let Some(info) = &self.info else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
        };
        let Some(game) = &self.base.game else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::GameInstance));
        };

        // 读取上次安装时保存的整合包 manifest（base 目录）
        let old_info = {
            let game = game.read().unwrap();
            let old_manifest_path = game.get_base_path().join(names::MODRINTH_FILE);
            path_helper::open_read(&old_manifest_path)
                .ok()
                .and_then(|stream| {
                    serialize_tools::json_from_stream::<ModrinthPackObj>(stream).ok()
                })
        };

        // 获取新整合包的模组信息（下载列表 + 在线信息）
        let path = game.read().unwrap().get_game_path();
        let res =
            modrinth::get_mod_info(&path, info, &self.base.pack_gui, self.base.cancel.clone())
                .await?;

        let mut online_info = game.read().unwrap().read_online_info();

        // 需要下载的文件列表
        let mut new_downloads: Vec<FileItemObj> = Vec::new();

        if let Some(old_info) = old_info {
            // 有旧 manifest：按 SHA1 对比新旧文件
            // temp1 = 新整合包文件，temp2 = 旧整合包文件
            let mut temp1: Vec<Option<&ModrinthPackFileObj>> =
                info.files.iter().map(Some).collect();
            let mut temp2: Vec<Option<&ModrinthPackFileObj>> =
                old_info.files.iter().map(Some).collect();

            // 相同 SHA1 的文件视为同一文件，从两侧移除
            for b in 0..temp1.len() {
                let Some(item) = temp1[b] else { continue };
                for a in 0..temp2.len() {
                    let Some(item1) = temp2[a] else { continue };
                    if item.hashes.sha1 == item1.hashes.sha1 {
                        temp1[b] = None;
                        temp2[a] = None;
                    }
                }
            }

            // 新包中有、旧包没有 → 需要下载
            let add_list: Vec<&ModrinthPackFileObj> = temp1.into_iter().flatten().collect();
            // 旧包中有、新包没有 → 需要删除
            let remove_list: Vec<&ModrinthPackFileObj> = temp2.into_iter().flatten().collect();

            // 删除被移除的文件
            for item in &remove_list {
                delete_with_disabled(path.join(&item.path));

                let url = item
                    .downloads
                    .iter()
                    .find(|u| u.starts_with(&format!("{}data/", urls::MODRINTH_DOWNLOAD)));
                if let Some(url) = url {
                    let modid = tools::get_string(url, "data/", "/ver");
                    online_info.remove(&modid);
                }
            }

            // 构建下载列表并更新在线信息
            for item in &add_list {
                let Some(download) = res
                    .list
                    .iter()
                    .find(|d| d.hash.get_sha1().as_deref() == Some(item.hashes.sha1.as_str()))
                    .cloned()
                else {
                    continue;
                };
                new_downloads.push(download);

                let url = item
                    .downloads
                    .iter()
                    .find(|u| u.starts_with(&format!("{}data/", urls::MODRINTH_DOWNLOAD)));
                if let Some(url) = url {
                    let modid = tools::get_string(url, "data/", "/ver");
                    let fileid = tools::get_string(url, "versions/", "/");

                    let path_part = tools::get_path_part(&item.path);
                    online_info.remove(&modid);
                    online_info.insert(
                        modid.clone(),
                        OnlineInfoObj {
                            path: path_part.parent,
                            name: path_part.file.clone(),
                            file: path_part.file,
                            sha1: item.hashes.sha1.clone(),
                            url: url.clone(),
                            modid,
                            fileid,
                        },
                    );
                }
            }
        } else {
            // 无旧 manifest：通过 mod_id 对比在线信息
            // temp1 = 当前已安装模组，temp2 = 新整合包模组
            let temp1: Vec<OnlineInfoObj> = online_info.values().cloned().collect();
            let mut temp2: Vec<Option<OnlineInfoObj>> =
                res.online.values().cloned().map(Some).collect();

            let mut add_list: Vec<OnlineInfoObj> = Vec::new();
            let mut remove_list: Vec<OnlineInfoObj> = Vec::new();

            for item in &temp1 {
                for a in 0..temp2.len() {
                    let Some(item1) = &temp2[a] else { continue };
                    if item.modid != item1.modid {
                        continue;
                    }
                    // 同 mod_id → 从新列表中取出该模组
                    let item1 = temp2[a].take().unwrap();
                    // 同 mod_id 但 fileid/sha1 不同 → 需要更新
                    if item.fileid != item1.fileid || item.sha1 != item1.sha1 {
                        add_list.push(item1);
                        remove_list.push(item.clone());
                    }
                    break;
                }
            }

            // 新整合包中有、当前未安装 → 新增
            for item in temp2.iter().flatten() {
                add_list.push(item.clone());
            }

            // 删除旧文件
            for item in &remove_list {
                delete_with_disabled(path.join(&item.path).join(&item.file));
                online_info.remove(&item.modid);
            }

            // 构建下载列表并更新在线信息
            for item in &add_list {
                if let Some(download) = res
                    .list
                    .iter()
                    .find(|d| d.hash.get_sha1().as_deref() == Some(item.sha1.as_str()))
                    .cloned()
                {
                    new_downloads.push(download);
                }
                online_info.insert(item.modid.clone(), item.clone());
            }
        }

        // 保存更新后的在线信息
        game.read().unwrap().save_online_info(&online_info);

        // 写入当前整合包 manifest
        serialize_tools::json_to_file(
            info,
            game.read()
                .unwrap()
                .get_base_path()
                .join(names::MODRINTH_FILE),
        )?;

        // 更新下载列表
        *self.base.downloads.lock().unwrap() = new_downloads;

        Ok(())
    }
}

/// 删除文件，若文件已被禁用（追加了 `.disable`/`.disabled` 后缀）则一并删除。
fn delete_with_disabled<P: AsRef<Path>>(file: P) {
    let file = file.as_ref();
    // `delete` 在文件不存在时是无操作
    let _ = path_helper::delete(file);
    let _ = path_helper::delete(format!("{}{}", file.display(), names::DISABLE_DOT_EXT));
    let _ = path_helper::delete(format!("{}{}", file.display(), names::DISABLED_DOT_EXT));
}
