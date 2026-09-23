//! M²L 的 IPC 源生成器
//!
//! 扫 Rust 源码（`#[tauri::command]` 命令 + `#[gui_macros::emit]` 事件 + 跨 IPC 类型），
//! 产出三份文本：
//!
//! - `mml-vue/src/lib/bindings.ts`：类型定义 + 类型化命令包装
//! - `mml-vue/src/lib/listens.ts`：事件名常量
//! - `$OUT_DIR/invokes_gen.rs`：`tauri_commands!` 注册宏 + `listens` 常量
//!
//! [`generate`] 是纯函数：只读源码、只返回文本，不碰环境变量也不写文件。
//! 写文件与 cargo 指令由调用方（`src-tauri/build.rs`）负责。

mod emit_rs;
mod emit_ts;
pub mod scan;
pub mod ts;

use std::{
    collections::BTreeMap,
    fmt, fs, io,
    path::{Path, PathBuf},
};

use scan::{commands_in, fns_after, module_path, scan_types, walk_rs};
use ts::{result_ok, ts_params, ts_type};

/// 抓命令的标记（本仓约定，不做成配置）
const CMD_MARK: &str = "#[tauri::command]";
/// 抓 emit 事件的标记（同上）
const EMIT_MARK: &str = "#[gui_macros::emit]";

/// TS 类型的来源：只有这些位置的类型会进 `bindings.ts`
///
/// 刻意收窄范围：`collect_utils.rs` 那种磁盘类型不该出现在 IPC 类型里。
#[derive(Debug, Clone)]
pub struct TypeSources {
    /// 目录前缀（相对 `src_dir`，带结尾 `/`）：其下所有类型都收
    pub dirs: Vec<String>,
    /// 单文件（相对 `src_dir`）：只收其中的枚举
    pub enum_files: Vec<String>,
}

impl Default for TypeSources {
    fn default() -> Self {
        Self {
            dirs: vec!["dtos/".into(), "models/".into()],
            enum_files: vec!["gui_config.rs".into()],
        }
    }
}

/// 生成配置
#[derive(Debug, Clone, Default)]
pub struct GenConfig {
    /// 要扫描的 Rust 源码根（通常 `<crate>/src`）；必须存在，否则 [`generate`] 报错
    pub src_dir: PathBuf,
    /// TS 类型的来源，默认 [`TypeSources::default`]
    pub type_sources: TypeSources,
    /// 外部 crate 类型 → TS 类型覆盖（如 `mml-names` 的 `Lang`）。
    /// 不在表里的类型名按「本 crate 生成」处理，直接用原名
    pub external_types: BTreeMap<String, String>,
}

/// 生成结果：三份文本，调用方自己决定往哪写
#[derive(Debug, Clone)]
pub struct Generated {
    pub bindings_ts: String,
    pub listens_ts: String,
    pub invokes_gen_rs: String,
}

#[derive(Debug)]
pub enum GenError {
    /// `src_dir` 不存在（通常是忘了设置 `GenConfig.src_dir`）
    SrcDirMissing(PathBuf),
}

impl fmt::Display for GenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SrcDirMissing(p) => write!(f, "源码目录不存在: {}", p.display()),
        }
    }
}

impl std::error::Error for GenError {}

/// 扫描类型声明（`Decl` 内部的中间表示）
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Decl {
    pub(crate) name: String,
    pub(crate) attrs: String,
    pub(crate) body: String,
    pub(crate) is_enum: bool,
}

/// 一条命令的签名（参数已转成 TS）
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Sig {
    pub(crate) module: String,
    pub(crate) name: String,
    /// (TS 参数名, TS 类型, 是否可缺省)
    pub(crate) params: Vec<(String, String, bool)>,
    pub(crate) ret: String,
}

