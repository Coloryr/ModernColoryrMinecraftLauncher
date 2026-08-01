use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use mcml_base::{
    archives::{ArchiveEntryInfo, BaseArchive},
    file_item::FileItemObj,
};
use mcml_names::i18_items::error_type::CoreResult;
use tokio_util::sync::CancellationToken;

use crate::{
    GameInstance,
    gui_hook::{IAddGui, IAddInstanceGui},
    loader::LoaderType,
};

pub mod curseforge_worker;
pub mod modrinth_worker;

/// 整合包安装器
#[async_trait]
pub trait ModPackWorker {
    /// 获取主信息
    fn read_info(&mut self) -> bool;
    /// 获取版本数据
    async fn read_version(&mut self) -> bool;
    /// 创建游戏实例
    async fn create_instance(&self, group: Option<String>) -> CoreResult<GameInstance>;
    /// 解压文件
    async fn unzip(&self, unselect: Option<&Vec<&ArchiveEntryInfo>>) -> bool;
    /// 获取模组信息
    async fn get_info(&self) -> bool;
    /// 下载所需文件
    async fn download(&self);
    /// 更新游戏实例版本信息
    fn update_game(&mut self, game: &GameInstance);
    /// 检查更新
    async fn check_upgrade(&self) -> bool;
}

/// 整合包安装器
pub struct BaseModPackWorker {
    /// 压缩包
    pub zip: BaseArchive,
    /// 界面
    pub gui: Option<Arc<dyn IAddInstanceGui>>,
    /// 更新界面
    pub pack_gui: Option<Arc<dyn IAddGui>>,
    /// 加载器类型
    pub loader: LoaderType,
    /// 加载器版本
    pub loader_version: String,
    /// 游戏版本
    pub game_version: String,
    /// 游戏实例
    pub game: Option<GameInstance>,
    /// 下载列表（Mutex 允许 `&self` 方法修改）
    pub downloads: Mutex<Vec<FileItemObj>>,
    /// 取消
    pub cancel: Option<CancellationToken>,
}

impl BaseModPackWorker {
    pub fn new(
        zip: BaseArchive,
        gui: Option<Arc<dyn IAddInstanceGui>>,
        pack_gui: Option<Arc<dyn IAddGui>>,
        cancel: Option<CancellationToken>,
    ) -> Self {
        Self {
            zip,
            gui,
            pack_gui,
            loader: LoaderType::Normal,
            loader_version: String::new(),
            game_version: String::new(),
            game: None,
            downloads: Mutex::new(Vec::new()),
            cancel,
        }
    }
}
