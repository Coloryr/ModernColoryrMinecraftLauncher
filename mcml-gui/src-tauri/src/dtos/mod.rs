//! 传输对象（DTO）—— 前端 IPC wire 形态，使用 TS 命名（camelCase）。
//!
//! 配置 / 持久化文件（gui_config.json、main_data.json 等）用 Rust 命名，
//! 仅当跨 Tauri IPC 传输到前端时才转成 DTO（或 DTO 本身即 wire 形态，
//! 如事件负载 / 命令入参）。纯传输、无业务方法的窗口类型也集中在此。

pub mod account;
pub mod add;
pub mod gui_config;
pub mod main;

pub use account::AccountStoreView;
pub use add::DirEntry;
pub use gui_config::{GuiConfigDto, MainWindowConfigDto};
pub use main::{
    ErrorEvent, ExitEvent, InstanceChangeEvent, InstancePatch, LogEvent, NewsItem, StateEvent,
};
