//! 加载器版本列表
//!
//! 添加实例窗口的"加载器版本"下拉数据源：按加载器类型（可选按游戏版本）
//! 拉取可用版本号列表，全部来自 mcml-net 的对应 API / 镜像源。

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex, RwLock};

use regex::Regex;

use mcml_config::config_obj::SourceLocal;
use mcml_names::i18_items::error_type::{CoreResult, DataNotFoundData, ErrorType};

use crate::gui_hook::ProgressGui;
use crate::loader::LoaderType;

/// Forge 官方源全量版本缓存（maven-metadata.xml 较大，避免每次选择重复下载解析）
static FORGE_META_ALL: LazyLock<RwLock<Option<Vec<String>>>> = LazyLock::new(|| RwLock::new(None));
/// Forge BMCLAPI 源按游戏版本的列表缓存
static FORGE_BMCLAPI: LazyLock<Mutex<HashMap<String, Vec<String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
/// NeoForge 官方源全量版本缓存（版本号 → 所属游戏版本分组，避免重复下载解析）
static NEOFORGE_META_ALL: LazyLock<RwLock<Option<HashMap<String, Vec<String>>>>> =
    LazyLock::new(|| RwLock::new(None));
/// NeoForge BMCLAPI 源按游戏版本的列表缓存
static NEOFORGE_BMCLAPI: LazyLock<Mutex<HashMap<String, Vec<String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
/// Fabric meta 列表缓存（通用列表，不随游戏版本变化）
static FABRIC_LIST: LazyLock<RwLock<Option<Vec<String>>>> = LazyLock::new(|| RwLock::new(None));
/// Quilt meta 列表缓存（通用列表，不随游戏版本变化）
static QUILT_LIST: LazyLock<RwLock<Option<Vec<String>>>> = LazyLock::new(|| RwLock::new(None));

/// 获取加载器的可用版本列表（新版本在前）
///
/// - `loader`: 加载器类型
/// - `mc`: 游戏版本号（Forge 的 BMCLAPI 源、OptiFine / LiteLoader 按版本过滤需要）
pub async fn get_loader_versions(loader: &LoaderType, mc: &str) -> CoreResult<Vec<String>> {
    match loader {
        LoaderType::Forge => forge_versions(mc).await,
        LoaderType::NeoForge => neoforge_versions(mc).await,
        LoaderType::Fabric => fabric_versions().await,
        LoaderType::Quilt => quilt_versions().await,
        LoaderType::OptiFine => optifine_versions(mc).await,
        LoaderType::LiteLoader => liteloader_versions(mc).await,
        LoaderType::Normal | LoaderType::Custom => Err(ErrorType::DataNotFound(DataNotFoundData::Info)),
    }
}

