/// 核心初始化参数
#[derive(Debug)]
pub struct CoreInitObj {
    /// 运行路径
    pub local: String,
    /// 微软登录密钥
    pub oauth_key: String,
    /// CF平台密钥
    pub curseforge_key: String,
}

impl CoreInitObj {
    /// 创建核心初始化参数
    pub fn new(local: String, oauth_key: String, curseforge_key: String) -> Self {
        CoreInitObj {
            local,
            oauth_key,
            curseforge_key,
        }
    }
}

pub mod core {
    use std::sync::OnceLock;

    use const_format::formatcp;

    use crate::{core::CoreInitObj, log::log::log};

    /// 启动器主版本号
    pub const VERSION_NUM: i32 = 1;
    /// 启动器日期
    pub const DATE: &str = "20260412";
    /// 启动器版本号
    pub const VERSION: &str = formatcp!("{}-{DATE}", VERSION_NUM);

    /// 基础运行路径
    pub static BASE_DIR: OnceLock<String> = OnceLock::new();
    /// 核心参数
    pub static CORE_ARG: OnceLock<CoreInitObj> = OnceLock::new();

    /// 初始化核心
    /// arg 核心参数
    pub fn init(arg: CoreInitObj) {
        if arg.local.is_empty() {
            panic!("Run local is empty");
        }

        CORE_ARG.set(arg).unwrap();

        BASE_DIR.set(CORE_ARG.get().unwrap().local.clone()).unwrap();

        log::init(BASE_DIR.get().unwrap().to_string());
    }
}
