use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SelectedProfileObj {
    pub name: String,
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