//! 传输对象（DTO）—— 前端 IPC wire 形态，使用 TS 命名（camelCase）。
//!
//! 配置 / 持久化文件（gui_config.json、main_data.json 等）用 Rust 命名，
//! 仅当跨 Tauri IPC 传输到前端时才转成 DTO（或 DTO 本身即 wire 形态，
//! 如事件负载 / 命令入参）。纯传输、无业务方法的窗口类型也集中在此。

pub mod account_dto;
pub mod add_dto;
pub mod download_dto;
pub mod gui_config_dto;
pub mod main_dto;

pub use account_dto::{AccountStoreDto, AccountStoreViewDto};
pub use add_dto::{
    DetectedPackDto, DirEntry, LoaderProgressDto, ModpackFileDto, ModpackItemDto, ModpackSearchDto,
    NameConflictDto, PackProgressDto,
};
pub use download_dto::{DownloadItemEvent, DownloadTaskDto, DownloadTaskEvent};
pub use gui_config_dto::{GuiConfigDto, MainWindowConfigDto};
pub use main_dto::{
    ErrorEvent, ExitEvent, InstanceChangeEvent, InstancePatch, LogEvent, NewsItem, StateEvent,
};
