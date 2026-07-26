use std::{
    collections::HashMap,
    path::Path,
    sync::{Mutex, OnceLock},
};

use mcml_base::file_item::{FileHash, FileItemObj, LaterRun};
use mcml_names::{i18_items::error_type::CoreResult, names};
use mcml_net::{
    curseforge_api::{self},
    urls,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

use crate::{
    curseforge::{
        curseforge_categories_obj::CurseForgeCategoriesObj,
        curseforge_mod_obj::CurseForgeDataObj,
        curseforge_obj::CurseForgeObj,
        curseforge_pack_obj::CurseForgePackObj,
        curseforge_version_obj::{CurseForgeVersionObj, CurseForgeVersionTypeObj},
    },
    data_res::{DownloadItemRes, ItemPathRes},
    gui_hook::{IAddModPackGui, IProgressGui},
    launcher::{
        FileType, file_online_info_obj::FileOnlineInfoObj, instance_setting_obj::InstanceSettingObj,
    },
    loader::LoaderType,
};

pub mod curseforge_categories_obj;
pub mod curseforge_mod_obj;
pub mod curseforge_obj;
pub mod curseforge_pack_obj;
pub mod curseforge_version_obj;

static CATEGORIES: OnceLock<CurseForgeCategoriesObj> = OnceLock::new();
static VERSIONS: OnceLock<Vec<String>> = OnceLock::new();

pub struct GetCurseForgeModDependenciesRes {
    pub name: String,
    pub mod_id: u64,
    pub opt: bool,
    pub list: Vec<CurseForgeDataObj>,
}

fn loader_to_index(loader: LoaderType) -> u32 {
    match loader {
        LoaderType::Forge => 1,
        LoaderType::Fabric => 4,
        LoaderType::Quilt => 5,
        LoaderType::NeoForge => 6,
        _ => 0,
    }
}

impl CurseForgeDataObj {
    /// 修正下载地址
    pub fn fix_download_url(&mut self) {
        if self.download_url.is_none() {
            self.download_url = Some(format!(
                "{}files/{}/{}/{}",
                urls::CURSEFORGE_DOWNLOAD,
                self.id / 1000,
                self.id % 1000,
                self.file_name
            ))
        }
    }

    /// 创建下载项目
    pub fn make_file_item_obj<P: AsRef<Path>>(&mut self, path: P) -> FileItemObj {
        self.fix_download_url();

        let mut hash = self.hashes.iter().filter(|item| item.algo == 1);

        let hash = hash
            .next()
            .map(|data| FileHash::Sha1(data.value.clone()))
            .unwrap_or_default();

        FileItemObj {
            url: self.download_url.clone().unwrap(),
            name: self.display_name.clone(),
            file: path.as_ref().join(&self.file_name),
            hash,
            later: LaterRun::None,
        }
    }

    /// 创建在线文件信息
    pub fn make_file_online_info_obj(&mut self, path: &str) -> FileOnlineInfoObj {
        self.fix_download_url();

        let mut hash = self.hashes.iter().filter(|item| item.algo == 1);

        let hash = hash
            .next()
            .map(|data| data.value.clone())
            .unwrap_or_default();

        FileOnlineInfoObj {
            path: path.to_string(),
            name: self.display_name.clone(),
            file: self.file_name.clone(),
            sha1: hash,
            url: self.download_url.clone().unwrap_or_default(),
            modid: self.mod_id.to_string(),
            fileid: self.id.to_string(),
        }
    }

    /// 获取模组依赖
    pub async fn get_mod_dependencies(
        &self,
        version: &str,
        loader: LoaderType,
    ) -> Vec<GetCurseForgeModDependenciesRes> {
        let mut ids = Mutex::new(Vec::new());

        self.get_mod_dependencies_inner(version, loader, &mut ids)
            .await
    }

    /// 获取模组依赖
    async fn get_mod_dependencies_inner(
        &self,
        version: &str,
        loader: LoaderType,
        ids: &mut Mutex<Vec<u64>>,
    ) -> Vec<GetCurseForgeModDependenciesRes> {
        match self.dependencies {
            Some(dep) => {
                if dep.is_empty() {
                    Vec::new()
                } else {
                    let list = Mutex::new(Vec::new());
                    tokio::task::spawn_blocking(move || {
                        dep.par_iter().for_each(|item| async {
                            if ids.lock().unwrap().contains(item) {
                                return;
                            }

                            // let opt = item
                        });

                        list.into()
                    })
                    .await
                    .unwrap_or_default()
                }
            }
            None => Vec::new(),
        }
    }
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
                let list = curseforge_api::get_categories::<CurseForgeCategoriesObj>().await?;
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
                let mut list =
                    curseforge_api::get_version_type::<CurseForgeVersionTypeObj>().await?;

                list.data.retain(|item| item.name.starts_with("Minecraft "));

                // 排序: ID > 17 的降序在前, ID < 18 的升序在后
                list.data.sort_by(|a, b| {
                    let a_id: i32 = a.id.parse().unwrap_or(0);
                    let b_id: i32 = b.id.parse().unwrap_or(0);
                    match (a_id > 17, b_id > 17) {
                        (true, true) => b_id.cmp(&a_id),
                        (false, false) => a_id.cmp(&b_id),
                        (true, false) => std::cmp::Ordering::Less,
                        (false, true) => std::cmp::Ordering::Greater,
                    }
                });

                let version_list = curseforge_api::get_version::<CurseForgeVersionObj>().await?;

                let mut result = vec![String::new()];
                for vtype in &list.data {
                    let vtype_id: u32 = vtype.id.parse().unwrap_or(0);
                    if let Some(version_data) =
                        version_list.data.iter().find(|v| v.verion_type == vtype_id)
                    {
                        result.extend(version_data.versions.clone());
                    }
                }

                let _ = VERSIONS.set(result.clone());
                Ok(result)
            }
        }
    }
}

