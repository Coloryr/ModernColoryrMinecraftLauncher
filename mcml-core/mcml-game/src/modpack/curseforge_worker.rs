use async_trait::async_trait;
use mcml_base::{archives::ArchiveEntryInfo, serialize_tools};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorType},
    names,
};

use crate::{
    GameInstance,
    curseforge::pack_obj::CurseForgePackObj,
    launcher::{SourceType, instance_setting_obj::InstanceSettingObj},
    launcher_path::version_path,
    loader::LoaderType,
    modpack::{BaseModPackWorker, ModPackWorker},
};

/// CurseForge整合包安装器
pub struct CurseForgeWorker {
    /// 整合包信息
    info: Option<CurseForgePackObj>,
    base: BaseModPackWorker,
}

impl CurseForgeWorker {
    pub fn new(base: BaseModPackWorker) -> Self {
        Self { info: None, base }
    }
}

#[async_trait]
impl ModPackWorker for CurseForgeWorker {
    /// 获取主信息
    fn read_info(&mut self) -> bool {
        if let Some(item) = self
            .base
            .zip
            .entries()
            .iter()
            .filter(|item| item.name.eq_ignore_ascii_case(names::MANIFEST_FILE))
            .next()
            && let Ok(data) = self
                .base
                .zip
                .read(&item.name)
                .and_then(|data| serialize_tools::json_from_bytes::<CurseForgePackObj>(&data))
        {
            self.info = Some(data);
            true
        } else {
            false
        }
    }

    /// 获取版本数据
    async fn read_version(&mut self) -> bool {
        if self.info.is_none() {
            return false;
        }

        let info = self.info.as_ref().unwrap();

        for item in info.minecraft.mod_loaders.iter() {
            if item.id.starts_with(names::FORGE_KEY) {
                self.base.loader = LoaderType::Forge;
                self.base.loader_version = item.id.replace(&format!("{}-", names::FORGE_KEY), "");
            } else if item.id.starts_with(names::FABRIC_KEY) {
                self.base.loader = LoaderType::Fabric;
                self.base.loader_version = item.id.replace(&format!("{}-", names::FABRIC_KEY), "");
            } else if item.id.starts_with(names::NEOFORGE_KEY) {
                self.base.loader = LoaderType::NeoForge;
                self.base.loader_version =
                    item.id.replace(&format!("{}-", names::NEOFORGE_KEY), "");
            } else if item.id.starts_with(names::QUILT_KEY) {
                self.base.loader = LoaderType::Quilt;
                self.base.loader_version = item.id.replace(&format!("{}-", names::QUILT_KEY), "");
            }
        }

        let minecraft = &self.info.as_ref().unwrap().minecraft.version;

        let version = &self.base.loader_version;

        if version.starts_with(&format!("{}-", minecraft)) && version.len() > minecraft.len() + 1 {
            self.base.loader_version = version[(minecraft.len() + 1)..].to_string();
        }

        self.base.game_version = minecraft.clone();

        let res = version_path::check_update(minecraft).await;

        res.is_ok()
    }

    /// 创建游戏实例
    async fn create_instance(&self, group: Option<String>) -> CoreResult<GameInstance> {
        match &self.info {
            Some(info) => {
                let name = format!("{}-{}", info.name, info.version);
                let game = InstanceSettingObj {
                    group,
                    name,
                    version: self.base.game_version.clone(),
                    is_modpack: true,
                    loader: self.base.loader,
                    source_type: SourceType::CurseForge,
                    loader_version: Some(self.base.loader_version.clone()),
                    ..Default::default()
                };

                game.create_instance(&self.base.gui).await
            }
            None => Err(ErrorType::InfoNotFound("info".to_string())),
        }
    }

    /// 解压文件
    async fn unzip(&self, unselect: Option<&Vec<&ArchiveEntryInfo>>) -> bool {
        todo!()
    }

    /// 获取模组信息
    async fn get_info(&self) -> bool {
        todo!()
    }

    /// 下载所需文件
    async fn download(&self) {
        todo!()
    }

    /// 更新游戏实例版本信息
    fn update_game(&mut self, game: &GameInstance) {
        self.base.game = Some(game.clone());

        let mut game = game.write().unwrap();
        game.loader = self.base.loader;
        game.loader_version = Some(self.base.loader_version.clone());
        game.version = self.base.game_version.clone();

        game.save();
    }

    /// 检查更新
    async fn check_upgrade(&self) -> bool {
        todo!()
    }
}
