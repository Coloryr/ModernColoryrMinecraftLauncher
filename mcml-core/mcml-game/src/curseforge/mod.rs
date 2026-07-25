use std::{path::Path, sync::OnceLock};

use mcml_base::file_item::{FileHash, FileItemObj, LaterRun::None};
use mcml_downloader::download_item::DownloadItem;
use mcml_net::urls;

use crate::{
    curseforge::{
        curseforge_categories_obj::CurseForgeCategoriesObj, curseforge_mod_obj::CurseForgeDataObj,
    },
    loader::LoaderType,
};

pub mod curseforge_categories_obj;
pub mod curseforge_mod_obj;
pub mod curseforge_obj;
pub mod curseforge_pack_obj;

static CATEGORIES: OnceLock<CurseForgeCategoriesObj> = OnceLock::new();

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
            later: None,
        }
    }
}
