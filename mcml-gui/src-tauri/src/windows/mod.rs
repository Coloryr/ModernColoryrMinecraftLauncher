//! 窗口模块
//!
//! 每个窗口一个 rs 文件，包含：
//! - 窗口规格（模型）：标签 / 标题 / 尺寸常量
//! - 窗口专属数据模型与方法（账户 / 新闻 / 游戏事件等）
//! - 窗口创建操作：`open(app)` 创建（或聚焦）对应窗口
//! - 窗口按钮调用的方法（IPC 命令，如 list_dir）
//! 通用数据模型见 `../models/`。
//!
//! 窗口的创建 / 聚焦 / 关闭统一由 `../window_manager.rs` 处理，
//! 各窗口的 `open` 只负责传入自己的规格（标签 / 标题 / 尺寸）。

pub mod account;
pub mod add;
pub mod help;
pub mod main;
pub mod resource;
pub mod settings;
pub mod skin;
pub mod stats;
