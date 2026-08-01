use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthVersionObj {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub date_published: String,
    pub downloads: u64,
    pub files: Vec<ModrinthVersionFileObj>,
    pub dependencies: Vec<DependencieObj>,
}

impl Default for ModrinthVersionObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            project_id: Default::default(),
            name: Default::default(),
            version_number: Default::default(),
            date_published: Default::default(),
            downloads: Default::default(),
            files: Default::default(),
            dependencies: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ModrinthVersionFileObj {
    pub hashes: HasheObj,
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
}

impl Default for ModrinthVersionFileObj {
    fn default() -> Self {
        Self {
            hashes: Default::default(),
            url: Default::default(),
            filename: Default::default(),
            primary: Default::default(),
            size: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct HasheObj {
    pub sha1: String,
    pub sha512: String,
}

impl Default for HasheObj {
    fn default() -> Self {
        Self {
            sha1: Default::default(),
            sha512: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(default)]
pub struct DependencieObj {
    pub version_id: Option<String>,
    pub project_id: String,
    //required optional
    pub dependency_type: String,
}

impl Default for DependencieObj {
    fn default() -> Self {
        Self {
            version_id: Default::default(),
            project_id: Default::default(),
            dependency_type: Default::default(),
        }
    }
}
