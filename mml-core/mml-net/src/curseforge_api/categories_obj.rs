//! CurseForge 分类 DTO

use serde::{Deserialize, Serialize};

/// CurseForge 分类列表
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct CurseForgeCategoriesObj {
    /// 分类数据列表
    pub data: Vec<CurseForgeCategoriesDataObj>,
}

impl Default for CurseForgeCategoriesObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

/// 单个分类
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct CurseForgeCategoriesDataObj {
    /// 分类 ID
    pub id: u64,
    /// 分类名称
    pub name: String,
    /// 所属大类 ID（区分模组 / 整合包等）
    #[serde(rename = "classId")]
    pub class_id: u32,
}

impl Default for CurseForgeCategoriesDataObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            name: Default::default(),
            class_id: Default::default(),
        }
    }
}
