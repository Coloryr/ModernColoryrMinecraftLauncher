use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;

use crate::launcher::instance_setting_obj::InstanceSettingObj;

/// 项目安装状态
pub enum AdddModPackState {
    DownloadPack,
    ReadInfo,
    GetInfo,
    DownloadFile,
    Unzip,
}

/// 实例创建界面回调
#[async_trait]
pub trait IAddInstanceGui {
    /// 是否同意替换名字
    async fn name_replace(&self, name: &str) -> bool;
    /// 是否同意覆盖
    async fn overwrite(&self, obj: Arc<InstanceSettingObj>) -> bool;
}

/// 进度条界面回调
pub trait IProgressGui {
    /// 进度
    fn set_now_process(value: i32, all: i32);
}

/// 整合包安装界面回调
pub trait IAddModPackGui {
    /// 设置整合包安装状态
    fn set_state(state: AdddModPackState);
    /// 设置当前进度
    fn set_now(value: i32, all: i32);
    /// 显示文件
    fn set_sub_text(text: Option<String>);
    /// 子进度
    fn set_sub_now(value: i32, all: i32);
}

/// 复制文件界面回调
pub trait ICopyGui {
    /// 更新数量
    fn update(&self, index: usize, count: usize);
    /// 当前文件
    fn file(&self, file: PathBuf);
}