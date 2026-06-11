use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use mcml_auth::LoginObj;
use mcml_base::file_item::FileItemObj;

use crate::{launcher::game_setting_obj::GameSettingObj, mojang::game_arg_obj::GameAssetIndexObj};

/// 游戏启动时的配置存储
pub struct GameLaunchObj {
    /// 游戏运行库
    pub game_libs: Vec<FileItemObj>,
    /// 加载器运行库
    pub loader_libs: Vec<FileItemObj>,
    /// 加载器安装运行库
    pub installer_libs: Vec<FileItemObj>,
    /// Jvm启动参数
    pub jvm_args: Vec<String>,
    /// 游戏启动参数
    pub game_args: Vec<String>,
    /// java版本
    pub java_versions: HashSet<i32>,
    /// 主类
    pub main_class: String,
    /// 本地库路径
    pub native_dir: String,
    /// 资源文件
    pub assets: GameAssetIndexObj,
    /// 游戏jar
    pub game_jar: FileItemObj,
    /// 安全log4j
    pub log4j_xml: Option<FileItemObj>,
    /// 是否使用ColorASM
    pub use_asm: bool,
}

/// 游戏实例实际运行使用的参数
pub struct GameRunObj {
    /// 游戏实例
    pub obj: Arc<GameSettingObj>,
    /// 登陆的账户
    pub auth: Arc<LoginObj>,
    /// 运行路径
    pub path: String,
    /// 启动参数
    pub args: Vec<String>,
    /// 运行环境
    pub env: HashMap<String, String>,
    /// 是否管理员方式启动
    pub admin: bool,
}
