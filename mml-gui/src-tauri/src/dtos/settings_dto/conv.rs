//! 设置窗口 DTO 的枚举 ↔ 字符串转换：下载源 / 代理状态 / 代理类型 / GC 类型
//!
//! 从 `settings_dto/mod.rs` 拆出来的。**这些字符串是前端契约**（写进 `config.json` 的也是
//! 它们），改动前先看已存的配置值。

use mml_config::config_obj::{GCType, ProxyState, ProxyType, SourceLocal};

/// 下载源 → wire 字符串（`Offical` 是 core 的既定拼写，保持一致）
pub(super) fn source_to_str(s: SourceLocal) -> String {
    match s {
        SourceLocal::Offical => "Offical".to_string(),
        SourceLocal::Bmclapi => "Bmclapi".to_string(),
    }
}

pub(super) fn source_from_str(s: &str) -> SourceLocal {
    match s {
        "Bmclapi" => SourceLocal::Bmclapi,
        _ => SourceLocal::Offical,
    }
}

pub(super) fn proxy_state_to_str(s: ProxyState) -> String {
    match s {
        ProxyState::Auto => "Auto".to_string(),
        ProxyState::None => "None".to_string(),
        ProxyState::User => "User".to_string(),
    }
}

pub(super) fn proxy_state_from_str(s: &str) -> ProxyState {
    match s {
        "None" => ProxyState::None,
        "User" => ProxyState::User,
        _ => ProxyState::Auto,
    }
}

pub(super) fn proxy_type_to_str(t: ProxyType) -> String {
    match t {
        ProxyType::Http => "Http".to_string(),
        ProxyType::Sock4 => "Sock4".to_string(),
        ProxyType::Sock5 => "Sock5".to_string(),
    }
}

pub(super) fn proxy_type_from_str(s: &str) -> ProxyType {
    match s {
        "Sock4" => ProxyType::Sock4,
        "Sock5" => ProxyType::Sock5,
        _ => ProxyType::Http,
    }
}

pub(super) fn gc_to_str(g: GCType) -> String {
    match g {
        GCType::Auto => "Auto".to_string(),
        GCType::G1GC => "G1GC".to_string(),
        GCType::ZGC => "ZGC".to_string(),
        GCType::None => "None".to_string(),
    }
}

pub(super) fn gc_from_str(s: &str) -> GCType {
    match s {
        "G1GC" => GCType::G1GC,
        "ZGC" => GCType::ZGC,
        "None" => GCType::None,
        _ => GCType::Auto,
    }
}
