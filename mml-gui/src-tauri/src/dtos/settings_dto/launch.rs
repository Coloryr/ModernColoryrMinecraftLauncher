//! 启动设置 DTO：运行参数（内存 / 窗口 / GC / 自定义命令）
//!
//! 从 `settings_dto/mod.rs` 拆出来的。与 `config.json` 的 `RunArgObj` 双向转换。

use serde::{Deserialize, Serialize};

use mml_config::config_obj::{GCType, RunArgObj};

use super::conv::{gc_from_str, gc_to_str};

/// 启动参数（前端 wire：camelCase，字段即 core `RunArgObj`）
///
/// core 侧是 `Option` 字段（`None` = 用全局默认），DTO 落到
/// `RunArgObj::new()` 同款默认值；保存时全部写回 `Some(...)`。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct RunArgSettingDto {
    /// 是否移除原有的 JVM 参数
    pub remove_jvm_arg: bool,
    /// 是否移除原有的游戏参数
    pub remove_game_arg: bool,
    /// 自定义 JVM 参数（换行分隔多条）
    pub jvm_args: String,
    /// 自定义游戏参数（换行分隔多条）
    pub game_args: String,
    /// 自定义 JVM 环境变量
    pub jvm_env: String,
    /// GC 模式：Auto / G1GC / ZGC / None
    pub gc_mode: String,
    /// 最小内存（MB）
    pub min_memory: u32,
    /// 最大内存（MB）
    pub max_memory: u32,
    /// 是否启用 ColorASM（彩色日志输出）
    pub colorasm: bool,
    /// 是否在启动游戏前执行预启动命令
    pub launch_pre_run: bool,
    /// 预启动命令与游戏同时运行（true）还是等命令结束再启动游戏（false）
    pub pre_run_with_game: bool,
    /// 是否在游戏结束后执行后置命令
    pub launch_post_run: bool,
    /// 预启动命令内容
    pub pre_run_arg: String,
    /// 后置命令内容
    pub post_run_arg: String,
}

impl From<&RunArgObj> for RunArgSettingDto {
    fn from(r: &RunArgObj) -> Self {
        Self {
            remove_jvm_arg: r.remove_jvm_arg.unwrap_or(false),
            remove_game_arg: r.remove_game_arg.unwrap_or(false),
            jvm_args: r.jvm_args.clone().unwrap_or_default(),
            game_args: r.game_args.clone().unwrap_or_default(),
            jvm_env: r.jvm_env.clone().unwrap_or_default(),
            gc_mode: gc_to_str(r.gc_mode.unwrap_or(GCType::Auto)),
            min_memory: r.min_memory.unwrap_or(512),
            max_memory: r.max_memory.unwrap_or(4096),
            colorasm: r.colorasm.unwrap_or(false),
            launch_pre_run: r.launch_pre_run.unwrap_or(false),
            pre_run_with_game: r.pre_run_with_game.unwrap_or(true),
            launch_post_run: r.launch_post_run.unwrap_or(false),
            pre_run_arg: r.pre_run_arg.clone().unwrap_or_default(),
            post_run_arg: r.post_run_arg.clone().unwrap_or_default(),
        }
    }
}

impl From<RunArgSettingDto> for RunArgObj {
    fn from(d: RunArgSettingDto) -> Self {
        Self {
            remove_jvm_arg: Some(d.remove_jvm_arg),
            remove_game_arg: Some(d.remove_game_arg),
            jvm_args: Some(d.jvm_args),
            game_args: Some(d.game_args),
            jvm_env: Some(d.jvm_env),
            gc_mode: Some(gc_from_str(&d.gc_mode)),
            max_memory: Some(d.max_memory),
            min_memory: Some(d.min_memory),
            colorasm: Some(d.colorasm),
            launch_pre_run: Some(d.launch_pre_run),
            pre_run_with_game: Some(d.pre_run_with_game),
            launch_post_run: Some(d.launch_post_run),
            pre_run_arg: Some(d.pre_run_arg),
            post_run_arg: Some(d.post_run_arg),
        }
    }
}
