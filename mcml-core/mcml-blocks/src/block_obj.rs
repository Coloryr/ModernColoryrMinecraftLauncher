use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct BlocksObj {
    /// 已经渲染好的游戏版本
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Tex")]
    pub tex: HashMap<String, String>,
    #[serde(rename = "Name")]
    pub name: HashMap<String, String>,
}

impl Default for BlocksObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            tex: Default::default(),
            name: Default::default(),
        }
    }
}
