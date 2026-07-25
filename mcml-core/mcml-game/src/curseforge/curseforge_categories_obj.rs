use serde::{Deserialize, Serialize};

/// 分类数据
#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeCategoriesObj {
    pub data: Vec<CurseForgeCategoriesDataObj>,
}

impl Default for CurseForgeCategoriesObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeCategoriesDataObj {
    pub id: u64,
    pub name: String,
    #[serde(rename = "classId")]
    pub class_id: u64,
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
