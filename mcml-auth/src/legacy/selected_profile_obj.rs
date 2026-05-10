use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct SelectedProfileObj {
    pub name: String,
    pub id: String,
}

impl SelectedProfileObj {
    pub fn new(name: String, id: String) -> Self {
        SelectedProfileObj { name, id }
    }
}

impl Default for SelectedProfileObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            id: Default::default(),
        }
    }
}
