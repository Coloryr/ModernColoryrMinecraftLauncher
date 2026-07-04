use std::{
    collections::HashMap,
    io::{Cursor, Read, Seek},
    path::{Path, PathBuf},
    sync::Mutex,
};

use mcml_base::{
    file_item::FileHash,
    hash_helper::{self, HashType},
    path_helper,
};
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData},
    names,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use toml::Table;
use zip::ZipArchive;

use crate::{launcher::instance_setting_obj::InstanceSettingObj, loader::LoaderType};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoadSideType {
    Unknown,
    Client,
    Server,
    Both,
}

impl Default for LoadSideType {
    fn default() -> Self {
        LoadSideType::Unknown
    }
}

pub enum DependantType {
    Required(String),
    Recommend(String),
}

pub struct ModItemObj {
    /// ID
    pub mod_id: String,
    /// 名字
    pub name: String,
    /// 描述
    pub description: Option<String>,
    /// 版本号
    pub version: Option<String>,
    /// 作者
    pub author: Vec<String>,
    /// 依赖
    pub dependants: Vec<DependantType>,
    /// 网站
    pub url: Option<String>,
    /// 图标
    pub icon: Option<Vec<u8>>,
    /// 支持的加载器
    pub loaders: LoaderType,
    /// 加载测
    pub side: LoadSideType,
}

impl Default for ModItemObj {
    fn default() -> Self {
        Self {
            mod_id: Default::default(),
            name: Default::default(),
            description: Default::default(),
            version: Default::default(),
            author: Default::default(),
            dependants: Default::default(),
            loaders: Default::default(),
            side: Default::default(),
            url: Default::default(),
            icon: Default::default(),
        }
    }
}

/// 模组信息
pub struct ModObj {
    pub info: Vec<ModItemObj>,
    /// 是否被禁用
    pub disable: bool,
    /// 是否为Core模组
    pub core: bool,
    /// 校验
    pub hash: FileHash,
    /// 内置的模组
    pub jar_in_jar: Vec<ModObj>,
    /// 是否读取失败
    pub fail: bool,
    /// 文件路径
    pub file: PathBuf,
}

impl Default for ModObj {
    fn default() -> Self {
        Self {
            info: Default::default(),
            disable: Default::default(),
            core: Default::default(),
            hash: Default::default(),
            jar_in_jar: Default::default(),
            fail: Default::default(),
            file: Default::default(),
        }
    }
}