/// 扫源码生成三份文本。纯函数：不写文件、不读环境变量
pub fn generate(cfg: &GenConfig) -> Result<Generated, GenError> {
    let src = cfg.src_dir.as_path();
    if !src.is_dir() {
        return Err(GenError::SrcDirMissing(cfg.src_dir.clone()));
    }

    let mut files = Vec::new();
    walk_rs(src, &mut files);
    files.sort();

    // 收集命令（命令名 + Rust 路径）与事件名；排序去重保证输出稳定
    let mut commands: Vec<(String, String)> = Vec::new();
    let mut events: Vec<String> = Vec::new();
    for file in &files {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        let module = module_path(src, file);
        for name in fns_after(&text, CMD_MARK) {
            let path = if module.is_empty() {
                name.clone()
            } else {
                format!("{module}::{name}")
            };
            commands.push((name, path));
        }
        for name in fns_after(&text, EMIT_MARK) {
            let event = name
                .strip_prefix("emit_")
                .unwrap_or(&name)
                .replace('_', "-");
            events.push(event);
        }
    }
    commands.sort();
    commands.dedup();
    events.sort();
    events.dedup();

    // 类型声明：来自 dtos/ 与 models/ 的全部类型，外加 gui_config.rs 里的枚举
    // （Theme / WindowMode / SidebarSide / ViewMode / HeadType 被 GuiConfigDto 引用）
    let mut decls: Vec<Decl> = Vec::new();
    for file in &files {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        let rel = rel_path(src, file);
        let wanted = cfg.type_sources.dirs.iter().any(|d| rel.starts_with(d.as_str()));
        let enums_only = cfg.type_sources.enum_files.iter().any(|f| f == &rel);
        if !wanted && !enums_only {
            continue;
        }
        for (name, attrs, body, is_enum) in scan_types(&text) {
            if enums_only && !is_enum {
                continue;
            }
            // 没有 serde derive 的不是 IPC 类型
            if !attrs.contains("Serialize") && !attrs.contains("Deserialize") {
                continue;
            }
            decls.push(Decl {
                name,
                attrs,
                body,
                is_enum,
            });
        }
    }

    // 命令签名：带来源模块，供按 .rs 分组用
    let mut sigs: Vec<Sig> = Vec::new();
    for file in &files {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        let module = module_path(src, file);
        for (name, params, ret) in commands_in(&text) {
            sigs.push(Sig {
                module: module.clone(),
                name,
                params: ts_params(&params, &cfg.external_types),
                ret: ts_type(&result_ok(&ret), &cfg.external_types),
            });
        }
    }
    sigs.sort_by(|a, b| (&a.module, &a.name).cmp(&(&b.module, &b.name)));

    Ok(Generated {
        bindings_ts: emit_ts::bindings_ts(&decls, &sigs, &cfg.external_types),
        listens_ts: emit_ts::listens_ts(&events),
        invokes_gen_rs: emit_rs::invokes_gen(&events, &commands),
    })
}

/// 只在内容变化时写，返回是否真的写了
///
/// 内容相同就跳过：否则每次 cargo 构建都会改动 mtime，触发前端重跑。
pub fn write_if_changed(path: &Path, content: &str) -> io::Result<bool> {
    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(true)
}

/// 相对 `src` 的路径，统一用 `/` 分隔（Windows 上是 `\`）
fn rel_path(src: &Path, file: &Path) -> String {
    file.strip_prefix(src)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// src_dir 不存在要报错，而不是静默生成空产物
    #[test]
    fn test_generate_rejects_missing_src_dir() {
        let cfg = GenConfig {
            src_dir: PathBuf::from("H:/Temp/__definitely_not_here__"),
            ..Default::default()
        };
        assert!(matches!(generate(&cfg), Err(GenError::SrcDirMissing(_))));
    }

    /// 内容没变不写（返回 false），变了才写
    #[test]
    fn test_write_if_changed_skips_identical() {
        let dir = std::env::temp_dir().join("gui_ipc_gen_wic");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("a/b.ts");
        assert!(write_if_changed(&path, "x").unwrap(), "首次应写入");
        assert!(!write_if_changed(&path, "x").unwrap(), "内容相同不该写");
        assert!(write_if_changed(&path, "y").unwrap(), "内容变了应写入");
        assert_eq!(fs::read_to_string(&path).unwrap(), "y");
        let _ = fs::remove_dir_all(&dir);
    }
}
