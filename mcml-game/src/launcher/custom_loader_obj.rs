use crate::loader::{forge_launch_obj::ForgeLaunchObj};

/// 自定义加载器类型
#[derive(Debug)]
pub enum CustomLoaderType {
    /// 类Forge加载器
    ForgeLaunch(ForgeLaunchObj),
}
