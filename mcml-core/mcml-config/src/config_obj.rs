//! 配置数据结构定义
//!
//! 本模块定义了启动器配置文件中所有可配置项的 Rust 数据结构。
//! 这些结构体通过 `serde` 序列化为 JSON 格式存储到 `config.json` 中。
//!
//! # 主要配置项
//!
//! | 结构体 | 用途 |
//! |--------|------|
//! | [`ConfigObj`] | 顶层配置文件 |
//! | [`HttpObj`] | 网络/代理设置 |
//! | [`DnsObj`] | 自定义 DNS 设置 |
//! | [`RunArgObj`] | JVM 启动参数 |
//! | [`WindowSettingObj`] | 游戏窗口设置 |
//! | [`GameCheckObj`] | 游戏文件校验设置 |
//! | [`JvmConfigObj`] | Java 运行时配置 |

use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Java 虚拟机配置
///
/// 记录一个已添加的 Java 运行时的名称和路径。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct JvmConfigObj {
    /// Java 显示名称
    #[serde(rename = "Name")]
    pub name: String,
    /// Java 可执行文件的路径（相对或绝对路径）
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

/// 下载源选择
///
/// 决定从哪个镜像源下载 Minecraft 相关资源。
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SourceLocal {
    /// Mojang 官方下载源
    Offical,
    /// BMCLAPI 国内镜像源（由 bangbang93 维护）
    Bmclapi,
}

impl Default for SourceLocal {
    fn default() -> Self {
        SourceLocal::Offical
    }
}

/// 代理使用策略
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProxyState {
    /// 自动检测系统代理设置
    Auto,
    /// 不使用代理
    None,
    /// 使用用户自定义代理
    User,
}

impl Default for ProxyState {
    fn default() -> Self {
        ProxyState::Auto
    }
}

/// 代理类型
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ProxyType {
    /// HTTP 代理
    Http,
    /// SOCKS4 代理
    Sock4,
    /// SOCKS5 代理
    Sock5,
}

impl Default for ProxyType {
    fn default() -> Self {
        ProxyType::Http
    }
}

/// 启动器网络配置
///
/// 控制下载行为、代理设置和文件校验策略。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HttpObj {
    /// 资源下载源
    #[serde(rename = "Source")]
    pub source: SourceLocal,
    /// 下载并发线程数
    #[serde(rename = "DownloadThread")]
    pub download_thread: u32,
    /// 代理服务器 IP 地址
    #[serde(rename = "ProxyIP")]
    pub proxy_ip: String,
    /// 代理服务器端口
    #[serde(rename = "ProxyPort")]
    pub proxy_port: u16,
    /// 代理认证用户名
    #[serde(rename = "ProxyUser")]
    pub proxy_user: String,
    /// 代理认证密码
    #[serde(rename = "ProxyPassword")]
    pub proxy_password: String,

    /// 一般请求（下载等）的代理策略
    #[serde(rename = "ProxyWork")]
    pub work_proxy: ProxyState,
    /// 一般请求的代理类型
    #[serde(rename = "ProxyWorkType")]
    pub work_proxy_type: ProxyType,

    /// 登录请求的代理策略
    #[serde(rename = "ProxyLogin")]
    pub login_proxy: ProxyState,
    /// 登录请求的代理类型
    #[serde(rename = "ProxyLoginType")]
    pub login_proxy_type: ProxyType,

    /// 是否校验下载文件完整性（SHA1）
    #[serde(rename = "CheckFile")]
    pub check_file: bool,
    /// 是否自动下载缺失文件
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
            check_file: true,
            auto_download: true,
            work_proxy: ProxyState::Auto,
            work_proxy_type: ProxyType::Http,
            login_proxy: ProxyState::Auto,
            login_proxy_type: ProxyType::Http,
        }
    }
}

