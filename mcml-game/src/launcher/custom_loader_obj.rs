use crate::loader::forge_install_obj::ForgeInstallObj;

/// 自定义加载器类型
#[derive(Debug)]
pub enum CustomLoaderType {
    /// 类Forge加载器
    ForgeLaunch(ForgeInstallObj)
}