use mcml_base::file_item::FileHash;
use mcml_config::config_obj::SourceLocal;
use mcml_names::names;

use crate::{url_helper, urls};

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

pub struct UrlHashObj {
    pub hash: FileHash,
    pub url: String,
}

/// 测试这个jar文件是否能从网上下载
/// - `dir`: jar文件路径
pub async fn test_hash(dir: &str) -> Option<UrlHashObj> {
    let url = match url_helper::get_source() {
        SourceLocal::Offical => urls::MAVEN,
        SourceLocal::Bmclapi => urls::MAVEN_ALIYUN,
    };

    let url1 = String::from(url) + dir;

    let hash = try_get_hash(&url1).await;
    if matches!(hash, FileHash::None) {
        None
    } else {
        Some(UrlHashObj { hash, url: url1 })
    }
}

/// 尝试获取校验值
pub async fn try_get_hash(url: &str) -> FileHash {
    let sha1_url = url.to_string() + names::SHA1_DOT_EXT;
    let sha256_url = url.to_string() + names::SHA256_DOT_EXT;
    let sha512_url = url.to_string() + names::SHA512_DOT_EXT;

    let client = crate::get_work_client();

    if let Ok(data) = client.get_text(&sha256_url).await {
        FileHash::Sha256(data)
    } else if let Ok(data) = client.get_text(&sha512_url).await {
        FileHash::Sha512(data)
    } else if let Ok(data) = client.get_text(&sha1_url).await {
        FileHash::Sha1(data)
    } else {
        FileHash::None
    }
}
