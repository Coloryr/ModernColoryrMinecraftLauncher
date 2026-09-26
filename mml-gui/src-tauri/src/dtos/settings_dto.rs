//! 设置窗口 DTO（TS 命名，camelCase wire）
//!
//! 网络设置对应 core `HttpObj`，启动设置对应 `RunArgObj` + Java 列表；
//! core 枚举（serde_repr 数字形态）在 wire 上转成变体名字符串，
//! 与 gui_config 的枚举 wire 约定一致（值即 Rust 变体名）。

use serde::{Deserialize, Serialize};

use mml_config::config_obj::{DnsObj, GameCheckObj, HttpObj, ProxyState, ProxyType, RunArgObj, SourceLocal};

use super::JavaInfoDto;

/// 下载源 → wire 字符串（`Offical` 是 core 的既定拼写，保持一致）
fn source_to_str(s: SourceLocal) -> String {
    match s {
        SourceLocal::Offical => "Offical".to_string(),
        SourceLocal::Bmclapi => "Bmclapi".to_string(),
    }
}

fn source_from_str(s: &str) -> SourceLocal {
    match s {
        "Bmclapi" => SourceLocal::Bmclapi,
        _ => SourceLocal::Offical,
    }
}

fn proxy_state_to_str(s: ProxyState) -> String {
    match s {
        ProxyState::Auto => "Auto".to_string(),
        ProxyState::None => "None".to_string(),
        ProxyState::User => "User".to_string(),
    }
}

fn proxy_state_from_str(s: &str) -> ProxyState {
    match s {
        "None" => ProxyState::None,
        "User" => ProxyState::User,
        _ => ProxyState::Auto,
    }
}

fn proxy_type_to_str(t: ProxyType) -> String {
    match t {
        ProxyType::Http => "Http".to_string(),
        ProxyType::Sock4 => "Sock4".to_string(),
        ProxyType::Sock5 => "Sock5".to_string(),
    }
}

fn proxy_type_from_str(s: &str) -> ProxyType {
    match s {
        "Sock4" => ProxyType::Sock4,
        "Sock5" => ProxyType::Sock5,
        _ => ProxyType::Http,
    }
}

/// 网络设置（前端 wire：camelCase）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct NetworkSettingDto {
    /// 下载源：Offical / Bmclapi
    pub source: String,
    /// 下载并发线程数
    pub download_thread: u32,
    /// 代理服务器 IP
    pub proxy_ip: String,
    /// 代理服务器端口
    pub proxy_port: u16,
    /// 代理认证用户名
    pub proxy_user: String,
    /// 代理认证密码
    pub proxy_password: String,
    /// 下载等一般请求的代理策略：Auto / None / User
    pub work_proxy: String,
    /// 下载等一般请求的代理类型：Http / Sock4 / Sock5
    pub work_proxy_type: String,
    /// 登录请求的代理策略：Auto / None / User
    pub login_proxy: String,
    /// 登录请求的代理类型：Http / Sock4 / Sock5
    pub login_proxy_type: String,
    /// 是否校验下载文件完整性（SHA1）
    pub check_file: bool,
    /// 是否自动下载缺失文件
    pub auto_download: bool,
    /// 自定义 DNS（DoH）
    pub dns: DnsSettingDto,
    /// 游戏文件检查
    pub check: GameCheckSettingDto,
}

impl From<&HttpObj> for NetworkSettingDto {
    fn from(h: &HttpObj) -> Self {
        Self {
            source: source_to_str(h.source),
            download_thread: h.download_thread,
            proxy_ip: h.proxy_ip.clone(),
            proxy_port: h.proxy_port,
            proxy_user: h.proxy_user.clone(),
            proxy_password: h.proxy_password.clone(),
            work_proxy: proxy_state_to_str(h.work_proxy),
            work_proxy_type: proxy_type_to_str(h.work_proxy_type),
            login_proxy: proxy_state_to_str(h.login_proxy),
            login_proxy_type: proxy_type_to_str(h.login_proxy_type),
            check_file: h.check_file,
            auto_download: h.auto_download,
            dns: DnsSettingDto::default(),
            check: GameCheckSettingDto::default(),
        }
    }
}