impl InstanceSettingObj {
    /// 获取整合包模组信息
    pub async fn get_modpack_info(
        &self,
        obj: &mut CurseForgePackObj,
        gui: Option<impl IAddModPackGui + IProgressGui>,
    ) -> CoreResult<DownloadItemRes> {
        let size = obj.files.len();
        let mut now = 0;
        let mut list = Mutex::new(Vec::new());
        let mut mods = Mutex::new(HashMap::new());

        async fn build_item(
            game: &InstanceSettingObj,
            item: &mut CurseForgeDataObj,
            list: &mut Mutex<Vec<FileItemObj>>,
            mods: &mut Mutex<HashMap<String, FileOnlineInfoObj>>,
        ) -> CoreResult<()> {
            let path = game.get_item_path(item).await?;
            let mut item1 = item.make_file_item_obj(&path.file_path);
            let modid = item.mod_id.to_string();

            {
                mods.lock().unwrap().remove(&modid);
            }

            if matches!(path.file_type, FileType::Save) {
                item1.later = LaterRun::UnpackSave(game.get_saves_path());
            } else {
                mods.lock()
                    .unwrap()
                    .insert(modid, item.make_file_online_info_obj(&path.path));
            }

            list.lock().unwrap().push(item1);

            Ok(())
        }

        let list2 = obj.files.iter().map(|item| item.file_id).collect();
        let list1 = curseforge_api::get_files::<Vec<CurseForgeDataObj>>(list2).await?;

        tokio::task::spawn_blocking(move || {
            list1.par_iter().for_each(|item| {
                build_item(self, obj, &mut list, &mut mods).await;
            });
        })
        .await
        .unwrap_or_default();

        Ok(DownloadItemRes {
            list: list.into(),
            online: mods.into(),
        })
    }

    async fn get_item_path(&self, item: &CurseForgeDataObj) -> CoreResult<ItemPathRes> {
        let mut item1 = ItemPathRes {
            file_path: self.get_mods_path(),
            path: names::GAME_MODS_DIR.to_string(),
            file_type: FileType::Mod,
        };

        if !item.file_name.ends_with(names::JAR_DOT_EXT) {
            let info1 =
                curseforge_api::get_mod_info::<CurseForgeObj>(&item.mod_id.to_string()).await?;
            for item2 in info1.data.categories.iter() {
                if item2.class_id == curseforge_api::CLASS_RESOURCEPACKS {
                    item1.change_to_resourcepacks(self);
                    break;
                } else if item2.class_id == curseforge_api::CLASS_SHADERPACKS {
                    item1.change_to_shaderpacks(self);
                    break;
                } else if item2.class_id == curseforge_api::CLASS_SAVES {
                    item1.change_to_saves(self);
                    break;
                } else if item2.class_id == curseforge_api::CLASS_OPENLOADER_DATAPACK {
                    item1.change_to_openloader_datapack(self);
                    break;
                }
            }

            if info1.data.class_id == curseforge_api::CLASS_SAVES {
                item1.change_to_saves(self);
            } else if info1.data.class_id == curseforge_api::CLASS_OPENLOADER_DATAPACK {
                item1.change_to_openloader_datapack(self);
            }
        }

        Ok(item1)
    }
}
