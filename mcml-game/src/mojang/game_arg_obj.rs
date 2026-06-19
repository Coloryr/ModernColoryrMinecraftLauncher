use std::{
    collections::{HashMap, HashSet},
    path::Path,
    sync::Mutex,
};

use mcml_base::{
    Os,
    file_item::{FileHash, FileItemObj, LaterRun},
    get_system_info,
};
use mcml_net::{
    maven_utils,
    url_helper::{self, change_source, replace_minecraft_libraries},
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use tokio::task::spawn_blocking;

use crate::{
    GameInstanceObj, launcher_path::libraies_path,
    loader::forge, mojang::check_allow,
};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GameOsObj {
    pub name: String,
    pub arch: String,
}

impl Default for GameOsObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            arch: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GameRulesObj {
    pub action: String,
    pub os: Option<GameOsObj>,
}

impl Default for GameRulesObj {
    fn default() -> Self {
        Self {
            action: Default::default(),
            os: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ArtifactObj {
    pub path: String,
    pub sha1: String,
    pub url: String,
}

impl Default for ArtifactObj {
    fn default() -> Self {
        Self {
            path: Default::default(),
            sha1: Default::default(),
            url: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ClassifiersObj {
    #[serde(rename = "natives-linux")]
    pub natives_linux: ArtifactObj,
    #[serde(rename = "natives-osx")]
    pub natives_osx: ArtifactObj,
    #[serde(rename = "natives-windows")]
    pub natives_windows: ArtifactObj,
    #[serde(rename = "natives-windows-32")]
    pub natives_windows_32: ArtifactObj,
    #[serde(rename = "natives-windows-64")]
    pub natives_windows_64: ArtifactObj,
}

impl Default for ClassifiersObj {
    fn default() -> Self {
        Self {
            natives_linux: Default::default(),
            natives_osx: Default::default(),
            natives_windows: Default::default(),
            natives_windows_32: Default::default(),
            natives_windows_64: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GameLibrariesDownloadsObj {
    pub classifiers: Option<ClassifiersObj>,
    pub artifact: ArtifactObj,
}

impl Default for GameLibrariesDownloadsObj {
    fn default() -> Self {
        Self {
            classifiers: Default::default(),
            artifact: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ArgValue {
    Single(String),
    Multi(Vec<String>),
}

impl Default for ArgValue {
    fn default() -> Self {
        ArgValue::Single(Default::default())
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct GameJvmObj {
    pub rules: Vec<GameRulesObj>,
    pub value: ArgValue,
}

impl Default for GameJvmObj {
    fn default() -> Self {
        Self {
            rules: Default::default(),
            value: Default::default(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Conditional(GameJvmObj),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameArgumentsObj {
    pub game: Vec<Argument>,
    pub jvm: Vec<Argument>,
}

impl Default for GameArgumentsObj {
    fn default() -> Self {
        Self {
            game: Default::default(),
            jvm: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameAssetIndexObj {
    pub id: String,
    pub url: String,
}

impl Default for GameAssetIndexObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            url: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameDownloadItemObj {
    pub sha1: String,
    pub url: String,
}

impl Default for GameDownloadItemObj {
    fn default() -> Self {
        Self {
            sha1: Default::default(),
            url: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameDownloadsObj {
    pub client: GameDownloadItemObj,
}

impl Default for GameDownloadsObj {
    fn default() -> Self {
        Self {
            client: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameJavaVersionObj {
    #[serde(rename = "majorVersion")]
    pub major_version: i32,
}

impl Default for GameJavaVersionObj {
    fn default() -> Self {
        Self {
            major_version: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct GameLibrariesObj {
    pub downloads: GameLibrariesDownloadsObj,
    pub name: String,
    pub rules: Vec<GameRulesObj>,
    pub url: String,
}

impl Default for GameLibrariesObj {
    fn default() -> Self {
        Self {
            downloads: Default::default(),
            name: Default::default(),
            rules: Default::default(),
            url: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ClientObj {
    pub argument: String,
    pub file: GameDownloadItemObj,
}

impl Default for ClientObj {
    fn default() -> Self {
        Self {
            argument: Default::default(),
            file: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LoggingObj {
    pub client: ClientObj,
}

impl Default for LoggingObj {
    fn default() -> Self {
        Self {
            client: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameArgObj {
    #[serde(rename = "assetIndex")]
    pub asset_index: Option<GameAssetIndexObj>,
    pub downloads: GameDownloadsObj,
    pub id: String,
    #[serde(rename = "javaVersion")]
    pub java_version: Option<GameJavaVersionObj>,
    pub libraries: Option<Vec<GameLibrariesObj>>,
    pub logging: Option<LoggingObj>,
    #[serde(rename = "mainClass")]
    pub main_class: String,
    #[serde(rename = "minecraftArguments")]
    pub minecraft_arguments: Option<String>,
    #[serde(rename = "minimumLauncherVersion")]
    pub minimum_launcher_version: i32,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
    pub arguments: Option<GameArgumentsObj>,
}

impl Default for GameArgObj {
    fn default() -> Self {
        Self {
            asset_index: Default::default(),
            downloads: Default::default(),
            id: Default::default(),
            java_version: Default::default(),
            libraries: Default::default(),
            logging: Default::default(),
            main_class: Default::default(),
            minecraft_arguments: Default::default(),
            minimum_launcher_version: Default::default(),
            release_time: Default::default(),
            arguments: Default::default(),
        }
    }
}

impl GameArgObj {
    /// 创建游戏运行库项目
    /// - `native`: 本地库路径
    /// - `game`: 游戏实例
    /// 返回需要下载的文件列表
    pub async fn build_game_libs(
        &self,
        native: &Path,
        game: Option<GameInstanceObj>,
    ) -> Vec<FileItemObj> {
        // Clone data needed inside the spawn_blocking closure to satisfy 'static requirement
        let libraries = self.libraries.clone();
        let native = native.to_path_buf();

        // Phase 1: Process all libraries in a blocking thread
        let (mut list, mut natives, natives_arm) = spawn_blocking(move || {
            let list = Mutex::new(Vec::<FileItemObj>::new());
            let keys = Mutex::new(HashSet::<String>::new());
            let natives = Mutex::new(HashMap::<String, bool>::new());
            let natives_arm = Mutex::new(Vec::<String>::new());

            if let Some(libs) = libraries.as_ref() {
                #[cfg(debug_assertions)]
                {
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
        let sys = get_system_info();
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

                let name = format!(
                    "{}:{}-{}-natives-{}-arm64",
                    item, path[1], path[2], system
                );
                let dir = format!(
                    "{}{}/{}/{}-{}-natives-{}-arm64.jar",
                    base_dir, path[1], path[2], path[1], path[2], system
                );

                let sha1 = maven_utils::test_sha1(&dir).await;

                if let Some(data) = sha1 {
                    list.push(FileItemObj {
                        name,
                        file: libraies_path::get_base_dir().join(dir),
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
                        file: libraies_path::get_base_dir().join(dir),
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
    game: &Option<GameInstanceObj>,
) {
    // 检查规则是否允许
    if !check_allow(&item.rules) {
        return;
    }

    let mut isadd = false;
    let sys = get_system_info();

    // 旧版 - 直接使用 url 字段
    if !item.url.is_empty() {
        isadd = true;
        let file = maven_utils::version_name_to_path(&item.name);
        let mut url = format!("{}{}", item.url, file);
        change_source(&mut url);
        list.lock().unwrap().push(FileItemObj {
            name: item.name.clone(),
            file: libraies_path::get_base_dir().join(&file),
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
            file: libraies_path::get_base_dir().join(&file),
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
                file: libraies_path::get_base_dir().join(&file),
                url: replace_minecraft_libraries(&lib.url),
                hash: FileHash::Sha1(lib.sha1.clone()),
                later: LaterRun::Unpack(native.to_path_buf()),
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
                file: libraies_path::get_base_dir().join(lib.downloads.artifact.path),
                url: url_helper::replace_forge_libraries(&lib.downloads.artifact.url),
                hash: Default::default(),
                later: Default::default(),
            });
        } else {
            let dir = game.setting.get_libraries_path();
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
