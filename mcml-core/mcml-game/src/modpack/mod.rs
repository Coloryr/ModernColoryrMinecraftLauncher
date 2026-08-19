use std::sync::Mutex;

use async_trait::async_trait;
use mcml_base::{archives::BaseArchive, file_item::FileItemObj};
use mcml_names::i18_items::error_type::{CoreResult, DataNotFoundData, ErrorType};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    GameInstance,
    gui_hook::{AddInstanceGui, AddModPackGui, BaseArchiveGui},
    loader::LoaderType,
};

pub mod curseforge_worker;
pub mod modrinth_worker;

/// 整合包安装器
#[async_trait]
pub(crate) trait ModPackWorker {
    /// 获取主信息
    fn read_info(&mut self) -> CoreResult<()>;
    /// 获取版本数据
    async fn read_version(&mut self) -> CoreResult<()>;
    /// 创建游戏实例
    async fn create_instance(
        &self,
        name: Option<String>,
        group: Option<String>,
    ) -> CoreResult<Uuid>;
    /// 解压文件
    async fn extract(&self, unselect: Option<Vec<String>>) -> CoreResult<()>;
    /// 获取模组信息
    async fn get_info(&self) -> CoreResult<bool>;
    /// 下载所需文件
    async fn download(&self);
    /// 更新游戏实例版本信息
    fn update_game(&mut self, game: &GameInstance);
    /// 检查更新
    async fn check_upgrade(&self) -> CoreResult<()>;
}

/// 整合包安装器
pub struct BaseModPackWorker {
    /// 压缩包
    pub archive: BaseArchive,
    /// 界面
    pub instance_gui: AddInstanceGui,
    /// 界面
    pub pack_gui: AddModPackGui,
    /// 界面
    pub archive_gui: BaseArchiveGui,
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
    pub cancel: CancellationToken,
}

impl BaseModPackWorker {
    pub fn new(
        archive: BaseArchive,
        instance_gui: AddInstanceGui,
        pack_gui: AddModPackGui,
        archive_gui: BaseArchiveGui,
        cancel: CancellationToken,
    ) -> Self {
        Self {
            archive,
            instance_gui,
            pack_gui,
            archive_gui,
            loader: LoaderType::Normal,
            loader_version: String::new(),
            game_version: String::new(),
            game: None,
            downloads: Mutex::new(Vec::new()),
            cancel,
        }
    }

    /// 解压整合包覆盖文件到游戏目录。
    ///
    /// `prefix/` 下的文件去除前缀后写入游戏根目录；其余文件直接写入
    /// 游戏基础目录。进度通过 `archive_gui` 上报。
    pub fn extract_pack_files(
        &self,
        prefix: &str,
        unselect: Option<Vec<String>>,
    ) -> CoreResult<()> {
        let Some(game) = &self.game else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::GameInstance));
        };

        // 解压期间不持有实例锁
        let (base_path, game_path) = {
            let game = game.read().unwrap();
            (game.get_base_path(), game.get_game_path())
        };
        let prefix = format!("{prefix}/");

        if self.cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }

        self.archive.extract_where(
            |entry| {
                // 跳过不需要解压的条目
                if let Some(ref unselect) = unselect {
                    if unselect.iter().any(|u| u == &entry.name) {
                        return None;
                    }
                }
                let output = if let Some(rel) = entry.name.strip_prefix(&prefix) {
                    // 覆盖文件：去除 prefix 前缀后放到游戏根目录
                    game_path.join(rel)
                } else {
                    base_path.join(&entry.name)
                };
                Some(output)
            },
            self.archive_gui.as_deref(),
        )
    }
}