impl From<NetworkSettingDto> for HttpObj {
    fn from(d: NetworkSettingDto) -> Self {
        Self {
            source: source_from_str(&d.source),
            download_thread: d.download_thread,
            proxy_ip: d.proxy_ip,
            proxy_port: d.proxy_port,
            proxy_user: d.proxy_user,
            proxy_password: d.proxy_password,
            work_proxy: proxy_state_from_str(&d.work_proxy),
            work_proxy_type: proxy_type_from_str(&d.work_proxy_type),
            login_proxy: proxy_state_from_str(&d.login_proxy),
            login_proxy_type: proxy_type_from_str(&d.login_proxy_type),
            check_file: d.check_file,
            auto_download: d.auto_download,
        }
    }
}

/// 自定义 DNS 设置（前端 wire：camelCase，字段即 core `DnsObj`）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct DnsSettingDto {
    /// 是否启用自定义 DNS
    pub enable: bool,
    /// DNS over HTTPS 服务器地址列表
    pub https: Vec<String>,
    /// 是否对代理连接也启用自定义 DNS
    pub http_proxy: bool,
}

impl From<&DnsObj> for DnsSettingDto {
    fn from(d: &DnsObj) -> Self {
        Self {
            enable: d.enable,
            https: d.https.clone(),
            http_proxy: d.http_proxy,
        }
    }
}

impl From<DnsSettingDto> for DnsObj {
    fn from(d: DnsSettingDto) -> Self {
        Self {
            enable: d.enable,
            https: d.https,
            http_proxy: d.http_proxy,
        }
    }
}

/// 游戏文件检查设置（前端 wire：camelCase，字段即 core `GameCheckObj`）
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct GameCheckSettingDto {
    /// 检查游戏核心 jar 是否存在
    pub core: bool,
    /// 检查运行库是否存在
    pub lib: bool,
    /// 检查资源文件是否存在
    pub assets: bool,
    /// 检查模组是否存在
    pub game_mod: bool,
    /// 校验游戏核心 SHA1
    pub core_sha1: bool,
    /// 校验运行库 SHA1
    pub lib_sha1: bool,
    /// 校验资源文件 SHA1
    pub assets_sha1: bool,
    /// 校验模组 SHA1
    pub mod_sha1: bool,
}

impl From<&GameCheckObj> for GameCheckSettingDto {
    fn from(c: &GameCheckObj) -> Self {
        Self {
            core: c.core,
            lib: c.lib,
            assets: c.assets,
            game_mod: c.game_mod,
            core_sha1: c.core_sha1,
            lib_sha1: c.lib_sha1,
            assets_sha1: c.assets_sha1,
            mod_sha1: c.mod_sha1,
        }
    }
}

impl From<GameCheckSettingDto> for GameCheckObj {
    fn from(c: GameCheckSettingDto) -> Self {
        Self {
            core: c.core,
            lib: c.lib,
            assets: c.assets,
            game_mod: c.game_mod,
            core_sha1: c.core_sha1,
            lib_sha1: c.lib_sha1,
            assets_sha1: c.assets_sha1,
            mod_sha1: c.mod_sha1,
        }
    }
}

/// 启动设置（前端 wire：camelCase）
///
/// Java 列表只读（增删走 `settings_scan_java` / `settings_add_java` /
/// `settings_remove_java`），内存与参数走 `settings_save_launch`。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LaunchSettingDto {
    /// 已添加的 Java 运行时列表
    pub java_list: Vec<JavaInfoDto>,
    /// 最小内存（MB）
    pub min_memory: u32,
    /// 最大内存（MB）
    pub max_memory: u32,
    /// 自定义 JVM 参数
    pub jvm_args: String,
    /// 自定义游戏参数
    pub game_args: String,
}

impl From<&RunArgObj> for LaunchSettingDto {
    fn from(r: &RunArgObj) -> Self {
        Self {
            java_list: Vec::new(),
            min_memory: r.min_memory.unwrap_or(512),
            max_memory: r.max_memory.unwrap_or(4096),
            jvm_args: r.jvm_args.clone().unwrap_or_default(),
            game_args: r.game_args.clone().unwrap_or_default(),
        }
    }
}

/// 背景图信息（前端 wire：camelCase，`settings_get_bg` 返回值）
///
/// 源地址与显示参数来自 `gui_config.json`，图片是后端处理落盘的
/// `bg_image.png`（base64 dataURL）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct BgInfoDto {
    /// 背景图来源（文件路径 / 网址）
    pub source: String,
    /// 处理后的背景图（dataURL）
    pub data_url: String,
    /// 不透明度（%）
    pub opacity: u32,
    /// 模糊（px）
    pub blur: u32,
    /// 原始分辨率（%）
    pub native_size: u32,
}