/// 查询指定游戏版本支持的加载器 ID 列表（选中版本后调用）
///
/// 原版 / 自定义恒可用，其余按各加载器的支持数据判断：
/// - Forge：官方源为全量元数据；BMCLAPI 源按版本查询结果非空
/// - Fabric / Quilt：meta 通用，非空即支持
/// - NeoForge：官方源要求 1.20.1+；BMCLAPI 源按版本查询结果非空
/// - OptiFine：官方支持版本表（[`mcml_net::optifine_api::get_support_version`]）
/// - LiteLoader：meta 的版本键前缀匹配
///
/// 各加载器的数据源失败时跳过该项而非整体报错，保证网络异常时仍返回
/// 原版 + 自定义兜底。
///
/// 六种加载器的数据源并发查询；每查完一个加载器通过 `gui` 回调一次
/// （已完成步数，总步数 [`SUPPORT_LOAD_STEPS`]），进度按完成顺序递增。
pub async fn get_support_loaders(mc: &str, gui: ProgressGui) -> CoreResult<Vec<String>> {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let mut list = vec![LoaderType::Normal.id().to_string()];

    let done = AtomicUsize::new(0);

    // 单个加载器查询完成：步数 +1 并上报
    fn bump(done: &AtomicUsize, gui: &ProgressGui) {
        let step = done.fetch_add(1, Ordering::Relaxed) + 1;
        if let Some(gui) = gui {
            gui.set_progress_now(step, Some(SUPPORT_LOAD_STEPS));
        }
    }

    let forge = async {
        let ok = forge_versions(mc).await.map(|v| !v.is_empty()).unwrap_or(false);
        bump(&done, &gui);
        ok
    };
    let fabric = async {
        let ok = fabric_versions().await.map(|v| !v.is_empty()).unwrap_or(false);
        bump(&done, &gui);
        ok
    };
    let quilt = async {
        let ok = quilt_versions().await.map(|v| !v.is_empty()).unwrap_or(false);
        bump(&done, &gui);
        ok
    };
    let neoforge = async {
        let ok = if mcml_net::url_helper::get_source() == SourceLocal::Offical {
            version_ge(mc, "1.20.1")
        } else {
            neoforge_versions(mc).await.map(|v| !v.is_empty()).unwrap_or(false)
        };
        bump(&done, &gui);
        ok
    };
    let optifine = async {
        let ok = matches!(
            mcml_net::optifine_api::get_support_version().await,
            Ok(Some(set)) if set.contains(mc)
        );
        bump(&done, &gui);
        ok
    };
    let liteloader = async {
        let ok = liteloader_supports(mc).await;
        bump(&done, &gui);
        ok
    };

    let (forge_ok, fabric_ok, quilt_ok, neoforge_ok, optifine_ok, liteloader_ok) =
        tokio::join!(forge, fabric, quilt, neoforge, optifine, liteloader);

    if forge_ok {
        list.push(LoaderType::Forge.id().to_string());
    }
    if fabric_ok {
        list.push(LoaderType::Fabric.id().to_string());
    }
    if quilt_ok {
        list.push(LoaderType::Quilt.id().to_string());
    }
    if neoforge_ok {
        list.push(LoaderType::NeoForge.id().to_string());
    }
    if optifine_ok {
        list.push(LoaderType::OptiFine.id().to_string());
    }
    if liteloader_ok {
        list.push(LoaderType::LiteLoader.id().to_string());
    }

    list.push(LoaderType::Custom.id().to_string());
    Ok(list)
}

/// 支持列表查询总步数（Forge / Fabric / Quilt / NeoForge / OptiFine / LiteLoader）
pub const SUPPORT_LOAD_STEPS: usize = 6;

/// LiteLoader 是否支持该版本：meta 的 versions 键为游戏版本前缀（如 "1.12"）
async fn liteloader_supports(mc: &str) -> bool {
    match liteloader_versions(mc).await {
        Ok(list) => !list.is_empty(),
        Err(_) => false,
    }
}

/// 版本号比较：a >= b（按 `.` 分段数值比较）
fn version_ge(a: &str, b: &str) -> bool {
    fn parts(v: &str) -> Vec<u64> {
        v.split('.')
            .map(|n| n.parse::<u64>().unwrap_or(0))
            .collect()
    }
    let (pa, pb) = (parts(a), parts(b));
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or(0);
        let y = pb.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    true
}

/// Forge：官方源为 maven-metadata.xml（全量，按 `{mc}-` 前缀筛选并去前缀），
/// BMCLAPI 源为按游戏版本的 JSON 数组（直接是构建号）；两者都取新版本在前
async fn forge_versions(mc: &str) -> CoreResult<Vec<String>> {
    if mcml_net::url_helper::get_source() == SourceLocal::Offical {
        // 命中缓存：只做前缀过滤
        if let Some(all) = FORGE_META_ALL.read().unwrap().as_ref() {
            return Ok(forge_of_mc(all, mc));
        }

        let url = mcml_net::url_helper::get_forge_versions(mc);
        let data = mcml_net::get_work_client().get_text(&url).await?;
        let re = Regex::new(r"<version>([^<]+)</version>").unwrap();
        let mut all: Vec<String> = re
            .captures_iter(&data)
            .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
            .collect();
        all.reverse();
        *FORGE_META_ALL.write().unwrap() = Some(all.clone());
        Ok(forge_of_mc(&all, mc))
    } else {
        if let Some(hit) = FORGE_BMCLAPI.lock().unwrap().get(mc) {
            return Ok(hit.clone());
        }

        let url = mcml_net::url_helper::get_forge_versions(mc);
        let data = mcml_net::get_work_client().get_text(&url).await?;
        let arr = serde_json::from_str::<serde_json::Value>(&data)
            .map_err(|_| ErrorType::DataNotFound(DataNotFoundData::Info))?;
        let mut list = value_versions(&arr);
        list.reverse();
        FORGE_BMCLAPI.lock().unwrap().insert(mc.to_string(), list.clone());
        Ok(list)
    }
}

