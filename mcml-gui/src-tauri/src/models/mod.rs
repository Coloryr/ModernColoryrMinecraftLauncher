//! 通用数据模型（跨窗口共享，与前端 mcml-vue/src/lib/types.ts 对应）
//! 窗口专属模型（账户 / 新闻 / 游戏事件）见 `../windows/<kind>.rs`。

pub mod args;
pub mod instance;
pub mod java;
pub mod version;

pub use args::{EnvVarLine, InstanceArgs};
pub use instance::InstanceInfo;
pub use java::JavaInfo;
pub use version::VersionInfo;