fn extract_strings_from_json(
    map: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Vec<String> {
    map.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn extract_dependants_from_json(
    map: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    required: bool,
) -> Vec<DependantType> {
    map.get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| {
                    v.as_str().map(|item| {
                        if required {
                            DependantType::Required(item.to_string())
                        } else {
                            DependantType::Recommend(item.to_string())
                        }
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn get_opt_string_from_json(
    map: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    map.get(key)
        .and_then(|v| Some(v.as_str().unwrap_or("").to_string()))
}

fn get_string_from_json(map: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    map.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

/// 字符串引号状态
#[derive(PartialEq)]
enum Quote {
    None,
    Double,
    Single,
}

/// 容错处理 mcmod.info 中常见的非法 JSON：
/// - 单引号字符串：`'value'` → `"value"`
/// - 数组内未加引号的标识符：`[mod_minecraftForge]` → `["mod_minecraftForge"]`
/// - 字符串内未转义的控制字符：换行符 → `\n`，回车符 → `\r`，制表符 → `\t`
fn sanitize_mcmod_json(json: &str) -> String {
    let mut result = String::with_capacity(json.len() + 64);
    let chars: Vec<char> = json.chars().collect();
    let mut i = 0;
    let mut quote = Quote::None;
    let mut escape = false;

    while i < chars.len() {
        let ch = chars[i];

        // 处理转义模式
        if escape {
            escape = false;
            match quote {
                Quote::Single => match ch {
                    '\'' => result.push('\''),       // \' → '
                    '"' => result.push_str("\\\""),  // \" → \"
                    '\\' => result.push_str("\\\\"), // \\ → \\
                    'n' => result.push_str("\\n"),
                    'r' => result.push_str("\\r"),
                    't' => result.push_str("\\t"),
                    '/' => result.push_str("\\/"),
                    other => {
                        result.push_str("\\\\");
                        result.push(other);
                    }
                },
                Quote::Double | Quote::None => {
                    result.push(ch);
                }
            }
            i += 1;
            continue;
        }

        // 反斜杠进入转义模式
        if ch == '\\' {
            match quote {
                Quote::Double => {
                    escape = true;
                    result.push(ch); // 双引号内的转义已合法，直接保留
                }
                Quote::Single => {
                    escape = true; // 单引号内需要转换，不先推入 \
                }
                Quote::None => {
                    result.push(ch);
                }
            }
            i += 1;
            continue;
        }

        // 双引号
        if ch == '"' {
            match quote {
                Quote::Double => {
                    quote = Quote::None;
                    result.push(ch);
                }
                Quote::Single => {
                    // 单引号字符串内的双引号 → 转义
                    result.push_str("\\\"");
                }
                Quote::None => {
                    quote = Quote::Double;
                    result.push(ch);
                }
            }
            i += 1;
            continue;
        }

        // 单引号 → 统一转换为双引号
        if ch == '\'' {
            match quote {
                Quote::Single => {
                    quote = Quote::None;
                    result.push('"');
                }
                Quote::Double => {
                    result.push(ch); // 双引号内的单引号是普通字符
                }
                Quote::None => {
                    quote = Quote::Single;
                    result.push('"');
                }
            }
            i += 1;
            continue;
        }

        // 字符串内未转义的控制字符
        if quote != Quote::None {
            match ch {
                '\n' => result.push_str("\\n"),
                '\r' => result.push_str("\\r"),
                '\t' => result.push_str("\\t"),
                other => result.push(other),
            }
            i += 1;
            continue;
        }

        // 仅在字符串外部处理数组内裸标识符
        if ch == '[' || ch == ',' {
            result.push(ch);
            i += 1;

            // 跳过空白
            while i < chars.len() && chars[i].is_whitespace() {
                result.push(chars[i]);
                i += 1;
            }

            // 检测裸标识符（以字母或下划线开头）
            if i < chars.len() && (chars[i].is_alphabetic() || chars[i] == '_') {
                let start = i;
                while i < chars.len()
                    && (chars[i].is_alphanumeric() || chars[i] == '_' || chars[i] == '.')
                {
                    i += 1;
                }
                let word: String = chars[start..i].iter().collect();

                // 不引用 JSON 字面量
                if matches!(word.as_str(), "true" | "false" | "null") || word.parse::<f64>().is_ok()
                {
                    result.push_str(&word);
                } else {
                    result.push('"');
                    result.push_str(&word);
                    result.push('"');
                }
            }

            continue;
        }

        result.push(ch);
        i += 1;
    }

    result
}

fn read_forge_json(mut reader: impl Read, mod_info: &mut ModObj) -> CoreResult<()> {
    let mut json = String::new();
    reader.read_to_string(&mut json).map_err(|err| {
        ErrorType::ArchiveReadError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let obj = match serde_json::from_str::<serde_json::Value>(&json) {
        Ok(obj) => obj,
        Err(_) => {
            // 尝试容错处理：修复数组内未加引号的标识符
            let sanitized = sanitize_mcmod_json(&json);
            serde_json::from_str::<serde_json::Value>(&sanitized).map_err(|err| {
                ErrorType::SerializerError(ErrorData {
                    error: err.to_string(),
                })
            })?
        }
    };

    let values = match obj {
        serde_json::Value::Array(values) => Some(values),
        serde_json::Value::Object(map) => map.get("modList").and_then(|v| v.as_array()).cloned(),
        _ => None,
    };

    if let Some(values) = values {
        mod_info.info.extend(values.iter().map(|v| {
            let mut info = ModItemObj::default();

            if let serde_json::Value::Object(map) = v {
                info.mod_id = get_string_from_json(map, "modid");
                info.name = get_opt_string_from_json(map, "name").unwrap_or(info.mod_id.clone());
                info.description = get_opt_string_from_json(map, "description");
                info.version = get_opt_string_from_json(map, "version");
                info.url = get_opt_string_from_json(map, "url");
                info.loaders = LoaderType::Forge;

                info.author = extract_strings_from_json(map, "authorList");
                info.dependants
                    .extend(extract_dependants_from_json(map, "dependants", false));
                info.dependants
                    .extend(extract_dependants_from_json(map, "dependencies", false));
                info.dependants
                    .extend(extract_dependants_from_json(map, "requiredMods", true));
            }

            info
        }));
    }

    Ok(())
}

fn read_forge_toml(
    mut reader: impl Read,
    loader: LoaderType,
    mod_info: &mut ModObj,
) -> CoreResult<()> {
    let mut toml = String::new();
    reader.read_to_string(&mut toml).map_err(|err| {
        ErrorType::ArchiveReadError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let obj = toml.parse::<Table>().map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?;

    // 读取 mods
    if let Some(toml::Value::Array(values)) = obj.get("mods") {
        for item in values.iter() {
            let mut info = ModItemObj::default();
            let get_str = |key: &str| item.get(key).and_then(|v| v.as_str());

            info.mod_id = get_str("modId").unwrap_or("").to_string();
            info.name = get_str("displayName").unwrap_or(&info.mod_id).to_string();
            info.description = get_str("description").map(String::from);
            info.version = get_str("version").map(String::from);
            info.url = get_str("displayURL").map(String::from);
            info.loaders = loader;

            let authors = |key: &str| -> Vec<String> {
                item.get(key)
                    .and_then(|v| v.as_str())
                    .map(|s| s.split(',').map(String::from).collect())
                    .unwrap_or_default()
            };

            info.author.extend(authors("authorList"));
            info.author.extend(authors("authors"));

            mod_info.info.push(info);
        }
    }

    // 处理依赖关系
    if let Some(toml::Value::Table(dep_table)) = obj.get("dependencies") {
        for (key, value) in dep_table.iter() {
            let toml::Value::Table(map) = value else {
                continue;
            };

            let key_str = key.to_string();
            let Some(mod_item) = mod_info
                .info
                .iter_mut()
                .find(|item| item.mod_id.eq_ignore_ascii_case(&key_str))
            else {
                continue;
            };

            let get_str = |field: &str| map.get(field).and_then(|v| v.as_str());
            let get_bool = |field: &str| {
                map.get(field)
                    .and_then(|v| v.as_bool())
                    .or_else(|| {
                        map.get(field)
                            .and_then(|v| v.as_str())
                            .map(|s| s.eq_ignore_ascii_case("true"))
                    })
                    .unwrap_or(false)
            };

            if let Some(modid) = get_str("modid") {
                if modid.eq_ignore_ascii_case("minecraft") {
                    if let Some(side) = get_str("side") {
                        mod_item.side = match side.to_ascii_lowercase().as_str() {
                            "both" => LoadSideType::Both,
                            "client" => LoadSideType::Client,
                            "server" => LoadSideType::Server,
                            _ => mod_item.side,
                        };
                    }
                } else {
                    let is_mandatory = get_bool("mandatory");
                    let is_required = get_str("type")
                        .map(|s| s.eq_ignore_ascii_case("required"))
                        .unwrap_or(false);

                    let dep_type = if is_required || !is_mandatory {
                        DependantType::Required(modid.to_string())
                    } else {
                        DependantType::Recommend(modid.to_string())
                    };

                    mod_item.dependants.push(dep_type);
                }
            }
        }
    }

    Ok(())
}

fn read_fabric_json(mut reader: impl Read, mod_info: &mut ModObj) -> CoreResult<()> {
    let mut json = String::new();
    reader.read_to_string(&mut json).map_err(|err| {
        ErrorType::ArchiveReadError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let obj = serde_json::from_str::<serde_json::Value>(&json).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let mut info = ModItemObj::default();

    match obj {
        serde_json::Value::Object(map) => {
            info.mod_id = get_string_from_json(&map, "id");
            info.name = get_string_from_json(&map, "name");
            info.description = get_opt_string_from_json(&map, "description");
            info.version = get_opt_string_from_json(&map, "version");
            if let Some(serde_json::Value::Object(map1)) = map.get("contact") {
                info.url = get_opt_string_from_json(map1, "homepage");
            }

            if let Some(serde_json::Value::String(str)) = map.get("environment") {
                info.side = if str.eq_ignore_ascii_case("client") {
                    LoadSideType::Client
                } else if str.eq_ignore_ascii_case("server") {
                    LoadSideType::Server
                } else if str.eq_ignore_ascii_case("*") {
                    LoadSideType::Both
                } else {
                    LoadSideType::Unknown
                };
            }

            if let Some(serde_json::Value::Array(list)) = map.get("authors") {
                for item in list.iter() {
                    match item {
                        serde_json::Value::String(str) => {
                            info.author.push(str.to_string());
                        }
                        serde_json::Value::Object(map) => {
                            if let Some(serde_json::Value::String(str)) = map.get("name") {
                                info.author.push(str.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }

            if let Some(serde_json::Value::Object(str)) = map.get("depends") {
                for (key, _) in str.iter() {
                    info.dependants
                        .push(DependantType::Required(key.to_string()));
                }
            }

            if let Some(serde_json::Value::Object(str)) = map.get("suggests") {
                for (key, _) in str.iter() {
                    info.dependants
                        .push(DependantType::Recommend(key.to_string()));
                }
            }
        }
        _ => {}
    }

    mod_info.info.push(info);

    Ok(())
}

fn read_quilt_json(mut reader: impl Read, mod_info: &mut ModObj) -> CoreResult<()> {
    let mut json = String::new();
    reader.read_to_string(&mut json).map_err(|err| {
        ErrorType::ArchiveReadError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let obj = serde_json::from_str::<serde_json::Value>(&json).map_err(|err| {
        ErrorType::SerializerError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let mut info = ModItemObj::default();

    match obj {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Object(map)) = map.get("quilt_loader") {
                info.mod_id = get_string_from_json(&map, "id");
                info.version = get_opt_string_from_json(&map, "version");
                if let Some(serde_json::Value::Object(map)) = map.get("metadata") {
                    if let Some(serde_json::Value::Object(map1)) = map.get("contact") {
                        info.url = get_opt_string_from_json(map1, "homepage");
                    }

                    info.name = get_string_from_json(map, "name");
                    info.description = get_opt_string_from_json(map, "description");

                    if let Some(serde_json::Value::Object(map1)) = map.get("contributors") {
                        for (item, _) in map1.iter() {
                            info.author.push(item.to_string());
                        }
                    }
                }

                if let Some(serde_json::Value::Array(list)) = map.get("depends") {
                    for item in list.iter() {
                        match item {
                            serde_json::Value::Object(map) => {
                                if let Some(serde_json::Value::String(str)) = map.get("id") {
                                    info.dependants
                                        .push(DependantType::Required(str.to_string()));
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        _ => {}
    }

    mod_info.info.push(info);

    Ok(())
}

fn parse_manifest(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    let mut current_key = String::new();
    let mut current_value = String::new();

    for line in content.lines() {
        if line.is_empty() {
            continue;
        }

        if line.starts_with(' ') {
            // 续行
            current_value.push_str(line.trim());
        } else if let Some((key, value)) = line.split_once(':') {
            if !current_key.is_empty() {
                map.insert(current_key, current_value.trim().to_string());
            }
            current_key = key.trim().to_string();
            current_value = value.trim().to_string();
        }
    }

    if !current_key.is_empty() {
        map.insert(current_key, current_value.trim().to_string());
    }
    map
}

fn read_core_mod(
    archive: &mut ZipArchive<impl Read + Seek>,
    mod_info: &mut ModObj,
) -> CoreResult<()> {
    // 读取 META-INF/MANIFEST.MF
    let mut manifest_file = match archive.by_name("META-INF/MANIFEST.MF") {
        Ok(file) => file,
        Err(_) => return Ok(()),
    };

    let mut info = ModItemObj::default();

    let mut content = String::new();
    manifest_file.read_to_string(&mut content).map_err(|err| {
        ErrorType::ArchiveReadError(ErrorData {
            error: err.to_string(),
        })
    })?;

    let manifest = parse_manifest(&content);

    // 检查 FMLCorePlugin（Forge core mod 主类）
    if let Some(core_plugin) = manifest.get("FMLCorePlugin") {
        info.mod_id = core_plugin.clone();
        info.name = core_plugin
            .rsplit('.')
            .next()
            .unwrap_or(core_plugin)
            .to_string();
        info.loaders = LoaderType::Forge;
    }

    // 检查 TweakClass（LaunchWrapper 注入类）
    if let Some(tweak_class) = manifest.get("TweakClass") {
        if info.mod_id.is_empty() {
            info.mod_id = tweak_class.clone();
            info.name = tweak_class
                .rsplit('.')
                .next()
                .unwrap_or(tweak_class)
                .to_string();
        }
        info.loaders = LoaderType::Forge;
    }

    mod_info.info.push(info);
    mod_info.core = true;
    Ok(())
}

/// 读取jarinjar
/// - `archive`: 压缩包
/// - `mod_info`: 模组信息
fn read_jar_in_jar(
    archive: &mut ZipArchive<impl Read + Seek>,
    mod_info: &mut ModObj,
) -> CoreResult<()> {
    // 收集所有 META-INF/jarjar/ 目录下的 .jar 文件
    let jar_entries: Vec<usize> = (0..archive.len())
        .filter_map(|i| {
            archive.by_index(i).ok().and_then(|entry| {
                let name = entry.name();
                if name.ends_with(names::JAR_EXT)
                    && (name.starts_with(names::MOD_JAR_JAR_DIR)
                        || name.starts_with(names::MOD_JARS_DIR))
                {
                    Some(i)
                } else {
                    None
                }
            })
        })
        .collect();

    for idx in jar_entries {
        let mut entry = archive.by_index(idx).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;

        let cursor = Cursor::new(bytes);
        let mut inner_zip = ZipArchive::new(cursor).map_err(|err| {
            ErrorType::ArchiveOpenError(FileSystemErrorData {
                path: PathBuf::new(),
                error: err.to_string(),
            })
        })?;

        match parse_mod_archive(&mut inner_zip) {
            Ok(inmod) => {
                mod_info.jar_in_jar.push(inmod);
            }
            Err(err) => {
                mcml_log::error_type(err);
            }
        }
    }

    Ok(())
}

/// 从任意可读的 ZIP 归档中解析模组信息（核心解析逻辑）
/// - `archive`: 压缩包
fn parse_mod_archive(archive: &mut ZipArchive<impl Read + Seek>) -> CoreResult<ModObj> {
    let mut mod_info = ModObj::default();

    // mcmod.info
    if let Ok(item) = archive.by_name(names::MC_MOD_INFO_FILE) {
        read_forge_json(item, &mut mod_info)?;
    }

    // mods.toml
    macro_rules! try_read_file {
        ($archive:expr, $name:expr, $func:expr, $loader:expr) => {
            if let Ok(item) = $archive.by_name($name) {
                $func(item, $loader, &mut mod_info)?;
            }
        };
    }

    try_read_file!(
        archive,
        names::MC_MOD_TOML_FILE,
        read_forge_toml,
        LoaderType::Forge
    );
    try_read_file!(
        archive,
        names::NEO_TOML_FILE,
        read_forge_toml,
        LoaderType::NeoForge
    );
    try_read_file!(
        archive,
        names::NEO_TOML1_FILE,
        read_forge_toml,
        LoaderType::NeoForge
    );

    // fabric.mod.json
    if let Ok(item) = archive.by_name(names::FABRIC_MOD_FILE) {
        read_fabric_json(item, &mut mod_info)?;
    }

    // quilt.mod.json
    if let Ok(item) = archive.by_name(names::QUILT_MOD_FILE) {
        read_quilt_json(item, &mut mod_info)?;
    }

    // 扫描coremod
    read_core_mod(archive, &mut mod_info)?;

    // 扫描jar-in-jar
    read_jar_in_jar(archive, &mut mod_info)?;

    Ok(mod_info)
}

/// 读取模组信息
/// - `path`: 路径
pub fn read_mod_info<P: AsRef<Path>>(path: P) -> CoreResult<ModObj> {
    let file = path_helper::open_read(&path)?;
    let mut zip = ZipArchive::new(file).map_err(|err| {
        ErrorType::ArchiveOpenError(FileSystemErrorData {
            path: path.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;

    let mut mod_info = parse_mod_archive(&mut zip)?;
    mod_info.file = path.as_ref().to_path_buf();

    // 从注解扫描 side（仅文件类模组可用）
    if let Ok(scan_result) = crate::class_scan::scan_jar(path.as_ref()) {
        for scan_mod in &scan_result.mods {
            if let Some(info) = mod_info
                .info
                .iter_mut()
                .find(|info| info.mod_id == scan_mod.modid)
            {
                info.side = scan_mod.side;
            }
        }
    }

    Ok(mod_info)
}

/// 读模组
/// - `path`: 路径
/// - `sha256`: 是否计算sha256
fn read_mod<P: AsRef<Path>>(path: P, sha256: bool) -> CoreResult<ModObj> {
    let sha1 = hash_helper::gen_hash_from_file(HashType::Sha1, path.as_ref())?;

    let hash = if sha256 {
        let sha256 = hash_helper::gen_hash_from_file(HashType::Sha256, path.as_ref())?;
        FileHash::Sha1Sha256(sha1, sha256)
    } else {
        FileHash::Sha1(sha1)
    };

    let mut mod_info = read_mod_info(path)?;
    mod_info.hash = hash;

    Ok(mod_info)
}

/// 扫描文件列表
/// - `files`: 文件列表
/// - `process_fn`: 处理的函数
fn scan_mod_files<F>(files: Vec<PathBuf>, process_fn: F) -> Vec<ModObj>
where
    F: Fn(&PathBuf, bool) -> CoreResult<ModObj> + Send + Sync,
{
    let list = Mutex::new(Vec::new());

    files.par_iter().for_each(|item| {
        if let Some(ext) = item.extension() {
            let is_jar = ext.eq_ignore_ascii_case(names::JAR_EXT);
            let is_disabled = ext.eq_ignore_ascii_case(names::DISABLE_EXT)
                || ext.eq_ignore_ascii_case(names::DISABLED_EXT);

            if is_jar || is_disabled {
                let disable = is_disabled;
                let result = process_fn(item, false);

                let entry = match result {
                    Ok(mut item) => {
                        item.disable = disable;
                        item
                    }
                    Err(err) => {
                        mcml_log::error_type(err);
                        ModObj {
                            fail: true,
                            file: item.clone(),
                            ..Default::default()
                        }
                    }
                };

                list.lock().unwrap().push(entry);
            }
        }
    });

    list.into_inner().unwrap()
}

impl InstanceSettingObj {
    /// 扫描模组
    pub async fn read_mod_fast(&self) -> Vec<ModObj> {
        let dir = self.get_mods_path();
        let files = path_helper::get_files(dir);

        tokio::task::spawn_blocking(move || {
            scan_mod_files(files, |item, _| {
                let hash = hash_helper::gen_hash_from_file(HashType::Sha1, item)?;
                Ok(ModObj {
                    hash: FileHash::Sha1(hash),
                    file: item.clone(),
                    ..Default::default()
                })
            })
        })
        .await
        .unwrap_or_default()
    }

    /// 读取模组列表
    /// - `sha256`: 是否计算SHA256
    pub async fn read_mod(&self, sha256: bool) -> Vec<ModObj> {
        let dir = self.get_mods_path();
        let files = path_helper::get_files(dir);

        tokio::task::spawn_blocking(move || scan_mod_files(files, |item, _| read_mod(item, sha256)))
            .await
            .unwrap_or_default()
    }
}
