use mcml_config::config_obj::SourceLocal;
use mcml_names::urls::{self};

/// 游戏版本
pub fn game_version(source: Option<SourceLocal>) -> String {
    let source = match source {
        None => {
            let config = mcml_config::CONFIG.get().unwrap().read().unwrap();
            config.http.source
        }
        Some(data) => data,
    };

    String::from(match source {
        SourceLocal::Bmclapi => urls::BMCLAPI,
        SourceLocal::Offical => urls::MOJANG_META,
    }) + "mc/game/version_manifest_v2.json"
}

/// 下载地址转换
/// - `url`: 原始下载地址
pub fn change_source(url: &mut String) {
    let config = mcml_config::CONFIG.get().unwrap().read().unwrap();
    let source = config.http.source;

    if source != SourceLocal::Bmclapi {
        return;
    }

    for item in urls::MOJANG {
        *url = url.replace(item, urls::BMCLAPI);
    }
}

pub fn get_fabric_meta() -> String {
    let config = mcml_config::CONFIG.get().unwrap().read().unwrap();
    let source = config.http.source;

    match source {
        SourceLocal::Offical => String::from(urls::FABRIC_META) + "v2/versions",
        SourceLocal::Bmclapi => String::from(urls::BMCLAPI) + "fabric-meta/v2/versions",
    }
}
