//! 启动参数模型
use serde::{Deserialize, Serialize};

/// 附加环境变量（键值对）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnvVarLine {
    pub key: String,
    pub value: String,
}

/// 实例启动参数（内存 / 窗口 / Java + 扩展参数 + 自定义执行 + 代理）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceArgs {
    /// 最大内存（MB）
    pub memory: i64,
    /// 最小内存（MB）
    pub min_memory: i64,
    pub fullscreen: bool,
    pub width: i64,
    pub height: i64,
    /// 使用的 Java 名；"custom" 表示自定义路径
    pub java_name: String,
    /// 自定义 Java 路径
    pub java_path: String,
    /// GC 回收器：auto / g1gc / zgc / none / custom
    pub gc: String,
    /// 自定义 GC 参数（gc 为 custom 时使用）
    pub gc_custom: String,
    /// 自定义主类
    pub main_class: String,
    /// 附加 JVM 参数
    pub jvm_args: Vec<String>,
    /// 附加游戏参数
    pub game_args: Vec<String>,
    /// 附加 classpath
    pub class_path: Vec<String>,
    /// 附加环境变量
    pub env_vars: Vec<EnvVarLine>,
    /// 游戏内语言
    pub lang: String,
    /// 日志编码：utf8 / gbk
    pub log_encoding: String,
    /// 启动前执行
    pub pre_enabled: bool,
    pub pre_cmd: String,
    /// 启动后执行
    pub post_enabled: bool,
    pub post_cmd: String,
    /// 游戏内代理
    pub proxy_ip: String,
    pub proxy_port: i64,
    pub proxy_user: String,
    pub proxy_pass: String,
    /// 自动加入服务器
    pub server_ip: String,
    pub server_port: i64,
    pub join_server: bool,
}

impl InstanceArgs {
    /// 默认启动参数
    pub fn default() -> Self {
        Self {
            memory: 2048,
            min_memory: 1024,
            fullscreen: false,
            width: 854,
            height: 480,
            java_name: String::new(),
            java_path: String::new(),
            gc: "auto".into(),
            gc_custom: String::new(),
            main_class: String::new(),
            jvm_args: Vec::new(),
            game_args: Vec::new(),
            class_path: Vec::new(),
            env_vars: Vec::new(),
            lang: "zh_cn".into(),
            log_encoding: "utf8".into(),
            pre_enabled: false,
            pre_cmd: String::new(),
            post_enabled: false,
            post_cmd: String::new(),
            proxy_ip: String::new(),
            proxy_port: 0,
            proxy_user: String::new(),
            proxy_pass: String::new(),
            server_ip: String::new(),
            server_port: 0,
            join_server: false,
        }
    }

    /// 使用自定义 Java 路径
    pub fn uses_custom_java(&self) -> bool {
        self.java_name == "custom"
    }

    /// 是否自动加入服务器
    pub fn should_join_server(&self) -> bool {
        self.join_server && !self.server_ip.is_empty()
    }

    /// 是否启用启动前执行
    pub fn has_pre_cmd(&self) -> bool {
        self.pre_enabled && !self.pre_cmd.is_empty()
    }

    /// 是否启用启动后执行
    pub fn has_post_cmd(&self) -> bool {
        self.post_enabled && !self.post_cmd.is_empty()
    }
}
