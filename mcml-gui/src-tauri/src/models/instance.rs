//! 游戏实例模型
use serde::{Deserialize, Serialize};

/// 游戏实例信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstanceInfo {
    pub uuid: String,
    pub name: String,
    pub group: Option<String>,
    pub version: String,
    /// 游戏版本类型：release / snapshot / other
    pub version_type: Option<String>,
    pub loader: String,
    pub loader_version: Option<String>,
    pub dir: String,
    pub running: bool,
    /// 整合包平台：CurseForge / Modrinth / McMod / 本地压缩包 / 文件夹
    pub modpack_type: Option<String>,
    /// 整合包项目 ID
    pub pid: Option<String>,
    /// 整合包文件 ID
    pub fid: Option<String>,
    /// 在线网络整合包地址（ServerPack）
    pub server_url: Option<String>,
    /// 游戏内语言
    pub lang: Option<String>,
    /// 日志编码：utf8 / gbk
    pub log_encoding: Option<String>,
    /// 来源：导入的压缩包 / 文件夹 / 在线网址
    pub source: Option<String>,
}

impl InstanceInfo {
    /// 分组名（无分组时为 None，前端映射为“默认分组”）
    pub fn group_key(&self) -> Option<&str> {
        self.group.as_deref()
    }

    /// 是否正在运行
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// 是否为整合包（有平台信息）
    pub fn is_modpack(&self) -> bool {
        self.modpack_type.is_some()
    }
}
