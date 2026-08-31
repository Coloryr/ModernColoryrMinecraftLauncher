//! 账户窗口 DTO：账户列表视图（IPC 返回 / 磁盘持久化，前端 wire camelCase）

use serde::{Deserialize, Serialize};

/// 账户列表视图（IPC 返回 / 磁盘持久化）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountStoreViewDto {
    pub accounts: Vec<AccountStoreDto>,
    pub current_uuid: Option<String>,
}

pub struct AccountStoreDto {

}