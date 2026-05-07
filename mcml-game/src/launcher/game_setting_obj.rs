use mcml_config::config_obj::{RunArgObj, WindowSettingObj};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    launcher::{LoaderType, LogEncoding, SourceType},
    mojang::GameType,
};

/// 加入服务器设置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ServerObj {
    /// 服务器地址
    #[serde(rename = "IP")]
    pub ip: Option<String>,
    /// 服务器端口
    #[serde(rename = "Port")]
    pub port: Option<u16>,
}

impl Default for ServerObj {
    fn default() -> Self {
        Self {
            ip: Default::default(),
            port: Default::default(),
        }
    }
}

/// 端口代理设置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ProxyHostObj {
    /// 服务器地址
    #[serde(rename = "IP")]
    pub ip: Option<String>,
    /// 服务器端口
    #[serde(rename = "Port")]
    pub port: Option<u16>,

    /// 服务器地址
    #[serde(rename = "User")]
    pub user: Option<String>,
    /// 服务器地址
    #[serde(rename = "Password")]
    pub password: Option<String>,
}

impl Default for ProxyHostObj {
    fn default() -> Self {
        Self {
            ip: Default::default(),
            port: Default::default(),
            user: Default::default(),
            password: Default::default(),
        }
    }
}

/// 高级Jvm启动参数
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AdvanceJvmObj {
    /// 自定义mainclass
    #[serde(rename = "MainClass")]
    pub main_class: Option<String>,
    /// 附加classpath
    #[serde(rename = "ClassPath")]
    pub class_path: Option<String>,
}

impl Default for AdvanceJvmObj {
    fn default() -> Self {
        Self {
            main_class: Default::default(),
            class_path: Default::default(),
        }
    }
}

/// 自定义模组加载器设置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CustomLoaderObj {
    /// 后加载原版运行库
    #[serde(rename = "OffLib")]
    pub off_list: bool,
    /// 删除原版运行库
    #[serde(rename = "RemoveLib")]
    pub remove_lib: bool,
    /// 是否启用自定义启动配置
    #[serde(rename = "CustomJson")]
    pub custom_json: bool,
    /// 删除原有启动配置
    #[serde(rename = "RemoveJson")]
    pub remove_json: bool,
}

impl Default for CustomLoaderObj {
    fn default() -> Self {
        Self {
            off_list: Default::default(),
            remove_lib: Default::default(),
            custom_json: Default::default(),
            remove_json: Default::default(),
        }
    }
}

/// 游戏实例配置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameSettingObj {
    /// 实例标识
    #[serde(rename = "UUID")]
    pub uuid: Uuid,
    /// 实例名
    #[serde(rename = "Name")]
    pub name: String,
    /// 实例组名
    #[serde(rename = "GroupName")]
    pub group: Option<String>,
    /// 路径名
    #[serde(rename = "DirName")]
    pub dir: String,
    /// 游戏版本
    #[serde(rename = "Version")]
    pub version: String,
    /// 模组加载器类型
    #[serde(rename = "Loader")]
    pub loader: LoaderType,
    /// 模组加载器版本
    #[serde(rename = "LoaderVersion")]
    pub loader_version: Option<String>,
    /// Jvm参数
    #[serde(rename = "JvmArg")]
    pub jvm_arm: Option<RunArgObj>,
    /// Jvm名字
    #[serde(rename = "JvmName")]
    pub jvm_name: Option<String>,
    /// Jvm路径
    #[serde(rename = "JvmLocal")]
    pub jvm_local: Option<String>,
    /// 窗口设置
    #[serde(rename = "Window")]
    pub window: Option<WindowSettingObj>,
    /// 加入服务器设置
    #[serde(rename = "StartServer")]
    pub start_server: Option<ServerObj>,
    /// 端口代理设置
    #[serde(rename = "ProxyHost")]
    pub proxy_host: Option<ProxyHostObj>,
    /// 高级Jvm设置
    #[serde(rename = "AdvanceJvm")]
    pub advance_jvm: Option<AdvanceJvmObj>,
    /// 是否为整合包
    #[serde(rename = "Modpack")]
    pub is_modpack: bool,
    /// 整合包类型
    #[serde(rename = "ModPackType")]
    pub source_type: SourceType,
    /// 游戏发布类型
    #[serde(rename = "GameType")]
    pub game_type: GameType,
    /// 整合包项目
    #[serde(rename = "PID")]
    pub pid: Option<String>,
    /// 整合包版本
    #[serde(rename = "FID")]
    pub fid: Option<String>,
    /// 图标
    #[serde(rename = "Icon")]
    pub icon: Option<String>,
    /// 服务器实例网址
    #[serde(rename = "ServerUrl")]
    pub server_url: Option<String>,
    /// 自定义模组加载器
    #[serde(rename = "CustomLoader")]
    pub custom_loader: Option<CustomLoaderObj>,
    /// 日志编码
    #[serde(rename = "Encoding")]
    pub encoding: LogEncoding,
}

impl Default for GameSettingObj {
    fn default() -> Self {
        Self {
            uuid: Default::default(),
            name: Default::default(),
            group: Default::default(),
            dir: Default::default(),
            version: Default::default(),
            loader: Default::default(),
            loader_version: Default::default(),
            jvm_arm: Default::default(),
            jvm_name: Default::default(),
            jvm_local: Default::default(),
            window: Default::default(),
            start_server: Default::default(),
            proxy_host: Default::default(),
            advance_jvm: Default::default(),
            is_modpack: Default::default(),
            source_type: Default::default(),
            game_type: Default::default(),
            pid: Default::default(),
            fid: Default::default(),
            icon: Default::default(),
            server_url: Default::default(),
            custom_loader: Default::default(),
            encoding: Default::default(),
        }
    }
}
