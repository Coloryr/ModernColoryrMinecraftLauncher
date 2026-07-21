use std::{collections::HashSet, path::Path, sync::OnceLock};

use itertools::Itertools;
use mcml_base::serialize_tools;
use mcml_config::config_obj::SourceLocal;
use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use scraper::{ElementRef, Html, Selector, selectable::Selectable};
use serde::{Deserialize, Serialize};

use crate::{
    WORK_CLIENT,
    url_helper::{self, get_source},
    urls,
};

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OptifineListObj {
    pub mcversion: String,
    pub patch: String,
    #[serde(rename = "type")]
    pub rtype: String,
    pub filename: String,
    pub forge: String,
}

impl Default for OptifineListObj {
    fn default() -> Self {
        Self {
            mcversion: Default::default(),
            patch: Default::default(),
            rtype: Default::default(),
            filename: Default::default(),
            forge: Default::default(),
        }
    }
}

/// 高清修复信息
#[derive(Clone)]
pub struct GetOptifineObj {
    /// 版本号
    pub version: String,
    /// 游戏版本号
    pub mc_version: String,
    /// Forge加载器信息
    pub forge: String,
    /// 文件名
    pub file_name: String,
    /// 日期
    pub date: String,
    /// 附加信息
    pub url1: Option<String>,
    /// 附加信息
    pub url2: Option<String>,
    /// 下载源
    pub source: SourceLocal,
}

static OPTIFINE_MC_VERSION: OnceLock<HashSet<String>> = OnceLock::new();

/// 获取高清修复版本
pub async fn get_optifine_version() -> CoreResult<Vec<GetOptifineObj>> {
    let url = url_helper::get_optifine_meta();
    let mut list = Vec::<GetOptifineObj>::new();

    let data = WORK_CLIENT.get().unwrap().get_text(&url).await?;
    if get_source() == SourceLocal::Offical {
        let html = Html::parse_document(&data);
        let select = Selector::parse("tr.downloadLine").unwrap();

        for row in html.select(&select) {
            let mut col_download = None;
            let mut col_mirror = None;
            let mut col_forge = None;
            let mut col_date = None;
            let mut col_file = None;

            let td_selector = Selector::parse("td").unwrap();

            for td in row.select(&td_selector) {
                if let Some(class) = td.value().attr("class") {
                    match class {
                        "colDownload" => {
                            if let Some(child) = td.children().next() {
                                if let Some(child_elem) = ElementRef::wrap(child) {
                                    col_download =
                                        child_elem.value().attr("href").map(String::from);
                                }
                            }
                        }
                        "colMirror" => {
                            if let Some(child) = td.children().next() {
                                if let Some(child_elem) = ElementRef::wrap(child) {
                                    col_mirror = child_elem.value().attr("href").map(String::from);
                                }
                            }
                        }
                        "colForge" => {
                            col_forge = Some(td.text().collect::<String>());
                        }
                        "colDate" => {
                            col_date = Some(td.text().collect::<String>());
                        }
                        "colFile" => {
                            col_file = Some(td.text().collect::<String>());
                        }
                        _ => {}
                    }
                }
            }

            let (temp, temp1, temp2, temp3, temp4) =
                match (col_download, col_mirror, col_forge, col_date, col_file) {
                    (Some(a), Some(b), Some(c), Some(d), Some(e)) => (a, b, c, d, e),
                    _ => continue,
                };

            let file_name = Path::new(&temp1)
                .file_name()
                .and_then(|f| f.to_str())
                .unwrap_or("")
                .replace("adloadx?f=", "");

            let mc_version = file_name
                .replace("preview_OptiFine_", "")
                .replace("OptiFine_", "");

            let mc_version = if let Some(idx) = mc_version.find('_') {
                &mc_version[..idx]
            } else {
                &mc_version
            };

            let date_parts: Vec<&str> = temp3.split('.').collect();
            let formatted_date = if date_parts.len() == 3 && date_parts[2].len() == 4 {
                format!("{}.{}.{}", date_parts[2], date_parts[1], date_parts[0])
            } else {
                temp3
            };

            let version = temp4.replace("OptiFine ", "").replace(" ", "_");

            list.push(GetOptifineObj {
                file_name,
                version,
                mc_version: mc_version.to_string(),
                forge: temp2.trim().to_string(),
                date: formatted_date,
                url1: Some(temp),
                url2: Some(temp1),
                source: SourceLocal::Offical,
            });
        }

        if list.is_empty() {
            return Err(ErrorType::InfoNotFound(String::from("optifine url")));
        }
    } else {
        let mut obj = serialize_tools::json_from_str::<Vec<OptifineListObj>>(&data)?;

        for item in obj.drain(..) {
            let url = url_helper::get_optifine_jar(&item);
            list.push(GetOptifineObj {
                version: format!("{}_{}", item.rtype, item.patch),
                mc_version: item.mcversion,
                forge: item.forge,
                file_name: item.filename,
                date: String::new(),
                url1: Some(url),
                url2: None,
                source: SourceLocal::Bmclapi,
            });
        }
    }

    Ok(list)
}

/// 获取Optifine下载地址
/// - `obj`: 下载项目
pub async fn get_optifine_download(
    source: &SourceLocal,
    url1: &Option<String>,
    url2: &Option<String>,
) -> CoreResult<Option<String>> {
    match source {
        SourceLocal::Offical => {
            WORK_CLIENT
                .get()
                .unwrap()
                .get_text(&url1.as_ref().unwrap())
                .await?;

            let data = WORK_CLIENT
                .get()
                .unwrap()
                .get_text(&url2.as_ref().unwrap())
                .await?;
            let html = Html::parse_document(&data);

            let selector =
                Selector::parse("table tr td table tbody tr td table tbody tr td span a").unwrap();

            let links: Vec<_> = html.select(&selector).collect();

            if links.is_empty() {
                return Ok(None);
            }

            let href = links[0].value().attr("href").unwrap();

            Ok(Some(format!("{}{href}", urls::OPTIFINE)))
        }
        SourceLocal::Bmclapi => Ok(url1.clone()),
    }
}

/// 获取支持的游戏版本
pub async fn get_support_version() -> CoreResult<Option<HashSet<String>>> {
    match OPTIFINE_MC_VERSION.get() {
        Some(data) => Ok(Some(data.clone())),
        None => {
            let list = get_optifine_version().await?;
            let list1 = list.iter().chunk_by(|item| &item.mc_version);

            let mut list2 = HashSet::<String>::new();
            for (key, _) in &list1 {
                list2.insert(key.clone());
            }

            Ok(Some(OPTIFINE_MC_VERSION.get_or_init(|| list2).clone()))
        }
    }
}
