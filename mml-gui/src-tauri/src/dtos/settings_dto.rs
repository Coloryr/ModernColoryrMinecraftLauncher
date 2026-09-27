//! 设置窗口 DTO（TS 命名，camelCase wire）
//!
//! 网络设置对应 core `HttpObj`，启动设置对应 `RunArgObj` + Java 列表；
//! core 枚举（serde_repr 数字形态）在 wire 上转成变体名字符串，
//! 与 gui_config 的枚举 wire 约定一致（值即 Rust 变体名）。

use serde::{Deserialize, Serialize};

use mml_config::config_obj::{
    DnsObj, GameCheckObj, GCType, HttpObj, ProxyState, ProxyType, RunArgObj, SourceLocal,
    WindowSettingObj,
};

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

fn gc_to_str(g: GCType) -> String {
    match g {
        GCType::Auto => "Auto".to_string(),
        GCType::G1GC => "G1GC".to_string(),
        GCType::ZGC => "ZGC".to_string(),
        GCType::None => "None".to_string(),
    }
}

fn gc_from_str(s: &str) -> GCType {
    match s {
        "G1GC" => GCType::G1GC,
        "ZGC" => GCType::ZGC,
        "None" => GCType::None,
        _ => GCType::Auto,
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

/// 启动参数（前端 wire：camelCase，字段即 core `RunArgObj`）
///
/// core 侧是 `Option` 字段（`None` = 用全局默认），DTO 落到
/// `RunArgObj::new()` 同款默认值；保存时全部写回 `Some(...)`。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RunArgSettingDto {
    /// 是否移除原有的 JVM 参数
    pub remove_jvm_arg: bool,
    /// 是否移除原有的游戏参数
    pub remove_game_arg: bool,
    /// 自定义 JVM 参数（换行分隔多条）
    pub jvm_args: String,
    /// 自定义游戏参数（换行分隔多条）
    pub game_args: String,
    /// 自定义 JVM 环境变量
    pub jvm_env: String,
    /// GC 模式：Auto / G1GC / ZGC / None
    pub gc_mode: String,
    /// 最小内存（MB）
    pub min_memory: u32,
    /// 最大内存（MB）
    pub max_memory: u32,
    /// 是否启用 ColorASM（彩色日志输出）
    pub colorasm: bool,
    /// 是否在启动游戏前执行预启动命令
    pub launch_pre_run: bool,
    /// 预启动命令与游戏同时运行（true）还是等命令结束再启动游戏（false）
    pub pre_run_with_game: bool,
    /// 是否在游戏结束后执行后置命令
    pub launch_post_run: bool,
    /// 预启动命令内容
    pub pre_run_arg: String,
    /// 后置命令内容
    pub post_run_arg: String,
}

impl From<&RunArgObj> for RunArgSettingDto {
    fn from(r: &RunArgObj) -> Self {
        Self {
            remove_jvm_arg: r.remove_jvm_arg.unwrap_or(false),
            remove_game_arg: r.remove_game_arg.unwrap_or(false),
            jvm_args: r.jvm_args.clone().unwrap_or_default(),
            game_args: r.game_args.clone().unwrap_or_default(),
            jvm_env: r.jvm_env.clone().unwrap_or_default(),
            gc_mode: gc_to_str(r.gc_mode.unwrap_or(GCType::Auto)),
            min_memory: r.min_memory.unwrap_or(512),
            max_memory: r.max_memory.unwrap_or(4096),
            colorasm: r.colorasm.unwrap_or(false),
            launch_pre_run: r.launch_pre_run.unwrap_or(false),
            pre_run_with_game: r.pre_run_with_game.unwrap_or(true),
            launch_post_run: r.launch_post_run.unwrap_or(false),
            pre_run_arg: r.pre_run_arg.clone().unwrap_or_default(),
            post_run_arg: r.post_run_arg.clone().unwrap_or_default(),
        }
    }
}

impl From<RunArgSettingDto> for RunArgObj {
    fn from(d: RunArgSettingDto) -> Self {
        Self {
            remove_jvm_arg: Some(d.remove_jvm_arg),
            remove_game_arg: Some(d.remove_game_arg),
            jvm_args: Some(d.jvm_args),
            game_args: Some(d.game_args),
            jvm_env: Some(d.jvm_env),
            gc_mode: Some(gc_from_str(&d.gc_mode)),
            max_memory: Some(d.max_memory),
            min_memory: Some(d.min_memory),
            colorasm: Some(d.colorasm),
            launch_pre_run: Some(d.launch_pre_run),
            pre_run_with_game: Some(d.pre_run_with_game),
            launch_post_run: Some(d.launch_post_run),
            pre_run_arg: Some(d.pre_run_arg),
            post_run_arg: Some(d.post_run_arg),
        }
    }
}

/// 游戏窗口设置（前端 wire：camelCase，字段即 core `WindowSettingObj`）
///
/// core 侧是 `Option` 字段，DTO 落到 `WindowSettingObj::new()` 同款默认值
/// （`title_delay` core 尚无既有默认值，取 3000ms）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WindowSettingDto {
    /// 是否启动全屏模式
    pub full_screen: bool,
    /// 窗口宽度（像素）
    pub width: u16,
    /// 窗口高度（像素）
    pub height: u16,
    /// 是否启用自定义标题
    pub edit_title: bool,
    /// 自定义游戏窗口标题
    pub game_title: String,
    /// 是否使用随机标题
    pub random_title: bool,
    /// 是否循环切换标题
    pub cycle_title: bool,
    /// 循环标题切换间隔（毫秒）
    pub title_delay: u32,
}

impl From<&WindowSettingObj> for WindowSettingDto {
    fn from(w: &WindowSettingObj) -> Self {
        Self {
            full_screen: w.full_screen.unwrap_or(false),
            width: w.width.unwrap_or(1280),
            height: w.height.unwrap_or(720),
            edit_title: w.edit_title.unwrap_or(false),
            game_title: w.game_title.clone().unwrap_or_default(),
            random_title: w.random_title.unwrap_or(false),
            cycle_title: w.cycle_title.unwrap_or(false),
            title_delay: w.title_delay.unwrap_or(3000),
        }
    }
}

impl From<WindowSettingDto> for WindowSettingObj {
    fn from(d: WindowSettingDto) -> Self {
        Self {
            full_screen: Some(d.full_screen),
            width: Some(d.width),
            height: Some(d.height),
            game_title: Some(d.game_title),
            edit_title: Some(d.edit_title),
            random_title: Some(d.random_title),
            cycle_title: Some(d.cycle_title),
            title_delay: Some(d.title_delay),
        }
    }
}

/// 启动设置（前端 wire：camelCase）
///
/// Java 列表只读（增删走 `settings_scan_java` / `settings_add_java` /
/// `settings_remove_java`），启动参数与窗口设置走 `settings_save_launch`。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct LaunchSettingDto {
    /// 已添加的 Java 运行时列表
    pub java_list: Vec<JavaInfoDto>,
    /// 启动参数（core `RunArgObj`）
    pub run: RunArgSettingDto,
    /// 游戏窗口设置（core `WindowSettingObj`）
    pub window: WindowSettingDto,
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
