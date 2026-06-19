use mcml_config::config_obj::SourceLocal;
use mcml_names::{names, urls};

use crate::{WORK_CLIENT, url_helper};

/// 将一个 Maven 坐标库名转换为文件路径
/// 例如 "com.example:artifact:1.0" -> "com/example/artifact/1.0/artifact-1.0.jar"
/// 例如 "com.example:artifact:1.0:ext" -> "com/example/artifact/1.0/artifact-1.0-ext.jar"
pub fn version_name_to_path(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    if parts.len() < 3 {
        format!("{}.jar", name.replace('.', "/"))
    } else if parts.len() > 3 {
        format!(
            "{}/{}/{}/{}-{}-{}.jar",
            parts[0].replace('.', "/"),
            parts[1],
            parts[2],
            parts[1],
            parts[2],
            parts[3]
        )
    } else {
        format!(
            "{}/{}/{}/{}-{}.jar",
            parts[0].replace('.', "/"),
            parts[1],
            parts[2],
            parts[1],
            parts[2]
        )
    }
}

pub struct UrlSha1Obj {
    pub sha1: String,
    pub url: String,
}

/// 测试这个jar文件是否能从网上下载
/// - `dir`: jar文件路径
pub async fn test_sha1(dir: &str) -> Option<UrlSha1Obj> {
    let url = match url_helper::get_source() {
        SourceLocal::Offical => urls::MAVEN,
        SourceLocal::Bmclapi => urls::MAVEN_ALIYUN,
    };

    let url1 = String::from(url) + dir;
    let url2 = url1.clone() + names::SHA1_EXT;

    let res = WORK_CLIENT.get().unwrap().get_text(&url2).await;

    if let Ok(data) = res {
        Some(UrlSha1Obj {
            sha1: data,
            url: url1,
        })
    } else {
        None
    }
}
