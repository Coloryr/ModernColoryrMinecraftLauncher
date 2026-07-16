/// 游戏实例运行库相关

use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Mutex,
};

use mcml_base::{
    Os,
    file_item::{FileHash, FileItemObj, LaterRun},
};
use mcml_net::{maven_utils, url_helper};
use tokio::task;

use crate::{
    launcher::instance_setting_obj::InstanceSettingObj,
    launcher_path::libraries_path,
    loader::forge,
    mojang::{
        self,
        game_arg_obj::{GameArgObj, GameLibrariesObj},
    },
};

impl GameArgObj {
    /// 创建游戏运行库项目
    /// - `native`: 本地库路径
    /// - `game`: 游戏实例
    /// 返回需要下载的文件列表
    pub async fn build_game_libraries(
        &self,
        native: &Path,
        game: Option<InstanceSettingObj>,
    ) -> Vec<FileItemObj> {
        // Clone data needed inside the spawn_blocking closure to satisfy 'static requirement
        let libraries = self.libraries.clone();
        let native = native.to_path_buf();

        // Phase 1: Process all libraries in a blocking thread
        let (mut list, mut natives, natives_arm) = task::spawn_blocking(move || {
            let list = Mutex::new(Vec::<FileItemObj>::new());
            let keys = Mutex::new(HashSet::<String>::new());
            let natives = Mutex::new(HashMap::<String, bool>::new());
            let natives_arm = Mutex::new(Vec::<String>::new());

            if let Some(libs) = libraries.as_ref() {
                #[cfg(debug_assertions)]
                {
                    use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

                    libs.par_iter().for_each(|item| {
                        process_one_library(
                            item,
                            &list,
                            &keys,
                            &natives,
                            &natives_arm,
                            &native,
                            &game,
                        );
                    });
                }
                #[cfg(not(debug_assertions))]
                {
                    for item in libs.iter() {
                        process_one_library(
                            item,
                            &list,
                            &keys,
                            &natives,
                            &natives_arm,
                            &native,
                            &game,
                        );
                    }
                }
            }

            (
                list.into_inner().unwrap(),
                natives.into_inner().unwrap(),
                natives_arm.into_inner().unwrap(),
            )
        })
        .await
        .unwrap();

        // Phase 2: ARM native library fallback (runs in async context for network calls)
        let sys = mcml_base::get_system_info();
        if sys.is_arm {
            for item in natives_arm.iter() {
                natives.remove(item);
            }

            for item in natives.keys() {
                let path: Vec<&str> = item.split(':').collect();
                let path1: Vec<&str> = path[0].split('.').collect();

                let mut base_dir = String::new();
                for item1 in path1 {
                    base_dir.push_str(item1);
                    base_dir.push('/');
                }

                let system = match sys.os {
                    Os::Linux => "linux",
                    Os::MacOS => "macos",
                    _ => "windows",
                };

                let name = format!("{}:{}-{}-natives-{}-arm64", item, path[1], path[2], system);
                let dir = format!(
                    "{}{}/{}/{}-{}-natives-{}-arm64.jar",
                    base_dir, path[1], path[2], path[1], path[2], system
                );

                let sha1 = maven_utils::test_sha1(&dir).await;

                if let Some(data) = sha1 {
                    list.push(FileItemObj {
                        name,
                        file: libraries_path::get_lib_dir().join(dir),
                        url: data.url,
                        hash: FileHash::Sha1(data.sha1),
                        later: Default::default(),
                    });
                }

                let name = format!(
                    "{}:{}-{}-natives-{}-aarch_64",
                    item, path[1], path[2], system
                );
                let dir = format!(
                    "{}{}/{}/{}-{}-natives-{}-aarch_64.jar",
                    base_dir, path[1], path[2], path[1], path[2], system
                );

                let sha1 = maven_utils::test_sha1(&dir).await;

                if let Some(data) = sha1 {
                    list.push(FileItemObj {
                        name,
                        file: libraries_path::get_lib_dir().join(dir),
                        url: data.url,
                        hash: FileHash::Sha1(data.sha1),
                        later: Default::default(),
                    });
                }
            }
        }

        list
    }
}

