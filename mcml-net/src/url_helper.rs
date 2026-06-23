use mcml_config::config_obj::SourceLocal;

use crate::{
    authlib_api::{ArtifactsObj, AuthlibInjectorObj},
    optifine_api::OptifineListObj,
    urls,
};

/// 获取下载源
pub fn get_source() -> SourceLocal {
    let config = mcml_config::read_config();

    config.http.source
}

/// 游戏版本
pub fn game_version(source: Option<SourceLocal>) -> String {
    let source = match source {
        None => get_source(),
        Some(data) => data,
    };

    format!(
        "{}mc/game/version_manifest_v2.json",
        match source {
            SourceLocal::Bmclapi => urls::BMCLAPI,
            SourceLocal::Offical => urls::MOJANG_META,
        }
    )
}

/// 获取minecraft资源下载地址
pub fn get_download_assets(hash: &str) -> String {
    let prefix: String = hash.chars().take(2).collect();

    match get_source() {
        SourceLocal::Offical => format!("{}{prefix}/{hash}", urls::MINECRAFT_RESOURCES),
        SourceLocal::Bmclapi => format!("{}assets/{prefix}/{hash}", urls::BMCLAPI),
    }
}

/// 获取其他下载源的Minecraft下载地址
/// - `version`: 游戏版本
pub fn get_minecraft_client(url: &str, version: &str) -> String {
    if get_source() == SourceLocal::Bmclapi {
        format!("{}version/{}/client", urls::BMCLAPI, version)
    } else {
        String::from(url)
    }
}

/// 下载地址转换
/// - `url`: 原始下载地址
pub fn change_source(url: &mut String) {
    if get_source() != SourceLocal::Bmclapi {
        return;
    }

    for item in urls::MOJANG {
        *url = url.replace(item, urls::BMCLAPI);
    }
}

/// 获取forge版本信息获取网址
/// - `version`: 游戏版本
pub fn get_forge_versions(version: &str) -> String {
    match get_source() {
        SourceLocal::Offical => {
            format!("{}net/minecraftforge/forge/maven-metadata.xml", urls::FORGE)
        }
        SourceLocal::Bmclapi => format!("{}forge/minecraft/{version}", urls::BMCLAPI),
    }
}

/// 获取neoforge信息获取网址
pub fn get_neoforge_meta(version: &str) -> String {
    match get_source() {
        SourceLocal::Offical => {
            format!(
                "{}api/maven/versions/releases/net%2Fneoforged%2Fneoforge",
                urls::NEOFORGE
            )
        }
        SourceLocal::Bmclapi => format!("{}neoforge/list/{version}", urls::BMCLAPI),
    }
}

/// 获取fabric信息获取网址
pub fn get_fabric_meta() -> String {
    match get_source() {
        SourceLocal::Offical => format!("{}v2/versions", urls::FABRIC_META),
        SourceLocal::Bmclapi => format!("{}fabric-meta/v2/versions", urls::BMCLAPI),
    }
}

/// 获取quilt信息获取网址
pub fn get_quilt_meta() -> String {
    // match get_source() {
    //     SourceLocal::Offical => format!("{}v2/versions", urls::FABRIC_META),
    //     SourceLocal::Bmclapi => format!("{}fabric-meta/v2/versions", urls::BMCLAPI),
    // }

    format!("{}v3/versions", urls::QUILT_META)
}

/// 获取外置登录信息地址
pub fn get_authlib_injector_meta() -> String {
    match get_source() {
        SourceLocal::Offical => format!("{}artifacts.json", urls::AUTHLIB),
        SourceLocal::Bmclapi => format!("{}mirrors/authlib-injector/artifacts.json", urls::BMCLAPI),
    }
}

/// 获取高清修复信息地址
pub fn get_optifine_meta() -> String {
    match get_source() {
        SourceLocal::Offical => format!("{}downloads", urls::OPTIFINE),
        SourceLocal::Bmclapi => format!("{}optifine/versionList", urls::BMCLAPI),
    }
}

/// 获取Forge下载地址
/// - `mc`: 游戏版本
/// - `version`: forge版本
pub fn get_forge_jar(mc: &str, version: &str) -> String {
    match get_source() {
        SourceLocal::Offical => format!("{}net/minecraftforge/forge/{mc}-{version}/", urls::FORGE),
        SourceLocal::Bmclapi => format!(
            "{}maven/net/minecraftforge/forge/{mc}-{version}/",
            urls::BMCLAPI
        ),
    }
}