/// 自定义 DNS 设置
///
/// 支持 DNS over HTTPS（DoH），用于绕过 DNS 污染。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct DnsObj {
    /// 是否启用自定义 DNS
    #[serde(rename = "Enable")]
    pub enable: bool,
    /// DNS over HTTPS 服务器地址列表
    #[serde(rename = "Https")]
    pub https: Vec<String>,
    /// 是否对代理连接也启用自定义 DNS
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

/// JVM 垃圾回收器类型
#[derive(Serialize_repr, Deserialize_repr, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GCType {
    /// 自动选择（根据 Java 版本自动匹配合适的 GC）
    Auto,
    /// G1 垃圾回收器（适合大内存、低延迟场景）
    G1GC,
    /// ZGC（分代式 Z Garbage Collector，Java 21+ 推荐）
    ZGC,
    /// 不添加任何 GC 参数，使用 JVM 默认
    None,
}

impl Default for GCType {
    fn default() -> Self {
        GCType::Auto
    }
}

/// 游戏启动参数配置
///
/// 控制 JVM 参数、游戏参数、内存分配、启动前后执行命令等。
/// 所有字段均为 `Option` 类型，`None` 表示使用全局默认值。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct RunArgObj {
    /// 是否移除原有的 JVM 参数
    #[serde(rename = "RemoveJvmArg")]
    pub remove_jvm_arg: Option<bool>,
    /// 是否移除原有的游戏参数
    #[serde(rename = "RemoveGameArg")]
    pub remove_game_arg: Option<bool>,
    /// 自定义 JVM 参数（追加或替换）
    #[serde(rename = "JvmArgs")]
    pub jvm_args: Option<String>,
    /// 自定义游戏参数（追加或替换）
    #[serde(rename = "GameArgs")]
    pub game_args: Option<String>,
    /// 自定义 JVM 环境变量
    #[serde(rename = "JvmEnv")]
    pub jvm_env: Option<String>,
    /// GC 模式
    #[serde(rename = "GC")]
    pub gc_mode: Option<GCType>,
    /// 最大内存（MB）
    #[serde(rename = "MaxMemory")]
    pub max_memory: Option<u32>,
    /// 最小内存（MB）
    #[serde(rename = "MinMemory")]
    pub min_memory: Option<u32>,
    /// 是否启用 ColorASM（彩色日志输出）
    #[serde(rename = "ColorASM")]
    pub colorasm: Option<bool>,
    /// 是否在启动 Minecraft 前执行预启动命令
    #[serde(rename = "LaunchPre")]
    pub launch_pre_run: Option<bool>,
    /// 预启动命令是否与游戏同时运行（`true`）还是等命令结束后再启动游戏（`false`）
    #[serde(rename = "PreRunSame")]
    pub pre_run_with_game: Option<bool>,
    /// 是否在游戏结束后执行后置命令
    #[serde(rename = "LaunchPost")]
    pub launch_post_run: Option<bool>,
    /// 预启动命令内容
    #[serde(rename = "LaunchPreData")]
    pub pre_run_arg: Option<String>,
    /// 后置命令内容
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
            colorasm: Option::None,
            launch_pre_run: Option::None,
            pre_run_with_game: Option::None,
            launch_post_run: Option::None,
            pre_run_arg: Option::None,
            post_run_arg: Option::None,
        }
    }
}

impl RunArgObj {
    /// 创建带有合理默认值的启动参数
    ///
    /// 默认分配 512MB–4096MB 内存，自动选择GC参数，
    /// 预启动命令与游戏同时运行。
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
            colorasm: Option::Some(false),
            launch_pre_run: Option::Some(false),
            pre_run_with_game: Option::Some(true),
            launch_post_run: Option::Some(false),
            pre_run_arg: Option::Some(String::new()),
            post_run_arg: Option::Some(String::new()),
        }
    }
}