/// 从 Forge 全量版本列表中筛出指定游戏版本的构建号
///
/// 元数据条目形如 `1.20.1-47.1.0`（旧版本可能带分支后缀），去掉 `{mc}-` 前缀。
fn forge_of_mc(all: &[String], mc: &str) -> Vec<String> {
    let prefix = format!("{mc}-");
    all.iter()
        .filter(|v| v.starts_with(&prefix))
        .map(|v| v[prefix.len()..].to_string())
        .collect()
}

/// 解析 NeoForge 官方源全量列表：把每个版本号归入对应的游戏版本分组
///
/// 数据为 JSON `{"versions": [...]}`，全量按游戏版本分组缓存，返回指定游戏版本的构建号。
async fn parse_neoforge_offical(mc: &str, data: &str) -> CoreResult<Vec<String>> {
    // 命中缓存：不再重复解析
    if let Some(all) = NEOFORGE_META_ALL.read().unwrap().as_ref() {
        return Ok(all.get(mc).cloned().unwrap_or_default());
    }

    let obj = serde_json::from_str::<serde_json::Value>(data)
        .map_err(|_| ErrorType::DataNotFound(DataNotFoundData::Info))?;
    let versions = obj
        .get("versions")
        .ok_or(ErrorType::DataNotFound(DataNotFoundData::Info))?;

    let mut map: HashMap<String, Vec<String>> = HashMap::new();
    for version in value_versions(versions) {
        if let Some(mcver) = neoforge_mc_version(&version) {
            map.entry(mcver).or_default().push(version);
        }
    }
    // 官方 API 为旧版本在前，倒转为新版本在前
    for list in map.values_mut() {
        list.reverse();
    }

    let list = map.get(mc).cloned().unwrap_or_default();
    *NEOFORGE_META_ALL.write().unwrap() = Some(map);
    Ok(list)
}

/// 从 NeoForge 版本号推断所属的游戏版本
///
/// - 旧方案：`21.1.77` → `1.21.1`（`1.` + 前两段）
/// - 新方案（主版本 ≥ 26）：`26.2.0.0-beta` → `26.2`，`26.1.1.x` → `26.1.1`，
///   带 `+snapshot-x` 的映射到对应快照版本
/// - 以 `0` 开头的愚人节版本返回 `None`（如 `0.25w14craftmine.3-beta`）
fn neoforge_mc_version(version: &str) -> Option<String> {
    if version.starts_with('0') {
        return None;
    }

    let mut spl = version.split('.');
    let major: u64 = spl.next()?.parse().ok()?;
    let second = spl.next()?;

    if major >= 26 {
        let mut mc = format!("{major}.{second}");
        // 26.1.0.x → 26.1；26.1.1.x → 26.1.1
        let third = spl.next().unwrap_or("0");
        if third != "0" {
            mc.push('.');
            mc.push_str(third);
        }
        // 26.1.0.0-alpha+snapshot-1 → 26.1-snapshot-1
        if let Some((_, suffix)) = version.split_once('+') {
            mc.push('-');
            mc.push_str(suffix);
        }
        Some(mc)
    } else {
        Some(format!("1.{major}.{second}"))
    }
}

/// NeoForge：按配置源拉取版本列表
///
/// 配置为官方源时只走官方源（全量列表按游戏版本过滤）；
/// 配置为 BMCLAPI 镜像时镜像按游戏版本返回，镜像不可达时回退官方源。
async fn neoforge_versions(mc: &str) -> CoreResult<Vec<String>> {
    let client = mcml_net::get_work_client();
    let official = format!(
        "{}api/maven/versions/releases/net%2Fneoforged%2Fneoforge",
        mcml_net::urls::NEOFORGE
    );

    let (data, from_offical) = if mcml_net::url_helper::get_source() == SourceLocal::Offical {
        (client.get_text(&official).await?, true)
    } else {
        // BMCLAPI 源按游戏版本返回：命中缓存直接用
        if let Some(hit) = NEOFORGE_BMCLAPI.lock().unwrap().get(mc) {
            return Ok(hit.clone());
        }
        match client
            .get_text(&mcml_net::url_helper::get_neoforge_meta(mc))
            .await
        {
            Ok(data) => (data, false),
            // BMCLAPI 镜像不可达：回退官方源
            Err(e) => (client.get_text(&official).await.map_err(|_| e)?, true),
        }
    };

    if from_offical {
        parse_neoforge_offical(mc, &data).await
    } else {
        let obj = serde_json::from_str::<serde_json::Value>(&data)
            .map_err(|_| ErrorType::DataNotFound(DataNotFoundData::Info))?;
        let mut list = value_versions(&obj);
        list.reverse();
        NEOFORGE_BMCLAPI.lock().unwrap().insert(mc.to_string(), list.clone());
        Ok(list)
    }
}

