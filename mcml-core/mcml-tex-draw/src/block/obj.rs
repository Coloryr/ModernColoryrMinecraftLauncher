use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct BlocksObj {
    /// 已经渲染好的游戏版本
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Tex")]
    pub tex: HashMap<String, String>,
    #[serde(rename = "Name")]
    pub name: HashMap<String, String>,
    /// 创造模式分类（itemGroup lang键尾段，如 buildingBlocks/natural）
    #[serde(rename = "Cat")]
    pub cat: HashMap<String, String>,
}

impl Default for BlocksObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            tex: Default::default(),
            name: Default::default(),
            cat: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct ItemsObj {
    /// 已经渲染好的游戏版本
    #[serde(rename = "Id")]
    pub id: String,
    #[serde(rename = "Tex")]
    pub tex: HashMap<String, String>,
    #[serde(rename = "Name")]
    pub name: HashMap<String, String>,
    /// 创造模式分类（itemGroup lang键尾段，与BlocksObj.cat同一套）
    #[serde(rename = "Cat")]
    pub cat: HashMap<String, String>,
}

impl Default for ItemsObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            tex: Default::default(),
            name: Default::default(),
            cat: Default::default(),
        }
    }
}
