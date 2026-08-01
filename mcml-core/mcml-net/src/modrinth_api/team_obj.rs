use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthTeamObj {
    pub user: TeamserObj,
}

impl Default for ModrinthTeamObj {
    fn default() -> Self {
        Self {
            user: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct TeamserObj {
    pub username: String,
    pub avatar_url: String,
}

impl Default for TeamserObj {
    fn default() -> Self {
        Self {
            username: Default::default(),
            avatar_url: Default::default(),
        }
    }
}