/// 游戏窗口设置
///
/// 控制 Minecraft 游戏窗口的大小、标题和全屏模式。
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct WindowSettingObj {
    /// 是否启动全屏模式
    #[serde(rename = "FullScreen")]
    pub full_screen: Option<bool>,
    /// 窗口宽度（像素）
    #[serde(rename = "Width")]
    pub width: Option<u16>,
    /// 窗口高度（像素）
    #[serde(rename = "Height")]
    pub height: Option<u16>,
    /// 自定义游戏窗口标题
    #[serde(rename = "GameTitle")]
    pub game_title: Option<String>,
    /// 是否启用自定义标题
    #[serde(rename = "EditTitle")]
    pub edit_title: Option<bool>,
    /// 是否使用随机标题
    #[serde(rename = "RandomTitle")]
    pub random_title: Option<bool>,
    /// 是否循环切换标题
    #[serde(rename = "CycTitle")]
    pub cycle_title: Option<bool>,
    /// 循环标题切换延迟（毫秒）
    #[serde(rename = "TitleDelay")]
    pub title_delay: Option<u32>,
}

impl WindowSettingObj {
    /// 创建默认窗口设置（1280×720，窗口模式）
    pub fn new() -> Self {
        Self {
            full_screen: Some(false),
            width: Some(1280),
            height: Some(720),
            game_title: None,
            edit_title: None,
            random_title: None,
            cycle_title: None,
            title_delay: None,
        }
    }
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

/// 游戏文件完整性检查设置
///
/// 控制启动器对游戏核心、运行库、资源文件和模组的校验行为。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct GameCheckObj {
    /// 检查游戏核心 jar 文件是否存在
    #[serde(rename = "CheckCore")]
    pub core: bool,
    /// 检查运行库文件是否存在
    #[serde(rename = "CheckLib")]
    pub lib: bool,
    /// 检查资源文件是否存在
    #[serde(rename = "CheckAssets")]
    pub assets: bool,
    /// 检查模组文件是否存在
    #[serde(rename = "CheckMod")]
    pub game_mod: bool,
    /// 校验游戏核心 SHA1
    #[serde(rename = "CheckCoreSha1")]
    pub core_sha1: bool,
    /// 校验运行库 SHA1
    #[serde(rename = "CheckLibSha1")]
    pub lib_sha1: bool,
    /// 校验资源文件 SHA1
    #[serde(rename = "CheckAssetsSha1")]
    pub assets_sha1: bool,
    /// 校验模组 SHA1
    #[serde(rename = "CheckModSha1")]
    pub mod_sha1: bool,
}

impl Default for GameCheckObj {
    fn default() -> Self {
        Self {
            core: true,
            lib: true,
            assets: true,
            game_mod: true,
            core_sha1: true,
            lib_sha1: true,
            assets_sha1: true,
            mod_sha1: true,
        }
    }
}

/// 启动器顶层配置
///
/// 包含所有可配置项，序列化为 JSON 存储。
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ConfigObj {
    /// 配置文件版本（用于自动迁移）
    #[serde(rename = "Version")]
    pub version: String,
    /// 已添加的 Java 运行时列表
    #[serde(rename = "JavaList")]
    pub java_list: Vec<JvmConfigObj>,
    /// 网络设置
    #[serde(rename = "Http")]
    pub http: HttpObj,
    /// 自定义 DNS 设置
    #[serde(rename = "Dns")]
    pub dns: DnsObj,
    /// 默认 JVM 启动参数
    #[serde(rename = "DefaultJvmArg")]
    pub jvm_arg: RunArgObj,
    /// 游戏窗口设置
    #[serde(rename = "Window")]
    pub window: WindowSettingObj,
    /// 游戏文件检查设置
    #[serde(rename = "GameCheck")]
    pub check: GameCheckObj,
}

impl Default for ConfigObj {
    fn default() -> Self {
        Self {
            version: mcml_names::VERSION.clone(),
            java_list: Vec::new(),
            http: HttpObj::default(),
            dns: DnsObj::default(),
            jvm_arg: RunArgObj::new(),
            window: WindowSettingObj::new(),
            check: GameCheckObj::default(),
        }
    }
}
