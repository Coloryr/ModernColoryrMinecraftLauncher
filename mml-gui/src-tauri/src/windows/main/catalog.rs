//! 主窗口的"目录类"数据：版本清单缓存与 MOTD 地址解析
//!
//! 从 `main.rs` 拆出来的纯逻辑（不碰窗口、不碰模型），连同它自己的单测。

use std::sync::{LazyLock, RwLock};

use crate::dtos::VersionInfoDto;

/// 版本列表缓存（进程级：主窗口 / 添加实例窗口共用，不挂在窗口模型上）
pub(super) static VERSIONS_CACHE: LazyLock<RwLock<Vec<VersionInfoDto>>> =
    LazyLock::new(|| RwLock::new(Vec::new()));

/// 从 mml-core 拉取版本清单（按配置源：官方 / BMCLAPI），
/// 按类型分组排序（正式版 > 快照 > 旧版 Beta > 旧版 Alpha），
/// 组内保持清单顺序（清单本身按新旧排列）；失败返回空
pub(super) async fn fetch_versions() -> Vec<VersionInfoDto> {
    #[derive(serde::Deserialize)]
    struct Manifest {
        versions: Vec<ManifestVersion>,
    }
    #[derive(serde::Deserialize)]
    struct ManifestVersion {
        id: String,
        #[serde(rename = "type")]
        version_type: String,
    }
    let Ok(bytes) = mml_net::mojang_api::get_versions(None).await else {
        return Vec::new();
    };
    let Ok(manifest) = serde_json::from_slice::<Manifest>(&bytes) else {
        return Vec::new();
    };
    let mut list: Vec<VersionInfoDto> = manifest
        .versions
        .into_iter()
        .map(|v| VersionInfoDto {
            id: v.id,
            version_type: v.version_type,
        })
        .collect();
    // 只按类型分组排序，组内不动：清单本身即最新在前，
    // 按版本号数值重排会把快照（25w14a）排到 1.21.x 之上、打乱 rc / 旧版顺序
    list.sort_by_key(|v| match v.version_type.as_str() {
        "release" => 0,
        "snapshot" => 1,
        "old_beta" => 2,
        "old_alpha" => 3,
        _ => 4,
    });
    // 不截断：快照 / 旧版类型也要有数据，否则切换版本类型后过滤结果为空
    list
}

/// MOTD 展示的默认端口
const MOTD_DEFAULT_PORT: u16 = 25565;

/// 解析服务器地址：host / host:port / [IPv6]:port / 裸 IPv6
pub(super) fn parse_motd_addr(address: &str) -> (String, u16) {
    let addr = address.trim();
    if addr.is_empty() {
        return (String::new(), MOTD_DEFAULT_PORT);
    }
    // [IPv6]:port
    if let Some(rest) = addr.strip_prefix('[')
        && let Some((host, port)) = rest.split_once(']')
    {
        let port = port
            .strip_prefix(':')
            .and_then(|p| p.parse().ok())
            .unwrap_or(MOTD_DEFAULT_PORT);
        return (host.to_string(), port);
    }
    // host:port（只有一个冒号才算 host:port，多个冒号视为裸 IPv6）
    if let Some((host, port)) = addr.rsplit_once(':')
        && !host.contains(':')
        && !port.is_empty()
        && let Ok(p) = port.parse::<u16>()
    {
        return (host.to_string(), p);
    }
    (addr.to_string(), MOTD_DEFAULT_PORT)
}

#[cfg(test)]
mod tests {
    use super::parse_motd_addr;

    #[test]
    fn test_parse_motd_addr() {
        assert_eq!(
            parse_motd_addr("mc.example.com"),
            ("mc.example.com".into(), 25565)
        );
        assert_eq!(
            parse_motd_addr("mc.example.com:25566"),
            ("mc.example.com".into(), 25566)
        );
        assert_eq!(
            parse_motd_addr("  mc.example.com:25566 "),
            ("mc.example.com".into(), 25566)
        );
        assert_eq!(parse_motd_addr("[::1]:25565"), ("::1".into(), 25565));
        assert_eq!(parse_motd_addr("::1"), ("::1".into(), 25565));
        assert_eq!(parse_motd_addr(""), (String::new(), 25565));
    }
}