/// 获取NeoForge下载地址
/// - `v2222`: 是否为1.20.2以上版本
/// - `mc`: 游戏版本
/// - `version`: forge版本
pub fn get_neoforge_jar(v2222: bool, mc: &str, version: &str) -> String {
    let url = if v2222 {
        format!("neoforge/{version}/")
    } else {
        format!("forge/{mc}-{version}/")
    };

    match get_source() {
        SourceLocal::Offical => format!("{}releases/net/neoforged/{url}/", urls::NEOFORGE),
        SourceLocal::Bmclapi => format!("{}maven/net/neoforged/{url}/", urls::BMCLAPI),
    }
}

/// 外置登录地址
/// - `obj`: 登陆地址
pub fn get_authlib_injector(obj: &ArtifactsObj) -> String {
    match get_source() {
        SourceLocal::Offical => format!("{}artifact/{}.json", urls::AUTHLIB, obj.build_number),
        SourceLocal::Bmclapi => format!(
            "{}mirrors/authlib-injector/artifact/{}.json",
            urls::BMCLAPI,
            obj.build_number
        ),
    }
}

/// 外置登录地址
/// - `obj`: 登陆地址
pub fn get_authlib_injector_jar(obj: &AuthlibInjectorObj) -> String {
    match get_source() {
        SourceLocal::Offical => format!(
            "{}artifact/{}/authlib-injector-{}.jar",
            urls::AUTHLIB,
            obj.build_number,
            obj.version
        ),
        SourceLocal::Bmclapi => format!(
            "{}mirrors/authlib-injector/artifact/{}/authlib-injector-{}.jar",
            urls::BMCLAPI,
            obj.build_number,
            obj.version
        ),
    }
}

/// 获取高清修复下载地址
/// - `obj`: 高清修复信息
pub fn get_optifine_jar(obj: &OptifineListObj) -> String {
    format!(
        "{}optifine/{}/{}/{}",
        urls::BMCLAPI,
        obj.mcversion,
        obj.rtype,
        obj.patch
    )
}

/// 替换运行库下载地址
/// - `url`: 运行库地址
pub fn replace_minecraft_libraries(url: &str) -> String {
    match get_source() {
        SourceLocal::Offical => String::from(url),
        SourceLocal::Bmclapi => url.replace(
            urls::MINECRAFT_LIBRARIES,
            &format!("{}maven/", urls::BMCLAPI),
        ),
    }
}

/// 替换运行库下载地址
/// - `url`: 运行库地址
pub fn replace_forge_libraries(url: &str) -> String {
    match get_source() {
        SourceLocal::Offical => String::from(url),
        SourceLocal::Bmclapi => url.replace(urls::FORGE, &format!("{}maven/", urls::BMCLAPI)),
    }
}

/// 替换运行库下载地址
/// - `url`: 运行库地址
pub fn replace_neoforge_libraries(url: &str) -> String {
    match get_source() {
        SourceLocal::Offical => String::from(url),
        SourceLocal::Bmclapi => url.replace(urls::FORGE, &format!("{}maven/", urls::BMCLAPI)),
    }
}

/// 替换fabric库下载地址
pub fn replace_fabric_libraries(url: &str) -> String {
    match get_source() {
        SourceLocal::Offical => String::from(url),
        SourceLocal::Bmclapi => url.replace(urls::FABRIC, &format!("{}maven/", urls::BMCLAPI)),
    }
}

/// 修正Forge下载地址
/// - `url`:
pub fn forge_url_fix(version: &str) -> String {
    if version.eq_ignore_ascii_case("1.7.2") {
        String::from("-mc172")
    } else if version.eq_ignore_ascii_case("1.7.10") {
        String::from("-1.7.10")
    } else if version.eq_ignore_ascii_case("1.8.9") {
        String::from("-1.8.9")
    } else if version.eq_ignore_ascii_case("1.9") {
        String::from("-1.9.0")
    } else if version.eq_ignore_ascii_case("1.9.4") {
        String::from("-1.9.4")
    } else if version.eq_ignore_ascii_case("1.10") {
        String::from("-1.10.0")
    } else {
        String::from(version)
    }
}
