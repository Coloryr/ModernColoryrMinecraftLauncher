//! IPC 名字源生成器（扫描式，注册表零维护）
//!
//! 扫描 `src/` 源码收集名单：
//! - 命令：`#[tauri::command]` 函数。命令名 = 函数名，Rust 路径 = 文件路径推导的模块路径
//!   （src/windows/account.rs -> windows::account::fn_name）
//! - 事件：`mcml::emit` 标记的函数。事件名 = 函数名去 `emit_` 前缀、`_` -> `-`
//!   （emit_account_change -> account-change）
//!
//! 生成：
//! - `OUT_DIR/invokes_gen.rs`：事件常量（listens 模块）、`tauri_commands!` 注册宏
//! - `../../mcml-vue/src/lib/listens.ts`：事件名常量
//! - `../../mcml-vue/src/lib/bindings.ts`：命令的类型化包装 + 跨 IPC 类型的 TS 定义
//!
//! 时序：cargo 编译必先于页面加载（vite 按需编译），TS 生成时机足够；
//! 单独重新生成可跑 `npm run gen`（mcml-gui）。生成的 TS 随仓库提交。

use std::{
    fs,
    path::{Path, PathBuf},
};

/// 写出生成的 TS 文件
///
/// - 把输出文件也登记为 `rerun-if-changed`：被删掉或被手改时会重新生成
///   （否则只盯 `src`，删了文件 cargo 也不会重跑 build.rs）
/// - 内容没变就不写：避免每次都刷新 mtime，反过来让 build.rs 反复重跑
fn emit_ts(path: &Path, content: &str) {
    println!("cargo:rerun-if-changed={}", path.display());
    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        return;
    }
    if let Err(err) = fs::write(path, content) {
        panic!("写入 {} 失败: {err}", path.display());
    }
}

/// 递归收集目录下的 .rs 文件
fn walk_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// 文件路径 -> Rust 模块路径（src/windows/account.rs -> "windows::account"；
/// mod.rs / lib.rs / main.rs -> 所在目录即模块）
fn module_path(src: &Path, file: &Path) -> String {
    let rel = match file.strip_prefix(src) {
        Ok(rel) => rel,
        Err(_) => return String::new(),
    };
    let mut comps: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().to_string())
        .collect();
    comps.pop();
    let file_name = rel
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let stem = file_name.trim_end_matches(".rs");
    // mod.rs / src 根下的 lib.rs、main.rs 不追加自身名，其余文件名即模块名
    let is_special = stem == "mod" || ((stem == "lib" || stem == "main") && comps.is_empty());
    if !is_special {
        comps.push(stem.to_string());
    }
    comps.join("::")
}

/// 找 marker 之后的第一个函数名（跳过中间的文档 / 属性，标准格式下足够可靠）
fn fns_after(text: &str, marker: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(marker) {
        let after = &rest[pos + marker.len()..];
        if let Some(f) = after.find("fn ") {
            let ident: String = after[f + 3..]
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            if !ident.is_empty() {
                names.push(ident);
            }
        }
        rest = after;
    }
    names
}

/// snake_case / kebab-case -> SCREAMING_SNAKE_CASE（account-change -> ACCOUNT_CHANGE）
fn screaming(name: &str) -> String {
    name.replace('-', "_").to_ascii_uppercase()
}

// ================= 签名与类型解析（生成 bindings.ts 用） =================

/// 从 `from` 起，最早的 `pub struct` / `pub enum` 声明起始位置
fn next_decl(text: &str, from: usize) -> Option<usize> {
    let a = text[from..].find("\npub struct ").map(|p| p + from + 1);
    let b = text[from..].find("\npub enum ").map(|p| p + from + 1);
    match (a, b) {
        (Some(x), Some(y)) => Some(x.min(y)),
        (Some(x), None) => Some(x),
        (None, Some(y)) => Some(y),
        (None, None) => None,
    }
}

