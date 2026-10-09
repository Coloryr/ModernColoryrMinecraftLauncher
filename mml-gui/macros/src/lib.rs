//! M²L GUI 过程宏
//!
//! [`emit`]：事件发射函数标记，本身不改动函数，仅作为源生成器的扫描标记。
//! src-tauri 的 build.rs 扫描 `#[gui_macros::emit]` 收集事件名单（事件名 = 函数名
//! 去 `emit_` 前缀、`_` -> `-`，如 emit_account_change -> account-change），据此
//! 生成前端 listens.ts 与 Rust `crate::listens` 常量。

use proc_macro::TokenStream;

/// 事件发射函数标记（属性宏，原样返回 item，不改动函数）
#[proc_macro_attribute]
pub fn emit(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// IPC 分组标记（属性宏，原样返回 item，不改动函数）
///
/// 只作为源生成器的扫描标记：命令拆进子模块时，靠它把 `bindings.ts` 的组键钉回
/// 原来的组（否则组键会跟着 .rs 文件变成子模块名，前端 `commands.<组>.*` 全断）。
/// 生成器侧见 `ipc-gen/src/scan.rs` 的 `group_of`，用法是
/// `#[gui_macros::ipc_group("resource")]`。
#[proc_macro_attribute]
pub fn ipc_group(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
