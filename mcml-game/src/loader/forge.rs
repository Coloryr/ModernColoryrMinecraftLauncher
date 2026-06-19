use std::{collections::HashMap, path::PathBuf, sync::LazyLock};

use mcml_base::{
    file_item::{FileHash, FileItemObj},
    path_helper,
};
use mcml_names::{names, urls};
use mcml_net::url_helper;

use crate::{
    GameInstanceObj,
    launcher_path::{libraies_path, version_path},
    loader::{
        forge_install_obj::ForgeInstallObj,
        forge_launch_obj::{ForgeDownloadsObj, ForgeLaunchObj, ForgeLibrariesObj},
    },
    mojang::{game_arg_obj::ArtifactObj, version_checker},
};

const WRAPPER_FILE: &[u8] = include_bytes!("../../assets/ForgeWrapper-prism-2025-12-07.jar");

static FORGE_WRAPPER: LazyLock<PathBuf> = LazyLock::new(|| {
    let local = libraies_path::get_base_dir()
        .join("io")
        .join("github")
        .join("zekerzhayard")
        .join("prism-2025-12-07")
        .join("ForgeWrapper-prism-2025-12-07.jar");

    local
});

/// 准备ForgeWrapper jar
pub fn ready_forge_wrapper() -> PathBuf {
    let local = FORGE_WRAPPER.clone();

    if !local.exists() {
        path_helper::write_bytes(&local, WRAPPER_FILE);
    }

    local
}

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
fn build_forge_item(mc: &str, version: &str, jar: &str) -> FileItemObj {
    let mut version = String::from(version);
    version.push_str(&url_helper::forge_url_fix(&mc));

    FileItemObj {
        name: format!("net.minecraftforge:{mc}-{version}-{jar}"),
        file: libraies_path::get_base_dir()
            .join("net")
            .join("minecraftforge")
            .join("forge")
            .join(format!("{mc}-{version}"))
            .join(format!("forge-{mc}-{version}-{jar}.jar")),
        url: url_helper::get_forge_jar(mc, &version),
        hash: Default::default(),
        later: Default::default(),
    }
}

/// 创建Forge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `jar`: 类型
fn build_neoforge_item(mc: &str, version: &str, jar: &str) -> FileItemObj {
    let v2222 = version_checker::is_game_version_1202(mc);
    let name = if v2222 {
        format!("neoforge-{version}-{jar}")
    } else {
        format!("forge-{mc}-{version}-{jar}")
    };

    let base_url = url_helper::get_neoforge_jar(v2222, mc, version);
    let base_path = libraies_path::get_base_dir().join("net").join("neoforged");

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
        url: format!("{base_url}{name}.jar"),
        hash: Default::default(),
        later: Default::default(),
    }
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_forge_installer(mc: &str, version: &str) -> FileItemObj {
    build_forge_item(mc, version, names::FILE_INSTALLER)
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_forge_universal(mc: &str, version: &str) -> FileItemObj {
    build_forge_item(mc, version, names::FILE_UNIVERSAL)
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_forge_client(mc: &str, version: &str) -> FileItemObj {
    build_forge_item(mc, version, names::FILE_CLIENT)
}

/// 创建Forge安装器下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_forge_launcher(mc: &str, version: &str) -> FileItemObj {
    let mut item = build_forge_item(mc, version, names::FILE_LAUNCHER);
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
pub fn build_neoforge_installer(mc: &str, version: &str) -> FileItemObj {
    build_neoforge_item(mc, version, names::FILE_INSTALLER)
}

/// 创建NeoForge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_neoforge_universal(mc: &str, version: &str) -> FileItemObj {
    build_neoforge_item(mc, version, names::FILE_UNIVERSAL)
}

/// 创建NeoForge下载项目
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
pub fn build_neoforge_client(mc: &str, version: &str) -> FileItemObj {
    build_neoforge_item(mc, version, names::FILE_CLIENT)
}

/// 构建Forge运行库下载项目列表
/// - `info`: 运行库列表
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `neo`: 是否为NeoForge
/// - `v2`: 是否为1.13以上
/// - `install`: 是否为安装器
pub fn build_forge_libs(
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
                file: libraies_path::get_base_dir().join(&item.downloads.artifact.path),
                url: Default::default(),
                hash: FileHash::Sha1(item.downloads.artifact.sha1.clone()),
                later: Default::default(),
            }
        } else {
            FileItemObj {
                name: item.name.clone(),
                file: libraies_path::get_base_dir().join(&item.downloads.artifact.path),
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
                build_neoforge_installer(mc, version)
            } else {
                build_forge_installer(mc, version)
            },
        );
    }

    if !universal {
        if !neo || version_checker::is_game_version_1202(mc) {
            list.insert(
                String::from(names::FILE_UNIVERSAL),
                if neo {
                    build_neoforge_universal(mc, version)
                } else {
                    build_forge_universal(mc, version)
                },
            );
        }
    }

    if v2 && !version_checker::is_game_version_117(mc) && !launcher {
        list.insert(
            String::from(names::FILE_LAUNCHER),
            build_forge_launcher(mc, version),
        );
    }

    let mut list1 = Vec::<FileItemObj>::new();

    for (_, item) in list.drain() {
        list1.push(item);
    }

    list1
}

/// 从启动信息构建Forge运行库下载项目列表
/// - `info`: 运行库列表
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `neo`: 是否为NeoForge
/// - `v2`: 是否为1.13以上
/// - `install`: 是否为安装器
pub fn build_forge_libs_from_info(
    info: &ForgeLaunchObj,
    mc: &str,
    version: &str,
    neo: bool,
    v2: bool,
    install: bool,
) -> Vec<FileItemObj> {
    build_forge_libs(&info.libraries, mc, version, neo, v2, install)
}

/// 从安装信息构建Forge运行库下载项目列表
/// - `info`: 运行库列表
/// - `mc`: 游戏版本号
/// - `version`: forge版本号
/// - `neo`: 是否为NeoForge
/// - `v2`: 是否为1.13以上
/// - `install`: 是否为安装器
pub fn build_forge_libs_from_install(
    info: &ForgeInstallObj,
    mc: &str,
    version: &str,
    neo: bool,
    v2: bool,
    install: bool,
) -> Vec<FileItemObj> {
    build_forge_libs(&info.libraries, mc, version, neo, v2, install)
}

impl GameInstanceObj {
    /// 获取Forge下载项目
    pub fn get_forge_files(&self) {}
}

pub struct ForgeGetFilesObj {
    pub loaders: Vec<FileItemObj>,
    pub installs: Vec<FileItemObj>,
}

async fn get_forge_files(mc: &str, version: &str, neo: bool) -> Option<ForgeGetFilesObj> {
    let ver = version_path::get_version(mc)?;
    let v2 = ver.is_game_version_v2();

    let installer = if neo {
        build_neoforge_installer(mc, version)
    } else {
        build_forge_installer(mc, version)
    };

    if !installer.check_hash() {
        
    }

    None
}
