//! MCML GUI 过程宏
//!
//! [`emit`]：事件发射函数标记，本身不改动函数，仅作为源生成器的扫描标记。
//! src-tauri 的 build.rs 扫描 `#[gui_macros::emit]` 收集事件名单（事件名 = 函数名
//! 去 `emit_` 前缀、`_` -> `-`，如 emit_account_change -> account-change），据此
//! 生成前端 listens.ts 与 Rust `crate::listens` 常量。

use proc_macro::TokenStream;

#[proc_macro_attribute]
pub fn emit(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}
