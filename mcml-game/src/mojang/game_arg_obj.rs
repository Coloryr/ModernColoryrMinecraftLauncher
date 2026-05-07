use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameRulesObj {
    pub action: String,
    pub os: GameOsObj,
}

impl Default for GameRulesObj {
    fn default() -> Self {
        Self {
            action: Default::default(),
            os: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameLibrariesDownloadsObj {
    pub classifiers: ClassifiersObj,
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
            value: ArgValue::Single(Default::default()),
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

#[derive(Serialize, Deserialize, Debug)]
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
