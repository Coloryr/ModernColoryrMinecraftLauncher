use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::core::core;

/// Jvm配置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct JvmConfigObj {
    /// 名字
    #[serde(rename = "Name")]
    pub name: String,
    /// 路径
    #[serde(rename = "Local")]
    pub local: String,
}

impl Default for JvmConfigObj {
    fn default() -> Self {
        Self {
            name: String::new(),
            local: String::new(),
        }
    }
}

/// 下载源
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SourceLocal {
    /// 官方下载源
    Offical,
    /// Bmcl下载源
    Bmclapi,
}

impl Default for SourceLocal {
    fn default() -> Self {
        SourceLocal::Offical
    }
}

/// 启动器网络配置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HttpObj {
    /// 下载源
    #[serde(rename = "Source")]
    pub source: SourceLocal,
    /// 下载线程数
    #[serde(rename = "DownloadThread")]
    pub download_thread: u32,
    /// 下载线程数
    #[serde(rename = "ProxyIP")]
    pub proxy_ip: String,
    /// 代理端口
    #[serde(rename = "ProxyPort")]
    pub proxy_port: u16,
    /// 代理用户
    #[serde(rename = "ProxyUser")]
    pub proxy_user: String,
    /// 代理密码
    #[serde(rename = "ProxyPassword")]
    pub proxy_password: String,
    /// 登录使用代理
    #[serde(rename = "LoginProxy")]
    pub login_proxy: bool,
    /// 下载使用代理
    #[serde(rename = "DownloadProxy")]
    pub download_proxy: bool,
    /// 游戏使用代理
    #[serde(rename = "GameProxy")]
    pub game_proxy: bool,
    /// 检查下载文件完整性
    #[serde(rename = "CheckFile")]
    pub check_file: bool,
    /// 自动下载缺失文件
    #[serde(rename = "AutoDownload")]
    pub auto_download: bool,
}

impl Default for HttpObj {
    fn default() -> Self {
        Self {
            source: SourceLocal::Offical,
            download_thread: 5,
            proxy_ip: String::from("127.0.0.1"),
            proxy_port: 7890,
            proxy_user: String::new(),
            proxy_password: String::new(),
            login_proxy: false,
            download_proxy: false,
            game_proxy: false,
            check_file: true,
            auto_download: true,
        }
    }
}

/// 自定义Dns设置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct DnsObj {
    /// 是否启用自定义DNS
    #[serde(rename = "Enable")]
    pub enable: bool,
    /// DNS over HTTPS地址
    #[serde(rename = "Https")]
    pub https: Vec<String>,
    /// 是否对代理也启用
    #[serde(rename = "HttpProxy")]
    pub http_proxy: bool,
}

impl Default for DnsObj {
    fn default() -> Self {
        Self {
            enable: false,
            https: Vec::new(),
            http_proxy: false,
        }
    }
}

/// JVM Gc模式
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GCType {
    /// 自动选择
    Auto,
    /// G1垃圾回收器
    G1GC,
    /// 分代式GC
    ZGC,
    /// 不添加GC参数
    None,
}

impl Default for GCType {
    fn default() -> Self {
        GCType::Auto
    }
}

/// 启动参数
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct RunArgObj {
    /// 删除原有的Jvm参数
    #[serde(rename = "RemoveJvmArg")]
    pub remove_jvm_arg: Option<bool>,
    /// 删除原有的游戏参数
    #[serde(rename = "RemoveGameArg")]
    pub remove_game_arg: Option<bool>,
    /// 自定义Jvm参数
    #[serde(rename = "JvmArgs")]
    pub jvm_args: Option<String>,
    /// 自定义游戏参数
    #[serde(rename = "GameArgs")]
    pub game_args: Option<String>,
    /// 自定义环境变量
    #[serde(rename = "JvmEnv")]
    pub jvm_env: Option<String>,
    /// GC模式
    #[serde(rename = "GC")]
    pub gc_mode: Option<GCType>,
    /// 最大内存
    #[serde(rename = "MaxMemory")]
    pub max_memory: Option<u32>,
    /// 最大内存
    #[serde(rename = "MinMemory")]
    pub min_memory: Option<u32>,
    /// 启用ColorASM
    #[serde(rename = "ColorASM")]
    pub color_asm: Option<bool>,
    /// 启动前运行
    #[serde(rename = "LaunchPre")]
    pub launch_pre_run: Option<bool>,
    /// 是否同时启动游戏
    #[serde(rename = "PreRunSame")]
    pub pre_run_with_game: Option<bool>,
    /// 启动后运行
    #[serde(rename = "LaunchPost")]
    pub launch_post_run: Option<bool>,
    /// 启动前运行
    #[serde(rename = "LaunchPreData")]
    pub pre_run_arg: Option<String>,
    /// 启动后运行
    #[serde(rename = "LaunchPostData")]
    pub post_run_arg: Option<String>,
}

