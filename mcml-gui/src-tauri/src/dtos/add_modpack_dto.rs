//! 下载整合包窗口 DTO：安装任务总览（多任务进度条的事件负载与查询返回）

use serde::Serialize;

/// 单个整合包安装任务的状态
///
/// state 为安装阶段 ID（与前端 i18n 键对应：downloadPack / readInfo /
/// getInfo / downloadFile / extract / done，见 add.rs `pack_state_id`）；
/// done / failed / cancelled 为终态标记，三者互斥。
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModPackTaskDto {
    /// 任务 ID（安装开始时生成，与最终实例 uuid 无关）
    pub uuid: String,
    pub source: String,
    pub pid: String,
    pub fid: String,
    /// 显示名（取自整合包列表缓存）
    pub name: String,
    pub state: String,
    pub now: u32,
    pub total: u32,
    pub sub_text: Option<String>,
    pub sub_now: u32,
    pub sub_total: u32,
    pub done: bool,
    pub failed: bool,
    pub cancelled: bool,
    /// 失败原因（failed 时有值）
    pub error: Option<String>,
    /// 安装成功的新实例 uuid（done 时有值，前端用它切换选中实例）
    pub instance_uuid: Option<String>,
}

/// 整合包安装任务总览（`add-modpack-status` 事件负载 / `add_modpack_status` 返回）
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModPackStatusDto {
    /// 下载整合包窗口是否开着（决定进度条显示在整合包窗口还是主窗口）
    pub window_open: bool,
    pub tasks: Vec<ModPackTaskDto>,
}
