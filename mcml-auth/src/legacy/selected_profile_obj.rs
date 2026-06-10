use serde::{Deserialize, Serialize};

/// 账户列表
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SelectedProfileObj {
    /// 账户名
    pub name: String,
    /// 账户标识
    pub id: String,
}

impl Default for SelectedProfileObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            id: Default::default(),
        }
    }
}
