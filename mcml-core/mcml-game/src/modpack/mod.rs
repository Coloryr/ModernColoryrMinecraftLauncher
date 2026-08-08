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
    async fn create_instance(&self, group: Option<String>) -> CoreResult<Uuid>;
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
pub(crate) struct BaseModPackWorker {
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
    pub(crate) fn extract_pack_files(
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

#[cfg(test)]
mod test {
    use std::io::Write;
    use std::sync::{Arc, RwLock};

    use mcml_base::archives::BaseArchive;
    use mcml_names::names;
    use tokio_util::sync::CancellationToken;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    use crate::{
        GameInstance,
        launcher::instance_setting_obj::InstanceSettingObj,
        modpack::BaseModPackWorker,
    };

    /// 生成一个最小的 mrpack（zip）：根目录的 `modrinth.index.json` +
    /// `overrides/` 下的覆盖文件。
    ///
    /// 二进制样本不提交进 git；这里直接程序化生成，无需网络。
    fn make_mini_mrpack() -> std::path::PathBuf {
        let zip_path = std::env::temp_dir().join(format!(
            "mcml-modpack-mini-{}.zip",
            uuid::Uuid::new_v4()
        ));
        let file = std::fs::File::create(&zip_path).expect("创建测试 mrpack 失败");
        let mut writer = ZipWriter::new(file);
        let options = SimpleFileOptions::default();

        let index = br#"{"formatVersion":1,"game":"minecraft","versionId":"1","name":"mini","files":[]}"#;
        writer.start_file(names::MODRINTH_FILE, options).unwrap();
        writer.write_all(index).unwrap();
        writer
            .start_file(
                &format!("{}/config/example.txt", names::OVERRIDE_DIR),
                options,
            )
            .unwrap();
        writer.write_all(b"hello").unwrap();
        writer
            .start_file(
                &format!("{}/config/modpack_defaults/config/bettergrass.json", names::OVERRIDE_DIR),
                options,
            )
            .unwrap();
        writer.write_all(b"{}").unwrap();
        writer.finish().unwrap();

        zip_path
    }

    /// 验证 `extract_pack_files` 的路径路由：
    /// - `overrides/` 前缀去除后写入游戏根目录（.minecraft）
    /// - 其余文件（如 `modrinth.index.json`）直接写入游戏基础目录
    /// - `unselect` 指定的条目被跳过
    #[test]
    fn extract_pack_files_routing() {
        let temp = std::env::temp_dir().join(format!("mcml-modpack-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp);
        crate::init(&temp).expect("初始化运行路径失败");

        let instance = InstanceSettingObj {
            name: "routing-test".to_string(),
            dir: "routing-test".to_string(),
            version: "26.2".to_string(),
            ..Default::default()
        };
        let base_path = instance.get_base_path();
        let game_path = instance.get_game_path();
        let game: GameInstance = Arc::new(RwLock::new(instance));

        let archive = BaseArchive::open(&make_mini_mrpack()).expect("打开测试 mrpack 失败");
        let mut worker = BaseModPackWorker::new(archive, None, None, None, CancellationToken::new());
        worker.game = Some(game.clone());

        // 第一次解压：跳过 modrinth.index.json
        worker
            .extract_pack_files(
                names::OVERRIDE_DIR,
                Some(vec![names::MODRINTH_FILE.to_string()]),
            )
            .expect("解压失败");

        // 被跳过的根文件不写入基础目录
        assert!(
            !base_path.join(names::MODRINTH_FILE).exists(),
            "unselect 的 modrinth.index.json 不应被解压"
        );
        // overrides/ 文件去掉前缀写入游戏目录
        let override_file = game_path.join("config/modpack_defaults/config/bettergrass.json");
        assert!(override_file.exists(), "overrides 文件应解压到游戏目录");

        // 第二次全量解压：根文件写入基础目录
        worker
            .extract_pack_files(names::OVERRIDE_DIR, None)
            .expect("全量解压失败");
        assert!(
            base_path.join(names::MODRINTH_FILE).exists(),
            "modrinth.index.json 应写入基础目录"
        );

        let _ = std::fs::remove_dir_all(&temp);
    }
}
