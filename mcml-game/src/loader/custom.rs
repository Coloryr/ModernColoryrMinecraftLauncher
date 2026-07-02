use std::{io::Read, path::Path};

use mcml_base::{
    file_item::{FileHash, FileItemObj},
    hash_helper::{self, HashType},
    path_helper,
};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData},
    names,
};
use zip::ZipArchive;

use crate::{
    launcher::{custom_loader_obj::CustomLoaderType, instance_setting_obj::InstanceSettingObj},
    launcher_path::{libraries_path, version_path},
    loader::{
        forge_install_obj::ForgeInstallObj,
        forge_launch_obj::{ForgeLaunchObj, ForgeLibrariesObj},
    },
};

pub struct CutsomLoaderRes {
    pub name: String,
    pub libs: Vec<FileItemObj>,
}

impl InstanceSettingObj {
    /// 分析jar
    pub async fn decode_loader_jar(&self) -> CoreResult<CutsomLoaderRes> {
        self.decode_loader_jar_with_path(self.get_loader_file())
            .await
    }

    /// 分析jar
    pub async fn decode_loader_jar_with_path<P: AsRef<Path>>(
        &self,
        path: P,
    ) -> CoreResult<CutsomLoaderRes> {
        let path = path.as_ref();
        let mut stream = path_helper::open_read(path)?;
        let mut zip = ZipArchive::new(&mut stream).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: path.to_path_buf(),
                error: err.to_string(),
            })
        })?;

        // Read version.json (ForgeLaunchObj)
        let mut version_json = String::new();
        let version_ok = match zip.by_name(names::VERSION_FILE) {
            Ok(mut file) => {
                file.read_to_string(&mut version_json).map_err(|err| {
                    ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    })
                })?;
                true
            }
            Err(_) => false,
        };

        // Read install_profile.json (ForgeInstallObj)
        let mut install_json = String::new();
        let install_ok = match zip.by_name(names::FILE_INSTALL_PROFILE) {
            Ok(mut file) => {
                file.read_to_string(&mut install_json).map_err(|err| {
                    ErrorType::ArchiveReadError(ErrorData {
                        error: err.to_string(),
                    })
                })?;
                true
            }
            Err(_) => false,
        };

        // Both files must be present
        if !version_ok || !install_ok {
            return Err(ErrorType::DataNotFound);
        }

        let obj1: ForgeLaunchObj = serde_json::from_str(&version_json).map_err(|err| {
            ErrorType::JsonError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let obj2: ForgeInstallObj = serde_json::from_str(&install_json).map_err(|err| {
            ErrorType::JsonError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let mut list = Vec::new();
        let libraries_path = libraries_path::get_lib_dir();

        // Collect all libraries from both objects
        let all_libraries: Vec<&ForgeLibrariesObj> =
            obj1.libraries.iter().chain(obj2.libraries.iter()).collect();

        for item in all_libraries {
            if !item.downloads.artifact.url.is_empty() {
                // Has download URL - check if local file already exists with correct hash
                let local = libraries_path.join(&item.downloads.artifact.path);
                let mut skip = false;
                if let Ok(mut read) = path_helper::open_read(&local) {
                    if let Ok(sha1) = hash_helper::gen_hash_from_reader(HashType::Sha1, &mut read) {
                        if sha1.eq_ignore_ascii_case(&item.downloads.artifact.sha1) {
                            skip = true;
                        }
                    }
                }
                if !skip {
                    list.push(FileItemObj {
                        name: item.name.clone(),
                        file: libraries_path.join(&item.downloads.artifact.path),
                        url: item.downloads.artifact.url.clone(),
                        hash: FileHash::Sha1(item.downloads.artifact.sha1.clone()),
                        later: Default::default(),
                    });
                }
            } else {
                // No URL - library is inside the zip archive
                let zip_path = format!("maven/{}", item.downloads.artifact.path);
                if let Ok(zip_file) = zip.by_name(&zip_path) {
                    let local = libraries_path.join(&item.downloads.artifact.path);
                    // Check if local file already exists with correct hash
                    let mut need_extract = true;
                    if let Ok(mut read) = path_helper::open_read(&local) {
                        if let Ok(sha1) =
                            hash_helper::gen_hash_from_reader(HashType::Sha1, &mut read)
                        {
                            if sha1.eq_ignore_ascii_case(&item.downloads.artifact.sha1) {
                                need_extract = false;
                            }
                        }
                    }
                    if need_extract {
                        // Extract from zip to local
                        path_helper::write_stream(&local, zip_file)?;
                    }
                }
                // If not found in zip, just skip this library
            }
        }

        // Build the loader name
        let name = if !obj2.version.starts_with(&obj2.profile) {
            format!("{}-{}", obj2.profile, obj2.version)
        } else {
            obj2.version.clone()
        };

        // Store custom loader info
        version_path::add_custom_loader(CustomLoaderType::ForgeLaunch(obj1), self.uuid);

        Ok(CutsomLoaderRes { name, libs: list })
    }

    /// 获取自定义加载器游戏参数
    pub fn get_custom_loader_game_args(&self) -> Vec<String> {
        if let Some(data) = self.get_custom_loader() {
            match data.as_ref() {
                CustomLoaderType::ForgeLaunch(forge) => {
                    let mut args = Vec::<String>::new();
                    if let Some(data) = &forge.minecraft_arguments {
                        let args1: Vec<&str> = data.split(' ').collect();
                        args1.iter().for_each(|item| {
                            args.push(String::from(*item));
                        });
                    }

                    if let Some(data) = &forge.arguments {
                        for item in data.game.iter() {
                            args.push(item.clone());
                        }
                    }

                    args
                }
            }
        } else {
            Default::default()
        }
    }

    /// 获取自定义加载器的JVM启动参数
    pub fn get_custom_loader_jvm_args(&self) -> Vec<String> {
        if let Some(data) = self.get_custom_loader() {
            match data.as_ref() {
                CustomLoaderType::ForgeLaunch(forge) => {
                    let mut args = Vec::<String>::new();

                    if let Some(data) = &forge.arguments {
                        for item in data.jvm.iter() {
                            args.push(item.clone());
                        }
                    }

                    args
                }
            }
        } else {
            Default::default()
        }
    }

    /// 获取自定义加载器主类
    pub fn get_custom_loader_mainclass(&self) -> String {
        if let Some(data) = self.get_custom_loader() {
            match data.as_ref() {
                CustomLoaderType::ForgeLaunch(forge) => {
                    forge.main_class.clone()
                }
            }
        } else {
            Default::default()
        }
    }
}
