use std::path::PathBuf;

use async_trait::async_trait;

use crate::GameInstance;

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
pub trait IAddInstanceGui: Send + Sync {
    /// 是否同意替换名字
    async fn name_replace(&self, name: &str) -> bool;
    /// 是否同意覆盖
    async fn overwrite(&self, obj: GameInstance) -> bool;
}

/// 进度条界面回调
pub trait IProgressGui: Send + Sync {
    /// 进度
    fn set_now_process(&self, value: usize, all: Option<usize>);
}

/// 整合包安装界面回调
pub trait IAddModPackGui: Send + Sync {
    /// 设置整合包安装状态
    fn set_state(&self, state: AdddModPackState);
    /// 设置当前进度
    fn set_now(&self, value: usize, all: Option<usize>);
    /// 显示文件
    fn set_sub_text(&self, text: Option<String>);
    /// 子进度
    fn set_sub_now(&self, value: usize, all: Option<usize>);
}

pub trait IAddGui: IAddModPackGui + IProgressGui {}

/// 复制文件界面回调
pub trait ICopyGui {
    /// 更新数量
    fn update(&self, index: usize, count: usize);
    /// 当前文件
    fn file(&self, file: PathBuf);
}