/// Fabric：meta 的 loader 数组（stable 优先）
async fn fabric_versions() -> CoreResult<Vec<String>> {
    // 命中缓存：meta 是通用列表，直接返回
    if let Some(list) = FABRIC_LIST.read().unwrap().as_ref() {
        return Ok(list.clone());
    }

    let meta = mcml_net::fabric_api::get_meta().await?;
    let obj: crate::loader::fabric_meta_obj::FabricMetaObj =
        mcml_base::serialize_tools::json_from_bytes(&meta)?;

    let mut list: Vec<String> = obj
        .loader
        .iter()
        .filter(|item| item.stable)
        .map(|item| item.version.clone())
        .collect();
    for item in &obj.loader {
        if !item.stable && !list.contains(&item.version) {
            list.push(item.version.clone());
        }
    }
    *FABRIC_LIST.write().unwrap() = Some(list.clone());
    Ok(list)
}

/// Quilt：meta 的 loader 数组（meta 从旧到新，倒转为新在前）
async fn quilt_versions() -> CoreResult<Vec<String>> {
    // 命中缓存：meta 是通用列表，直接返回
    if let Some(list) = QUILT_LIST.read().unwrap().as_ref() {
        return Ok(list.clone());
    }

    let meta = mcml_net::quilt_api::get_meta().await?;
    let obj: crate::loader::quilt_meta_obj::QuiltMetaObj =
        mcml_base::serialize_tools::json_from_bytes(&meta)?;

    let mut list: Vec<String> = obj.loader.iter().map(|item| item.version.clone()).collect();
    list.reverse();
    *QUILT_LIST.write().unwrap() = Some(list.clone());
    Ok(list)
}

/// OptiFine：按游戏版本过滤（官方源版本号形如 `HD_U_I6`）
async fn optifine_versions(mc: &str) -> CoreResult<Vec<String>> {
    let list = mcml_net::optifine_api::get_optifine_version().await?;
    Ok(list
        .iter()
        .filter(|item| item.mc_version == mc)
        .map(|item| item.version.clone())
        .collect())
}

/// LiteLoader：versions.json 形如 `{"versions": {"1.12": {"1.12.2-SNAPSHOT": {...}}}}`，
/// 按游戏版本前缀取内层版本名
async fn liteloader_versions(mc: &str) -> CoreResult<Vec<String>> {
    let meta = mcml_net::liteloader_api::get_meta().await?;
    let obj: serde_json::Value = serde_json::from_slice(&meta)
        .map_err(|_| ErrorType::DataNotFound(DataNotFoundData::Info))?;

    let versions = obj
        .get("versions")
        .and_then(|v| v.as_object())
        .ok_or(ErrorType::DataNotFound(DataNotFoundData::Info))?;

    let mut list = Vec::new();
    for (mcver, inner) in versions {
        if !mc.starts_with(mcver.as_str()) {
            continue;
        }
        if let Some(inner) = inner.as_object() {
            list.extend(inner.keys().cloned());
        }
    }
    Ok(list)
}

/// 从 JSON 值提取版本号列表：元素为对象取 `version` 字段，为字符串直接取
fn value_versions(value: &serde_json::Value) -> Vec<String> {
    let Some(arr) = value.as_array() else {
        return Vec::new();
    };
    arr.iter()
        .filter_map(|item| match item {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Object(map) => map.get("version").and_then(|v| v.as_str()).map(String::from),
            _ => None,
        })
        .collect()
}