impl Default for RunArgObj {
    fn default() -> Self {
        Self {
            remove_jvm_arg: Option::None,
            remove_game_arg: Option::None,
            jvm_args: Option::None,
            game_args: Option::None,
            jvm_env: Option::None,
            gc_mode: Option::None,
            max_memory: Option::None,
            min_memory: Option::None,
            color_asm: Option::None,
            launch_pre_run: Option::None,
            pre_run_with_game: Option::None,
            launch_post_run: Option::None,
            pre_run_arg: Option::None,
            post_run_arg: Option::None,
        }
    }
}

impl RunArgObj {
    pub fn new() -> Self {
        RunArgObj {
            remove_jvm_arg: Option::Some(false),
            remove_game_arg: Option::Some(false),
            jvm_args: Option::Some(String::new()),
            game_args: Option::Some(String::new()),
            jvm_env: Option::Some(String::new()),
            gc_mode: Option::Some(GCType::Auto),
            max_memory: Option::Some(512),
            min_memory: Option::Some(4096),
            color_asm: Option::Some(false),
            launch_pre_run: Option::Some(false),
            pre_run_with_game: Option::Some(true),
            launch_post_run: Option::Some(false),
            pre_run_arg: Option::Some(String::new()),
            post_run_arg: Option::Some(String::new()),
        }
    }
}

/// 游戏窗口设置
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct WindowSettingObj {
    /// 启动后运行
    #[serde(rename = "FullScreen")]
    pub full_screen: Option<bool>,
    /// 窗口宽度
    #[serde(rename = "Width")]
    pub width: Option<u16>,
    /// 窗口高度
    #[serde(rename = "Height")]
    pub height: Option<u16>,
    /// 窗口高度
    #[serde(rename = "GameTitle")]
    pub game_title: Option<String>,
    /// 是否使用自定义标题
    #[serde(rename = "EditTitle")]
    pub edit_title: Option<bool>,
    /// 随机游戏标题
    #[serde(rename = "RandomTitle")]
    pub random_title: Option<bool>,
    /// 循环游戏标题
    #[serde(rename = "CycTitle")]
    pub cycle_title: Option<bool>,
    /// 循环游戏标题延迟
    #[serde(rename = "TitleDelay")]
    pub title_delay: Option<u32>,
}

impl Default for WindowSettingObj {
    fn default() -> Self {
        Self {
            full_screen: Option::None,
            width: Option::None,
            height: Option::None,
            game_title: Option::None,
            edit_title: Option::None,
            random_title: Option::None,
            cycle_title: Option::None,
            title_delay: Option::None,
        }
    }
}

/// 配置文件
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ConfigObj {
    /// 配置文件版本
    #[serde(rename = "Version")]
    pub version: String,
    /// Java列表
    #[serde(rename = "JavaList")]
    pub java_list: Vec<JvmConfigObj>,
    /// 联网设置
    #[serde(rename = "Http")]
    pub http: HttpObj,
    /// 内置DNS设置
    #[serde(rename = "Dns")]
    pub dns: DnsObj,
    /// 启动参数
    #[serde(rename = "DefaultJvmArg")]
    pub jvm_arg: RunArgObj,
    /// 游戏窗口设置
    #[serde(rename = "Window")]
    pub window: WindowSettingObj,
}

impl Default for ConfigObj {
    fn default() -> Self {
        Self {
            version: String::from(core::VERSION),
            java_list: Vec::new(),
            http: Default::default(),
            dns: Default::default(),
            jvm_arg: RunArgObj::new(),
            window: Default::default(),
        }
    }
}
