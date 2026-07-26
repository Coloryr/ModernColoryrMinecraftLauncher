use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeObj {
    pub data: CurseForgeListDataObj,
}

impl Default for CurseForgeObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeListObj {
    pub data: Vec<CurseForgeListDataObj>,
    pub pagination: CurseForgeListPaginationObj,
}

impl Default for CurseForgeListObj {
    fn default() -> Self {
        Self {
            data: Default::default(),
            pagination: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeListDataObj {
    pub id: u64,
    #[serde(rename = "classId")]
    pub class_id: u32,
    pub name: String,
    pub links: LinksObj,
    pub summary: String,
    #[serde(rename = "downloadCount")]
    pub download_count: u64,
    pub categories: Vec<CategoriesObj>,
    pub authors: Vec<AuthorsObj>,
    pub logo: LogoObj,
    pub screenshots: Vec<ScreenshotsObj>,
    #[serde(rename = "dateModified")]
    pub date_modified: String,
}

impl Default for CurseForgeListDataObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            class_id: Default::default(),
            name: Default::default(),
            links: Default::default(),
            summary: Default::default(),
            download_count: Default::default(),
            categories: Default::default(),
            authors: Default::default(),
            logo: Default::default(),
            screenshots: Default::default(),
            date_modified: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ScreenshotsObj {
    pub title: String,
    pub description: String,
    pub url: String,
}

impl Default for ScreenshotsObj {
    fn default() -> Self {
        Self {
            title: Default::default(),
            description: Default::default(),
            url: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LogoObj {
    pub url: String,
}

impl Default for LogoObj {
    fn default() -> Self {
        Self {
            url: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct AuthorsObj {
    pub name: String,
    #[serde(rename = "avatarUrl")]
    pub avatar_url: String,
}

impl Default for AuthorsObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            avatar_url: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CategoriesObj {
    pub name: String,
    #[serde(rename = "iconUrl")]
    pub icon_url: String,
    #[serde(rename = "classId")]
    pub class_id: u32,
}

impl Default for CategoriesObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            icon_url: Default::default(),
            class_id: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct LinksObj {
    #[serde(rename = "websiteUrl")]
    pub website_url: String,
}

impl Default for LinksObj {
    fn default() -> Self {
        Self {
            website_url: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct CurseForgeListPaginationObj {
    #[serde(rename = "totalCount")]
    pub total_count: u64,
}

impl Default for CurseForgeListPaginationObj {
    fn default() -> Self {
        Self {
            total_count: Default::default(),
        }
    }
}
