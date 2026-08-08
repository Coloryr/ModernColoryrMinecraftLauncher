use std::sync::Arc;

use async_trait::async_trait;
use mcml_auth::LoginObj;
use mcml_base::archives::IBaseArchiveGui;

use crate::{GameInstance, launcher::instance_setting_obj::InstanceSettingObj};

pub type LaunchGui = Option<Arc<dyn ILaunchGui>>;
pub type AddInstanceGui = Option<Arc<dyn IAddInstanceGui>>;
pub type AddModPackGui = Option<Arc<dyn IAddModPackGui>>;
pub type ProgressGui = Option<Arc<dyn IProgressGui>>;
pub type BaseArchiveGui = Option<Arc<dyn IBaseArchiveGui>>;

/// 项目安装状态
pub enum AddModPackState {
    DownloadPack,
    ReadInfo,
    GetInfo,
    DownloadFile,
    Extract,
    Done,
}

/// 实例创建界面回调
#[async_trait]
pub trait IAddInstanceGui: Send + Sync {
    /// 是否同意替换名字
    async fn name_replace(&self, name: &str) -> bool;
    /// 是否同意覆盖
    async fn overwrite(&self, obj: GameInstance) -> bool;
}

/// 整合包安装界面回调
pub trait IAddModPackGui: Send + Sync {
    /// 设置整合包安装状态
    fn set_state(&self, state: AddModPackState);
    /// 设置当前进度
    fn set_now(&self, value: usize, all: Option<usize>);
    /// 子进度文字
    fn set_sub_text(&self, text: Option<String>);
    /// 子进度
    fn set_sub_now(&self, value: usize, all: Option<usize>);
}

/// 进度条界面回调
pub trait IProgressGui: Send + Sync {
    /// 显示文字
    fn set_progress_text(&self, text: Option<String>);
    /// 进度
    fn set_progress_now(&self, value: usize, all: Option<usize>);
}

/// 实例启动状态
pub enum LaunchState {
    /// 登陆账户
    Login,
    /// 检查文件
    Check,
    /// 读取信息
    ReadInfo,
    /// 下载文件
    Download,
    /// 准备启动参数
    Jvm,
    /// 启动前运行
    Pre,
    /// 启动后运行
    Post,
    /// 结束
    End,
    LoadServerPack,
    CheckServerPack,
    DownloadServerPack,
}

/// 进程运行时机
pub enum ProcessRunType {
    /// 启动前运行
    PreLaunch,
    /// 启动后运行
    PostLaunch,
}

/// 界面回调
#[async_trait]
pub trait ILaunchGui {
    /// 启动状态修改
    fn update_state(&self, setting: &InstanceSettingObj, state: LaunchState);
    /// 登陆失败
    async fn login_fail(&self, auth: &LoginObj) -> bool;
    /// 请求是否要下载文件
    async fn requesst_download_file(&self) -> bool;
    /// 没有合适的java
    fn no_java(&self, java: i32);
    /// 是否运行启动其他进程
    fn launch_process(&self, run_type: ProcessRunType) -> bool;
}
