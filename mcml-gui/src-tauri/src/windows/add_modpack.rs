//! 下载整合包

use mcml_game::launcher::{FileType, ModPackType};
use mcml_net::{
    curseforge_api::{self, CurseForgeSortType},
    modrinth_api::{self, ModrinthSortType},
};

use crate::dtos::add_resource_dto::CategoriesDto;