/// 处理单个库项目
fn process_one_library(
    item: &GameLibrariesObj,
    list: &Mutex<Vec<FileItemObj>>,
    keys: &Mutex<HashSet<String>>,
    natives: &Mutex<HashMap<String, bool>>,
    natives_arm: &Mutex<Vec<String>>,
    native: &Path,
    game: &Option<InstanceSettingObj>,
) {
    // 检查规则是否允许
    if !mojang::check_allow(&item.rules) {
        return;
    }

    let mut isadd = false;
    let sys = mcml_base::get_system_info();

    // 旧版 - 直接使用 url 字段
    if !item.url.is_empty() {
        isadd = true;
        let file = maven_utils::version_name_to_path(&item.name);
        let mut url = format!("{}{}", item.url, file);
        url_helper::change_source(&mut url);
        list.lock().unwrap().push(FileItemObj {
            name: item.name.clone(),
            file: libraries_path::get_lib_dir().join(&file),
            url,
            hash: Default::default(),
            later: Default::default(),
        });
    }

    // 全系统 artifact 处理
    if !item.downloads.artifact.path.is_empty() {
        // SHA1 去重：如果已存在且没有 native 分类器则跳过
        {
            let keys = keys.lock().unwrap();
            if keys.contains(&item.downloads.artifact.sha1) && item.downloads.classifiers.is_none()
            {
                return;
            }
        }

        isadd = true;

        // Natives 追踪
        if item.name.contains("natives") {
            if let Some(index) = item.name.rfind(':') {
                let key = &item.name[..index];
                if item.name.ends_with("arm64") || item.name.ends_with("aarch_64") {
                    natives_arm.lock().unwrap().push(key.to_string());
                    natives.lock().unwrap().remove(key);
                } else {
                    natives.lock().unwrap().insert(key.to_string(), true);
                }
            }
        }

        // 确定文件路径
        let file = if item.downloads.artifact.path.is_empty() {
            maven_utils::version_name_to_path(&item.name)
        } else {
            item.downloads.artifact.path.clone()
        };

        let url = url_helper::replace_minecraft_libraries(&item.downloads.artifact.url);

        list.lock().unwrap().push(FileItemObj {
            name: item.name.clone(),
            file: libraries_path::get_lib_dir().join(&file),
            url,
            hash: FileHash::Sha1(item.downloads.artifact.sha1.clone()),
            later: Default::default(),
        });

        keys.lock()
            .unwrap()
            .insert(item.downloads.artifact.sha1.clone());
    }

    // 分系统 classifiers (native) 处理
    if let Some(classifiers) = &item.downloads.classifiers {
        // 根据操作系统选择对应的 native 库
        let mut lib = match sys.os {
            Os::Windows => &classifiers.natives_windows,
            Os::Linux => &classifiers.natives_linux,
            Os::MacOS => &classifiers.natives_osx,
            _ => return, // 不支持的操作系统，跳过该库的 native 处理
        };

        // Windows 下如果主 native 为空，尝试 32/64 位特定版本
        if lib.path.is_empty() && sys.os == Os::Windows {
            use mcml_base::ArchEnum;
            if sys.system_arch == ArchEnum::X86 {
                lib = &classifiers.natives_windows_32;
            } else {
                lib = &classifiers.natives_windows_64;
            }
        }

        if !lib.path.is_empty() {
            // SHA1 去重
            {
                let keys = keys.lock().unwrap();
                if keys.contains(&lib.sha1) {
                    return;
                }
            }

            isadd = true;

            natives.lock().unwrap().insert(item.name.clone(), true);

            let file = if lib.path.is_empty() {
                maven_utils::version_name_to_path(&format!("{}-native{}", item.name, sys.os))
            } else {
                lib.path.clone()
            };

            list.lock().unwrap().push(FileItemObj {
                name: format!("{}-native{}", item.name, sys.os),
                file: libraries_path::get_lib_dir().join(&file),
                url: url_helper::replace_minecraft_libraries(&lib.url),
                hash: FileHash::Sha1(lib.sha1.clone()),
                later: LaterRun::UnpackNative(native.to_path_buf()),
            });

            keys.lock().unwrap().insert(lib.sha1.clone());
        }
    }

    // 游戏目录回退 - 库未找到时尝试从游戏实例目录寻找
    if !isadd
        && !item.name.is_empty()
        && let Some(game) = game
    {
        let lib = forge::make_forge_libraries(&item.name);
        if let Some(lib) = lib {
            list.lock().unwrap().push(FileItemObj {
                name: item.name.clone(),
                file: libraries_path::get_lib_dir().join(lib.downloads.artifact.path),
                url: url_helper::replace_forge_libraries(&lib.downloads.artifact.url),
                hash: Default::default(),
                later: Default::default(),
            });
        } else {
            let dir = game.get_libraries_path();
            if dir.exists() && dir.is_dir() {
                let file = dir.join(maven_utils::version_name_to_path(&item.name));
                if file.exists() && file.is_file() {
                    list.lock().unwrap().push(FileItemObj {
                        name: item.name.clone(),
                        file,
                        url: Default::default(),
                        hash: Default::default(),
                        later: Default::default(),
                    });
                }
            }
        }
    }
}
