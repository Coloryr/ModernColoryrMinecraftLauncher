use std::{collections::HashMap, io::Read};

use mcml_base::{
    file_item::{FileHash, FileItemObj},
    path_helper,
};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData},
    names,
};
use mcml_net::{
    maven_utils::{self},
    url_helper, urls,
};
use zip::ZipArchive;

use crate::{
    launcher::instance_setting_obj::InstanceSettingObj,
    launcher_path::{libraries_path, version_path},
    loader::{
        LoaderType,
        forge_install_obj::{ForgeInstallObj, ForgeInstallOldObj},
        forge_launch_obj::{ForgeDownloadsObj, ForgeLaunchObj, ForgeLibrariesObj},
    },
    mojang::{game_arg_obj::ArtifactObj, version_checker},
};

/// 根据名字构建运行库信息
/// - `name`: 运行库名字
pub fn make_forge_libraries(name: &str) -> Option<ForgeLibrariesObj> {
    let args: Vec<&str> = name.split(':').collect();
    if args.len() < 2 {
        None
    } else if args[0] == "net.minecraftforge" && args[1] == "forge" {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: Default::default(),
                    sha1: Default::default(),
                    url: Default::default(),
                },
            },
        })
    } else if args[0] == "net.minecraft" && args[1] == "launchwrapper" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!(
                        "net/minecraft/launchwrapper/{}/launchwrapper-{}.jar",
                        args[2], args[2]
                    ),
                    sha1: Default::default(),
                    url: format!(
                        "https://libraries.minecraft.net/net/minecraft/launchwrapper/{}/launchwrapper-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else if args[0] == "org.ow2.asm" && args[1] == "asm-all" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!("org/ow2/asm/asm-all/{}/asm-all-{}.jar", args[2], args[2]),
                    sha1: Default::default(),
                    url: format!(
                        "https://maven.minecraftforge.net/org/ow2/asm/asm-all/{}/asm-all-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else if args[0] == "lzma" && args[1] == "lzma" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!("lzma/lzma/{}/lzma-{}.jar", args[2], args[2]),
                    sha1: Default::default(),
                    url: format!(
                        "https://libraries.minecraft.net/lzma/lzma/{}/lzma-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else if args[0] == "net.sf.jopt-simple" && args[1] == "jopt-simple" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!(
                        "net/sf/jopt-simple/jopt-simple/{}/jopt-simple-{}.jar",
                        args[2], args[2]
                    ),
                    sha1: Default::default(),
                    url: format!(
                        "https://libraries.minecraft.net/net/sf/jopt-simple/jopt-simple/{}/jopt-simple-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else if args[0] == "com.google.guava" && args[1] == "guava" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!("com/google/guava/guava/{}/guava-{}.jar", args[2], args[2]),
                    sha1: Default::default(),
                    url: format!(
                        "https://libraries.minecraft.net/com/google/guava/guava/{}/guava-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else if args[0] == "org.apache.commons" && args[1] == "commons-lang3" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!(
                        "org/apache/commons/commons-lang3/{}/commons-lang3-{}.jar",
                        args[2], args[2]
                    ),
                    sha1: Default::default(),
                    url: format!(
                        "https://maven.minecraftforge.net/org/apache/commons/commons-lang3/{}/commons-lang3-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else if args[0] == "net.java.jinput" && args[1] == "jinput" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!("net/java/jinput/jinput/{}/jinput-{}.jar", args[2], args[2]),
                    sha1: Default::default(),
                    url: format!(
                        "https://maven.minecraftforge.net/net/java/jinput/jinput/{}/jinput-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else if args[0] == "net.java.jutils" && args[1] == "jutils" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!("net/java/jutils/jutils/{}/jutils-{}.jar", args[2], args[2]),
                    sha1: Default::default(),
                    url: format!(
                        "https://maven.minecraftforge.net/net/java/jutils/jutils/{}/jutils-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else if args[0] == "java3d" && args[1] == "vecmath" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!("java3d/vecmath/{}/vecmath-{}.jar", args[2], args[2]),
                    sha1: Default::default(),
                    url: format!(
                        "https://libraries.minecraft.net/java3d/vecmath/{}/vecmath-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else if args[0] == "net.sf.trove4j" && args[1] == "trove4j" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!("net/sf/trove4j/trove4j/{}/trove4j-{}.jar", args[2], args[2]),
                    sha1: Default::default(),
                    url: format!(
                        "https://maven.minecraftforge.net/net/sf/trove4j/trove4j/{}/trove4j-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else if args[0] == "io.netty" && args[1] == "netty-all" && args.len() >= 3 {
        Some(ForgeLibrariesObj {
            name: String::from(name),
            downloads: ForgeDownloadsObj {
                artifact: ArtifactObj {
                    path: format!("io/netty/netty-all/{}/netty-all-{}.jar", args[2], args[2]),
                    sha1: Default::default(),
                    url: format!(
                        "https://maven.minecraftforge.net/io/netty/netty-all/{}/netty-all-{}.jar",
                        args[2], args[2]
                    ),
                },
            },
        })
    } else {
        None
    }
}

/// 创建Forge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `jar`: 类型
async fn build_forge_item(mc: &str, version: &str, jar_type: &str, hash: bool) -> FileItemObj {
    let mut version = String::from(version);
    version.push_str(&url_helper::forge_url_fix(&mc));

    let name = format!("forge-{mc}-{version}-{jar_type}.jar");
    let url = url_helper::get_forge_jar(mc, &version) + &name;

    let hash = if hash {
        maven_utils::try_get_hash(&url).await
    } else {
        FileHash::None
    };
    FileItemObj {
        name: format!("net.minecraftforge:{mc}-{version}-{jar_type}"),
        file: libraries_path::get_lib_dir()
            .join("net")
            .join("minecraftforge")
            .join("forge")
            .join(format!("{mc}-{version}"))
            .join(name),
        url,
        hash,
        later: Default::default(),
    }
}

/// 创建Forge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `jar`: 类型
async fn build_neoforge_item(mc: &str, version: &str, jar: &str, hash: bool) -> FileItemObj {
    let v2222 = version_checker::is_game_version_1202(mc);
    let name = if v2222 {
        format!("neoforge-{version}-{jar}")
    } else {
        format!("forge-{mc}-{version}-{jar}")
    };

    let base_url = url_helper::get_neoforge_jar(v2222, mc, version);
    let base_path = libraries_path::get_lib_dir().join("net").join("neoforged");

    let url = format!("{base_url}{name}.jar");
    let hash = if hash {
        maven_utils::try_get_hash(&url).await
    } else {
        FileHash::None
    };
    FileItemObj {
        name: format!("net.neoforged:{name}"),
        file: if v2222 {
            base_path
                .join("neoforge")
                .join(version)
                .join(format!("{name}.jar"))
        } else {
            base_path
                .join("forge")
                .join(format!("{mc}-{version}"))
                .join(format!("{name}.jar"))
        },
        url,
        hash,
        later: Default::default(),
    }
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub async fn build_forge_installer(mc: &str, version: &str, hash: bool) -> FileItemObj {
    build_forge_item(mc, version, names::FILE_INSTALLER, hash).await
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub async fn build_forge_universal(mc: &str, version: &str, hash: bool) -> FileItemObj {
    build_forge_item(mc, version, names::FILE_UNIVERSAL, hash).await
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub async fn build_forge_client(mc: &str, version: &str, hash: bool) -> FileItemObj {
    build_forge_item(mc, version, names::FILE_CLIENT, hash).await
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub async fn build_forge_launcher(mc: &str, version: &str, hash: bool) -> FileItemObj {
    let mut item = build_forge_item(mc, version, names::FILE_LAUNCHER, hash).await;
    item.url = format!(
        "{}net/minecraftforge/forge/{mc}-{version}/forge-{mc}-{version}-{}.jar",
        urls::FORGE,
        names::FILE_LAUNCHER
    );
    item
}

/// 创建NeoForge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub async fn build_neoforge_installer(mc: &str, version: &str, hash: bool) -> FileItemObj {
    build_neoforge_item(mc, version, names::FILE_INSTALLER, hash).await
}

/// 创建NeoForge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub async fn build_neoforge_universal(mc: &str, version: &str, hash: bool) -> FileItemObj {
    build_neoforge_item(mc, version, names::FILE_UNIVERSAL, hash).await
}

/// 创建NeoForge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub async fn build_neoforge_client(mc: &str, version: &str, hash: bool) -> FileItemObj {
    build_neoforge_item(mc, version, names::FILE_CLIENT, hash).await
}

/// 构建Forge运行库下载项目列表
/// - `info`: 运行库列表
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `neo`: 是否为NeoForge
/// - `v2`: 是否为1.13以上
/// - `install`: 是否为安装器
pub async fn build_forge_libs(
    info: &Vec<ForgeLibrariesObj>,
    mc: &str,
    version: &str,
    neo: bool,
    v2: bool,
    install: bool,
) -> Vec<FileItemObj> {
    let mut list = HashMap::<String, FileItemObj>::new();

    let mut universal = false;
    let mut installer = false;
    let mut launcher = false;

    for item in info {
        if item.downloads.artifact.path.is_empty() {
            continue;
        }

        let obj = if item.downloads.artifact.url.is_empty() {
            FileItemObj {
                name: item.name.clone(),
                file: libraries_path::get_lib_dir().join(&item.downloads.artifact.path),
                url: Default::default(),
                hash: FileHash::Sha1(item.downloads.artifact.sha1.clone()),
                later: Default::default(),
            }
        } else {
            FileItemObj {
                name: item.name.clone(),
                file: libraries_path::get_lib_dir().join(&item.downloads.artifact.path),
                url: item.downloads.artifact.url.clone(),
                hash: FileHash::Sha1(item.downloads.artifact.sha1.clone()),
                later: Default::default(),
            }
        };

        list.insert(item.name.clone(), obj);

        if item.name.ends_with(names::FILE_UNIVERSAL) {
            universal = true;
        } else if item.name.ends_with(names::FILE_INSTALLER) {
            installer = true;
        } else if item.name.ends_with(names::FILE_LAUNCHER) {
            launcher = true;
        }
    }

    if !installer && install {
        list.insert(
            String::from(names::FILE_INSTALLER),
            if neo {
                build_neoforge_installer(mc, version, false).await
            } else {
                build_forge_installer(mc, version, false).await
            },
        );
    }

    if !universal {
        if !neo || version_checker::is_game_version_1202(mc) {
            list.insert(
                String::from(names::FILE_UNIVERSAL),
                if neo {
                    build_neoforge_universal(mc, version, false).await
                } else {
                    build_forge_universal(mc, version, false).await
                },
            );
        }
    }

    if v2 && !version_checker::is_game_version_117(mc) && !launcher {
        list.insert(
            String::from(names::FILE_LAUNCHER),
            build_forge_launcher(mc, version, false).await,
        );
    }

    let mut list1 = Vec::<FileItemObj>::new();

    for (_, item) in list.drain() {
        list1.push(item);
    }

    list1
}

pub struct ForgeGetFilesObj {
    pub loaders: Vec<FileItemObj>,
    pub installs: Vec<FileItemObj>,
}

async fn get_forge_libs(mc: &str, version: &str, neo: bool) -> CoreResult<ForgeGetFilesObj> {
    let ver = version_path::get_version(mc)?;
    let v2 = ver.is_game_version_v2();

    let installer = if neo {
        build_neoforge_installer(mc, version, true).await
    } else {
        build_forge_installer(mc, version, true).await
    };

    if !installer.check_hash() {
        let res = mcml_downloader::run_download_task(vec![installer.clone()]).await;
        if !res {
            return Err(ErrorType::InfoNotFound(mc.to_string()));
        }
    }

    let stream = path_helper::open_read(&installer.file)?;
    let mut zip = ZipArchive::new(stream).map_err(|err| {
        ErrorType::ArchiveOpenError(FileSystemErrorData {
            path: installer.file.clone(),
            error: err.to_string(),
        })
    })?;

    // Read version.json
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

    // Read install_profile.json
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

    if version_ok && install_ok {
        // 1.12.2以上 新版Forge
        let info = serde_json::from_str::<ForgeLaunchObj>(&version_json).map_err(|err| {
            ErrorType::SerializerError(ErrorData {
                error: err.to_string(),
            })
        })?;
        let info = version_path::add_forge(info, &version_json.into_bytes(), mc, version, neo);
        let loaders = info.build_forge_libs(mc, version, neo, v2, false).await;

        let install_info =
            serde_json::from_str::<ForgeInstallObj>(&install_json).map_err(|err| {
                ErrorType::SerializerError(ErrorData {
                    error: err.to_string(),
                })
            })?;
        let install_info = version_path::add_forge_install(
            install_info,
            &install_json.into_bytes(),
            mc,
            version,
            neo,
        );
        let installs = install_info
            .build_forge_libs(mc, version, neo, v2, true)
            .await;

        Ok(ForgeGetFilesObj { loaders, installs })
    } else if install_ok {
        // 旧版Forge
        let obj = serde_json::from_str::<ForgeInstallOldObj>(&install_json).map_err(|err| {
            ErrorType::SerializerError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let mut libraries: Vec<ForgeLibrariesObj> = Vec::new();
        for item in &obj.version_info.libraries {
            if let Some(lib) = make_forge_libraries(&item.name) {
                libraries.push(lib);
            } else if !item.url.is_empty() {
                let path = mcml_net::maven_utils::version_name_to_path(&item.name);
                libraries.push(ForgeLibrariesObj {
                    name: item.name.clone(),
                    downloads: ForgeDownloadsObj {
                        artifact: ArtifactObj {
                            url: format!("{}{}", item.url, path),
                            path,
                            sha1: Default::default(),
                        },
                    },
                });
            }
        }

        let info = ForgeLaunchObj {
            main_class: obj.version_info.main_class,
            minecraft_arguments: Some(obj.version_info.minecraft_arguments),
            libraries,
            ..Default::default()
        };

        let json_bytes = serde_json::to_vec(&info).map_err(|err| {
            ErrorType::SerializerError(ErrorData {
                error: err.to_string(),
            })
        })?;
        let info = version_path::add_forge(info, &json_bytes, mc, version, neo);

        Ok(ForgeGetFilesObj {
            loaders: info.build_forge_libs(mc, version, neo, v2, true).await,
            installs: Vec::new(),
        })
    } else {
        Err(ErrorType::InfoNotFound(mc.to_string()))
    }
}

impl ForgeLaunchObj {
    /// 从启动信息构建Forge运行库下载项目列表
    /// - `mc`: 游戏版本号
    /// - `version`: forge版本号
    /// - `neo`: 是否为NeoForge
    /// - `v2`: 是否为1.13以上
    /// - `install`: 是否为安装器
    pub async fn build_forge_libs(
        &self,
        mc: &str,
        version: &str,
        neo: bool,
        v2: bool,
        install: bool,
    ) -> Vec<FileItemObj> {
        build_forge_libs(&self.libraries, mc, version, neo, v2, install).await
    }
}

impl ForgeInstallObj {
    /// 从安装信息构建Forge运行库下载项目列表
    /// - `mc`: 游戏版本号
    /// - `version`: forge版本号
    /// - `neo`: 是否为NeoForge
    /// - `v2`: 是否为1.13以上
    /// - `install`: 是否为安装器
    pub async fn build_forge_libs(
        &self,
        mc: &str,
        version: &str,
        neo: bool,
        v2: bool,
        install: bool,
    ) -> Vec<FileItemObj> {
        build_forge_libs(&self.libraries, mc, version, neo, v2, install).await
    }
}

impl InstanceSettingObj {
    /// 获取Forge下载项目
    pub async fn get_forge_libs(&self) -> CoreResult<ForgeGetFilesObj> {
        get_forge_libs(
            &self.version,
            &self.loader_version.as_ref().unwrap(),
            self.loader == LoaderType::NeoForge,
        )
        .await
    }

    /// 创建Forge安装器下载项目
    pub async fn build_forge_installer(&self, hash: bool) -> FileItemObj {
        build_forge_installer(&self.version, &self.loader_version.as_ref().unwrap(), hash).await
    }

    /// 创建Forge安装器下载项目
    pub async fn build_forge_universal(&self, hash: bool) -> FileItemObj {
        build_forge_universal(&self.version, &self.loader_version.as_ref().unwrap(), hash).await
    }

    /// 创建Forge安装器下载项目
    pub async fn build_forge_client(&self, hash: bool) -> FileItemObj {
        build_forge_client(&self.version, &self.loader_version.as_ref().unwrap(), hash).await
    }

    /// 创建NeoForge安装器下载项目
    pub async fn build_neoforge_installer(&self, hash: bool) -> FileItemObj {
        build_neoforge_installer(&self.version, &self.loader_version.as_ref().unwrap(), hash).await
    }

    /// 创建NeoForge下载项目
    pub async fn build_neoforge_universal(&self, hash: bool) -> FileItemObj {
        build_neoforge_universal(&self.version, &self.loader_version.as_ref().unwrap(), hash).await
    }

    /// 创建NeoForge下载项目
    pub async fn build_neoforge_client(&self, hash: bool) -> FileItemObj {
        build_neoforge_client(&self.version, &self.loader_version.as_ref().unwrap(), hash).await
    }
}
