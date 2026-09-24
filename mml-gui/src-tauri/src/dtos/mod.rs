//! 传输对象（DTO）—— 前端 IPC wire 形态，使用 TS 命名（camelCase）。
//!
//! 配置 / 持久化文件（gui_config.json、main_data.json 等）用 Rust 命名，
//! 仅当跨 Tauri IPC 传输到前端时才转成 DTO（或 DTO 本身即 wire 形态，
//! 如事件负载 / 命令入参）。纯传输、无业务方法的窗口类型也集中在此。

pub mod account_dto;
pub mod add_dto;
pub mod add_modpack_dto;
pub mod add_resource_dto;
pub mod args_dto;
pub mod collect_dto;
pub mod download_dto;
pub mod gui_config_dto;
pub mod instance_dto;
pub mod java_dto;
pub mod main_dto;
pub mod resource_dto;
pub mod version_dto;

pub use account_dto::{AccountStoreDto, AccountStoreViewDto};
pub use args_dto::{EnvVarLineDto, InstanceArgsDto};
pub use instance_dto::InstanceInfoDto;
pub use java_dto::JavaInfoDto;
pub use version_dto::VersionInfoDto;
pub use add_dto::{
    DetectedPackDto, DirEntry, LoaderProgressDto, ModpackItemDto, NameConflictDto, PackProgressDto,
};
pub use add_modpack_dto::{ModPackStatusDto, ModPackTaskDto};
pub use add_resource_dto::{
    DecPicDto, FileListDto, FileListItemDto, McmodDto, PicDto, ProjectDetailDto, ProjectDto,
    ProjectItemDto, ResourceSaveDto, ResourceStatusDto, ResourceTaskDto, SourceTypeDto, TagDto,
};
pub use collect_dto::{CollectDataDto, CollectItemDto};
pub use download_dto::{
    DownloadItemEvent, DownloadStatusDto, DownloadTaskDto, DownloadTaskEvent, DownloadThreadDto,
};
pub use gui_config_dto::{GuiConfigDto, MainWindowConfigDto};
pub use main_dto::{
    BlockItemDto, BlockStatusDto, ErrorEvent, ExitEvent, InstanceChangeEvent, InstancePatch,
    LogEvent, NewsItem, StateEvent,
};
pub use resource_dto::{
    DataPackItemDto, ModItemDto, PackItemDto, SaveItemDto, ScreenshotItemDto, ServerItemDto,
    ShaderItemDto, SchematicItemDto,
};