/// 扫描 `pub struct` / `pub enum` 声明
///
/// 返回 (类型名, 声明前的属性原文, 花括号内的正文, 是枚举)
fn scan_types(text: &str) -> Vec<(String, String, String, bool)> {
    let mut out = Vec::new();
    let mut search = 0usize;
    while let Some(pos) = next_decl(text, search) {
        let is_enum = text[pos..].starts_with("pub enum ");
        let head = if is_enum { "pub enum " } else { "pub struct " };
        let name_start = pos + head.len();
        let name: String = text[name_start..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if name.is_empty() {
            search = name_start;
            continue;
        }
        // 属性块：从声明往回逐行收集 `#[...]` / `///`，遇到其它内容（空行、`}`）即停
        let mut attr_lines: Vec<&str> = Vec::new();
        for line in text[..pos].lines().rev() {
            let t = line.trim();
            if t.starts_with("#[") || t.starts_with("///") {
                attr_lines.push(line);
            } else {
                break;
            }
        }
        attr_lines.reverse();
        let attrs = attr_lines.join("\n");

        // 正文：花括号配平
        let Some(open) = text[name_start..].find('{') else {
            search = name_start;
            continue;
        };
        let body_start = name_start + open + 1;
        let mut depth = 1usize;
        let mut i = body_start;
        let bytes = text.as_bytes();
        while i < bytes.len() && depth > 0 {
            match bytes[i] {
                b'{' => depth += 1,
                b'}' => depth -= 1,
                _ => {}
            }
            i += 1;
        }
        out.push((name, attrs, text[body_start..i.saturating_sub(1)].to_string(), is_enum));
        search = i;
    }
    out
}

/// 从属性块里取 `#[serde(rename_all = "camelCase")]` 的值
fn rename_all_of(attrs: &str) -> Option<String> {
    let p = attrs.find("rename_all")?;
    let rest = &attrs[p..];
    let q1 = rest.find('"')? + 1;
    let q2 = rest[q1..].find('"')? + q1;
    Some(rest[q1..q2].to_string())
}

/// 拆顶层逗号（不切泛型里的逗号）
fn split_top_level(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut cur = String::new();
    for c in s.chars() {
        match c {
            '<' | '(' | '[' => {
                depth += 1;
                cur.push(c);
            }
            '>' | ')' | ']' => {
                depth -= 1;
                cur.push(c);
            }
            ',' if depth == 0 => {
                out.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    if !cur.trim().is_empty() {
        out.push(cur.trim().to_string());
    }
    out
}

/// `Option<T>` -> `T`；不是该泛型则 None
fn generic_inner<'a>(ty: &'a str, name: &str) -> Option<String> {
    let t = ty.trim();
    let rest = t.strip_prefix(name)?;
    let rest = rest.strip_prefix('<')?;
    let rest = rest.strip_suffix('>')?;
    Some(rest.to_string())
}

/// Rust 类型 -> TS 类型
fn ts_type(ty: &str) -> String {
    let t = ty.trim();
    // 引用类型（`&str`、`&'static str`）按值处理
    let t = t.trim_start_matches('&').trim_start_matches("'static").trim();

    if let Some(inner) = generic_inner(t, "Option") {
        return format!("{} | null", ts_type(&inner));
    }
    if let Some(inner) = generic_inner(t, "Vec") {
        let inner = ts_type(&inner);
        return if inner.contains('|') || inner.contains(' ') {
            format!("({inner})[]")
        } else {
            format!("{inner}[]")
        };
    }
    if let Some(inner) = generic_inner(t, "HashMap").or_else(|| generic_inner(t, "BTreeMap")) {
        let kv = split_top_level(&inner);
        if kv.len() == 2 {
            return format!("Record<{}, {}>", ts_type(&kv[0]), ts_type(&kv[1]));
        }
    }

    match t {
        "String" | "str" => "string".into(),
        "bool" => "boolean".into(),
        "i8" | "i16" | "i32" | "i64" | "isize" | "u8" | "u16" | "u32" | "u64" | "usize"
        | "f32" | "f64" => "number".into(),
        // 空串 = Rust 侧写了 `fn f()`（无返回类型），等价于 `()`
        "" | "()" => "void".into(),
        // 外部 crate 的 unit 枚举（mcml-names 的 Lang）：线上是变体名，
        // 变体固定为 zh_cn / en_us，前端也一直是这么写死的（见 lib/guiConfig.ts 的 Locale）
        "Lang" => "\"zh_cn\" | \"en_us\"".into(),
        // 本 crate 生成的类型：直接用名字
        other => other.to_string(),
    }
}

/// 字段类型 -> (TS 类型, 是否可选)
///
/// - `Option<Option<T>>` 是三态补丁（不传 = 不改 / null = 清空 / 有值 = 更新）：
///   可缺省且可为 null
/// - `input_only`（只 derive `Deserialize` 的入参 DTO）：serde 允许 `Option` 字段缺失，
///   所以也标成可缺省；输出 DTO 的字段则一定会被序列化出来，保持必填
fn ts_field(ty: &str, input_only: bool) -> (String, bool) {
    if let Some(inner) = generic_inner(ty.trim(), "Option") {
        if let Some(inner2) = generic_inner(inner.trim(), "Option") {
            return (format!("{} | null", ts_type(&inner2)), true);
        }
        return (format!("{} | null", ts_type(&inner)), input_only);
    }
    (ts_type(ty), false)
}

/// `ab_cd` / `ab-cd` -> `abCd`（缩写修正 Oauth -> OAuth，与现有 invokes.ts 命名保持一致）
fn camel(name: &str) -> String {
    let mut parts = name.split(['_', '-']).filter(|s| !s.is_empty());
    let first = parts.next().unwrap_or("").to_string();
    let rest: String = parts
        .map(|s| {
            let mut c = s.chars();
            format!("{}{}", c.next().unwrap().to_ascii_uppercase(), c.as_str())
        })
        .collect();
    format!("{first}{rest}").replace("Oauth", "OAuth")
}

/// 取 `#[tauri::command]` 后面的函数：返回 (函数名, 参数原文, 返回类型原文)
///
/// 参数/返回按括号与尖括号配平截取，能处理跨多行的签名。
fn commands_in(text: &str) -> Vec<(String, String, String)> {
    const MARK: &str = "#[tauri::command]";
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(pos) = rest.find(MARK) {
        let after = &rest[pos + MARK.len()..];
        let Some(fpos) = after.find("fn ") else {
            rest = after;
            continue;
        };
        let after_fn = &after[fpos + 3..];
        let name: String = after_fn
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        let Some(lp) = after_fn.find('(') else {
            rest = after_fn;
            continue;
        };

        // 参数：括号配平
        let mut depth = 0i32;
        let mut end = None;
        for (i, c) in after_fn[lp..].char_indices() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(lp + i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(rp) = end else {
            rest = after_fn;
            continue;
        };
        let params = after_fn[lp + 1..rp].to_string();

        // 返回类型：若紧跟 `->`，取到函数体的 `{`（尖括号/圆括号配平）
        let tail = &after_fn[rp + 1..];
        let ret = match tail.find("->") {
            Some(a) if tail[..a].trim().is_empty() => {
                let ty_start = a + 2;
                let mut d = 0i32;
                let mut cut = tail.len();
                for (i, c) in tail[ty_start..].char_indices() {
                    match c {
                        '<' | '(' | '[' => d += 1,
                        '>' | ')' | ']' => d -= 1,
                        '{' if d == 0 => {
                            cut = ty_start + i;
                            break;
                        }
                        _ => {}
                    }
                }
                tail[ty_start..cut].trim().to_string()
            }
            _ => String::new(),
        };

        out.push((name, params, ret));
        rest = tail;
    }
    out
}

/// 参数原文 -> Vec<(TS 参数名, TS 类型, 是否可选)>，并丢掉 Tauri 注入的参数
fn ts_params(params: &str) -> Vec<(String, String, bool)> {
    split_top_level(params)
        .into_iter()
        .filter_map(|p| {
            let (name, ty) = p.split_once(':')?;
            let name = name.trim().trim_start_matches("mut ").trim();
            let ty = ty.trim();
            // Tauri 注入参数不进前端签名
            if ty == "AppHandle" || ty == "WebviewWindow" || ty == "Window" || ty == "State" {
                return None;
            }
            let (ts, optional) = ts_field(ty, false);
            Some((camel(name.trim_start_matches("r#")), ts, optional))
        })
        .collect()
}

/// `Result<T, E>` -> `T`；其它原样（命令返回值都是 Promise，错误通道另算）
fn result_ok(ty: &str) -> String {
    if let Some(inner) = generic_inner(ty, "Result") {
        let parts = split_top_level(&inner);
        if let Some(first) = parts.first() {
            return first.clone();
        }
    }
    ty.to_string()
}

/// snake_case / kebab-case -> PascalCase，缩写词修正 Oauth -> OAuth
fn pascal(name: &str) -> String {
    name.split(['_', '-'])
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut chars = s.chars();
            format!(
                "{}{}",
                chars.next().unwrap().to_ascii_uppercase(),
                chars.as_str()
            )
        })
        .collect::<Vec<_>>()
        .join("")
        .replace("Oauth", "OAuth")
}

fn main() {
    tauri_build::build();

    // src 下任何文件变动都触发重新扫描；单独重生成可跑 npm run gen
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("src");

    let mut files = Vec::new();
    walk_rs(&src, &mut files);
    files.sort();

    // 收集命令（命令名 + Rust 路径）与事件名；排序去重保证输出稳定
    let mut commands: Vec<(String, String)> = Vec::new();
    let mut events: Vec<String> = Vec::new();
    for file in &files {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        let module = module_path(&src, file);
        for name in fns_after(&text, "#[tauri::command]") {
            let path = if module.is_empty() {
                name.clone()
            } else {
                format!("{module}::{name}")
            };
            commands.push((name, path));
        }
        for name in fns_after(&text, "#[gui_macros::emit]") {
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

    // ---------- Rust：OUT_DIR/invokes_gen.rs ----------
    let mut rs = String::from("// @generated by build.rs（扫描 src/ 收集）—— 勿手改\n\n");

    rs.push_str("pub mod listens {\n");
    for name in &events {
        rs.push_str(&format!(
            "    pub const {}: &str = \"{}\";\n",
            screaming(name),
            name
        ));
    }
    rs.push_str("}\n\n");

    // 命令注册宏：lib.rs 里 invoke_handler(tauri_commands!())
    rs.push_str("/// generate_handler 注册表（扫描 src/ 生成）\n");
    rs.push_str("#[macro_export]\nmacro_rules! tauri_commands {\n");
    rs.push_str("    () => {\n        tauri::generate_handler![\n");
    for (_, path) in &commands {
        rs.push_str(&format!("            {},\n", path));
    }
    rs.push_str("        ]\n    };\n}\n");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    fs::write(out_dir.join("invokes_gen.rs"), rs).expect("写入 invokes_gen.rs 失败");

    // ---------- 前端：mcml-vue/src/lib/{listens,bindings}.ts ----------
    let ts_dir = manifest.join("../../mcml-vue/src/lib");

    let mut ts = String::from(
        "// @generated by mcml-gui/src-tauri/build.rs —— 勿手改，新增事件请给函数加 gui_macros::emit 标记\n\n",
    );
    for name in &events {
        ts.push_str(&format!(
            "export const {}: string = \"{}\"\n",
            pascal(name),
            name
        ));
    }
    emit_ts(&ts_dir.join("listens.ts"), &ts);

    // ---------- 前端：mcml-vue/src/lib/bindings.ts（类型 + 类型化命令） ----------
    //
    // 覆盖范围：`dtos/`、`models/` 下的全部类型，外加 `gui_config.rs` 里的枚举
    // （Theme / WindowMode / SidebarSide / ViewMode / HeadType 被 GuiConfigDto 引用）。
    // 只扫这三处是刻意的：`collect_utils.rs` 那种磁盘类型不该出现在 IPC 类型里。
    let mut decls: Vec<(String, String, String, bool)> = Vec::new();
    for file in &files {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        let rel = file
            .strip_prefix(&src)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let wanted = rel.starts_with("dtos/") || rel.starts_with("models/");
        let enums_only = rel == "gui_config.rs";
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
            decls.push((name, attrs, body, is_enum));
        }
    }

    // 命令签名：带来源模块，供按 .rs 分组用
    let mut sigs: Vec<(String, String, Vec<(String, String, bool)>, String)> = Vec::new();
    for file in &files {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        let module = module_path(&src, file);
        for (name, params, ret) in commands_in(&text) {
            sigs.push((
                module.clone(),
                name,
                ts_params(&params),
                ts_type(&result_ok(&ret)),
            ));
        }
    }
    sigs.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    let mut b = String::from(
        "// @generated by mcml-gui/src-tauri/build.rs —— 勿手改\n\
         //\n\
         // 命令的类型化包装 + 跨 IPC 类型的 TS 定义，都由 Rust 侧源码扫描生成。\n\n\
         import { invoke } from \"@tauri-apps/api/core\";\n\n",
    );

    // 命令表：按来源 .rs 模块分组（AGENTS §4 下，命令名 = 窗口名_方法名，
    // 所以同一文件里的命令共享同一个前段；把它剥掉，组内只剩方法名）
    b.push_str("/** 命令，按来源 .rs 模块分组（参数顺序与 Rust 一致，Tauri 注入的 AppHandle/WebviewWindow 已剔除）*/\n");
    b.push_str("export const commands = {\n");

    let mut i = 0usize;
    while i < sigs.len() {
        let module = sigs[i].0.clone();
        let start = i;
        while i < sigs.len() && sigs[i].0 == module {
            i += 1;
        }
        let group: Vec<&(String, String, Vec<(String, String, bool)>, String)> =
            sigs[start..i].iter().collect();

        // 组键取模块最后一段（即 .rs 文件名），如 windows::main -> main
        let group_key = camel(module.rsplit("::").next().unwrap_or(&module));

        // 组内命令名的公共前段，按 `_` 分段取最长公共前缀
        // （`add.rs` -> `add`；`add_modpack.rs` -> `add_modpack`；`window_manager.rs` -> `window`）
        let segs = |n: &str| n.split('_').map(|s| s.to_string()).collect::<Vec<_>>();
        let mut common = segs(&group[0].1);
        for (_, n, _, _) in &group {
            let other = segs(n);
            let mut k = 0;
            while k < common.len() && k < other.len() && common[k] == other[k] {
                k += 1;
            }
            common.truncate(k);
        }
        // 公共前缀不能吃掉整个名字，否则方法名会空（单命令组会走到这里）
        let mut strip_len = common.len();
        if strip_len > 0 && group.iter().any(|(_, n, _, _)| segs(n).len() == strip_len) {
            strip_len -= 1;
        }
        let strip = if strip_len == 0 {
            None
        } else {
            Some(common[..strip_len].join("_") + "_")
        };

        b.push_str(&format!("  {group_key}: {{\n"));
        for (_, name, params, ret) in group {
            let method = match &strip {
                Some(prefix) => name.strip_prefix(prefix).unwrap_or(name),
                None => name.as_str(),
            };
            let list = params
                .iter()
                .map(|(n, t, opt)| format!("{n}{}: {t}", if *opt { "?" } else { "" }))
                .collect::<Vec<_>>()
                .join(", ");
            let arg = if params.is_empty() {
                String::new()
            } else {
                let obj = params
                    .iter()
                    .map(|(n, _, _)| n.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(", {{ {obj} }}")
            };
            b.push_str(&format!(
                "    {}: ({list}) => invoke<{ret}>(\"{name}\"{arg}),\n",
                camel(method)
            ));
        }
        b.push_str("  },\n");
    }
    b.push_str("};\n\n");

    // 类型定义
    b.push_str("/** 类型 */\n");
    for (name, attrs, body, is_enum) in &decls {
        let rename_all = rename_all_of(attrs);
        if *is_enum {
            let variants: Vec<String> = split_top_level(body)
                .into_iter()
                .filter_map(|v| {
                    // 跳过变体前置的文档注释与属性（如 `#[default]`），再取标识符
                    let line = v
                        .lines()
                        .map(|l| l.trim())
                        .find(|l| !l.is_empty() && !l.starts_with("//") && !l.starts_with("#["))?;
                    let name: String = line
                        .chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect();
                    if name.is_empty() {
                        return None;
                    }
                    Some(match rename_all.as_deref() {
                        Some("lowercase") => name.to_ascii_lowercase(),
                        Some("camelCase") => camel(&name),
                        _ => name,
                    })
                })
                .collect();
            b.push_str(&format!(
                "export type {name} = {};\n\n",
                variants
                    .iter()
                    .map(|v| format!("\"{v}\""))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
            continue;
        }

        // 只 derive Deserialize = 前端传给后端的入参 DTO（字段可缺省，见 ts_field）
        let input_only = !attrs.contains("Serialize");
        b.push_str(&format!("export type {name} = {{\n"));
        for line in body.lines() {
            let line = line.trim();
            if line.starts_with("//") || line.starts_with("#[") || line.is_empty() {
                continue;
            }
            let Some((raw_name, raw_ty)) = line.split_once(':') else {
                continue;
            };
            let raw_name = raw_name.trim().trim_start_matches("pub ").trim();
            if raw_name.is_empty() || raw_name.starts_with('/') {
                continue;
            }
            let raw_name = raw_name.trim_start_matches("r#");
            let raw_ty = raw_ty.trim().trim_end_matches(',').trim();
            let ts_name = match rename_all.as_deref() {
                Some("camelCase") => camel(raw_name),
                Some("lowercase") => raw_name.to_ascii_lowercase(),
                _ => raw_name.to_string(),
            };
            let (ts, opt) = ts_field(raw_ty, input_only);
            b.push_str(&format!("  {ts_name}{}: {ts},\n", if opt { "?" } else { "" }));
        }
        b.push_str("};\n\n");
    }

    emit_ts(&ts_dir.join("bindings.ts"), &b);
}
