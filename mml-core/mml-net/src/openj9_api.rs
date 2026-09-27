use itertools::Itertools;
use mml_base::serialize_tools;
use mml_names::i18_items::error_type::{CoreResult, DataNotFoundData, ErrorData, ErrorType};
use rquickjs::{AsyncContext, AsyncRuntime};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

use crate::urls;

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct ResultsObj {
    pub content: String,
    pub pagepost_custom_js_value: String,
}

impl Default for ResultsObj {
    fn default() -> Self {
        Self {
            content: Default::default(),
            pagepost_custom_js_value: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OpenJ9Obj {
    pub error: bool,
    pub results: Vec<ResultsObj>,
}

impl Default for OpenJ9Obj {
    fn default() -> Self {
        Self {
            error: Default::default(),
            results: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OptObj {
    #[serde(rename = "downloadLink")]
    pub download_link: String,
    pub checksum: String,
}

impl Default for OptObj {
    fn default() -> Self {
        Self {
            download_link: Default::default(),
            checksum: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct JdkObj {
    pub opt1: OptObj,
}

impl Default for JdkObj {
    fn default() -> Self {
        Self {
            opt1: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct DownloadObj {
    pub name: String,
    pub version: i32,
    pub os: String,
    pub arch: String,
    pub jdk: JdkObj,
    pub jre: JdkObj,
}

impl Default for DownloadObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            version: Default::default(),
            os: Default::default(),
            arch: Default::default(),
            jdk: Default::default(),
            jre: Default::default(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct OpenJ9FileObj {
    pub downloads: Vec<DownloadObj>,
}

impl Default for OpenJ9FileObj {
    fn default() -> Self {
        Self {
            downloads: Default::default(),
        }
    }
}

pub struct OpenJ9ListObj {
    pub arch: Vec<String>,
    pub os: Vec<String>,
    pub main_version: Vec<String>,
    pub download: Vec<DownloadObj>,
}

/// 获取JAVA列表
pub async fn get_java_list() -> CoreResult<OpenJ9ListObj> {
    let data: OpenJ9Obj = crate::get_work_client().get_json(urls::OPENJ9).await?;
    let data = &data.results[0];

    let html = Html::parse_document(&data.content);
    let select = Selector::parse("select").unwrap();

    let mut obj = OpenJ9ListObj {
        arch: Default::default(),
        os: Default::default(),
        main_version: Default::default(),
        download: Default::default(),
    };

    {
        let mut list = html
            .select(&select)
            .filter(|item| item.value().attr("id").unwrap_or_default() == "runtimeVersion");

        let Some(item) = list.next() else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
        };

        let select1 = Selector::parse("option").unwrap();
        let node1 = item
            .select(&select1)
            .filter(|item| item.value().classes().contains("bx--select-option"));

        for item in node1 {
            obj.main_version
                .push(item.value().attr("value").unwrap_or_default().to_string());
        }
    }

    {
        let mut list = html
            .select(&select)
            .filter(|item| item.value().attr("id").unwrap_or_default() == "operatingSystem");

        let Some(item) = list.next() else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
        };

        let select1 = Selector::parse("option").unwrap();
        let node1 = item.select(&select1);

        for item in node1 {
            let os = item.value().attr("value").unwrap_or_default().to_string();
            if os.eq_ignore_ascii_case("any") {
                continue;
            }
            obj.os.push(os);
        }
    }

    {
        let mut list = html
            .select(&select)
            .filter(|item| item.value().attr("id").unwrap_or_default() == "arch");

        let Some(item) = list.next() else {
            return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
        };

        let select1 = Selector::parse("option").unwrap();
        let node1 = item.select(&select1);

        for item in node1 {
            let arch = item.value().attr("value").unwrap_or_default().to_string();
            if arch.eq_ignore_ascii_case("any") {
                continue;
            }
            obj.arch.push(arch);
        }
    }

    let Some(temp) = data.pagepost_custom_js_value.find("function") else {
        return Err(ErrorType::DataNotFound(DataNotFoundData::Info));
    };

    let mut data = data.pagepost_custom_js_value[..temp].to_string();
    data.push_str(&mml_names::get_line_ending());
    data.push_str("JSON.stringify(sourceDataJson)");

    let rt = AsyncRuntime::new().map_err(|err| {
        ErrorType::TaskError(ErrorData {
            error: err.to_string(),
        })
    })?;
    let ctx = AsyncContext::full(&rt).await.map_err(|err| {
        ErrorType::TaskError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let result: String = ctx.with(|ctx| ctx.eval(data).unwrap()).await;
    
    let files = serialize_tools::json_from_str::<OpenJ9FileObj>(&result)?;

    for mut item in files.downloads {
        let names: Vec<&str> = item.name.split("<br>").collect();
        item.name = if names.len() == 3 {
            format!("{}+{}", names[1], names[2])
        } else {
            item.name
        };
        obj.download.push(item);
    }

    Ok(obj)
}
