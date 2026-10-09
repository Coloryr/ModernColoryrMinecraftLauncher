//! 添加实例窗口的加载器查询：可选加载器 / 版本列表 / 支持列表（带单飞缓存与重试）
//!
//! 从 `add/mod.rs` 拆出来的（`add_get_support_loaders` 62 行）。命令带
//! `#[gui_macros::ipc_group("add")]` 把组键钉回 `add`（AGENTS.md §4）。

use std::collections::HashMap;
use std::sync::Arc;

use mml_game::add_game::PackType;
use mml_game::gui_hook::{IProgressGui, ProgressGui};
use mml_game::loader::LoaderType;
use tauri::{AppHandle, Emitter};

use crate::dtos::LoaderProgressDto;
use crate::listens;

/// 前端加载器 ID -> LoaderType（ID 列表见 [`add_get_loaders`]）
pub(super) fn parse_loader(id: &str) -> Result<LoaderType, String> {
    LoaderType::from_string(id).ok_or_else(|| "err.unknownLoader".to_string())
}

/// 前端压缩包 ID -> PackType（ID 列表见 [`add_get_pack_types`]）
pub(super) fn parse_pack_type(id: &str) -> Result<PackType, String> {
    PackType::from_id(id).ok_or_else(|| "err.unknownPackType".to_string())
}

/// 获取加载器的可用版本列表（添加实例窗口的加载器版本下拉）
///
/// - `loader`: 加载器独立 ID（见 [`add_get_loaders`]）
/// - `mc`: 游戏版本号（Forge 的 BMCLAPI 源、OptiFine / LiteLoader 需要按版本过滤）
#[gui_macros::ipc_group("add")]
#[tauri::command]
pub async fn add_get_loader_versions(loader: String, mc: String) -> Result<Vec<String>, String> {
    let loader = parse_loader(&loader)?;
    mml_game::loader::loader_versions::get_loader_versions(&loader, &mc)
        .await
        .map_err(|e| e.to_string())
}

/// 获取加载器 ID 列表（添加实例窗口的加载器下拉）
#[gui_macros::ipc_group("add")]
#[tauri::command]
pub fn add_get_loaders() -> Vec<&'static str> {
    LoaderType::ids()
}

/// 加载器支持列表查询进度事件（前端弹窗显示进度条）
#[gui_macros::emit]
pub(super) fn emit_add_loader_progress(app: &AppHandle, dto: LoaderProgressDto) {
    let _ = app.emit(listens::ADD_LOADER_PROGRESS, dto);
}

/// 加载器支持列表查询进度回调：把步数发到前端弹窗（进度条）
pub(super) struct SupportLoadersProgressGui {
    app: AppHandle,
}

impl IProgressGui for SupportLoadersProgressGui {
    fn set_progress_text(&self, _text: Option<String>) {}

    /// 把步数转发为进度事件
    fn set_progress_now(&self, value: usize, all: Option<usize>) {
        emit_add_loader_progress(
            &self.app,
            LoaderProgressDto {
                step: value as u32,
                total: all.unwrap_or(0) as u32,
            },
        );
    }
}

/// 查询指定游戏版本支持的加载器 ID 列表（选中版本后调用）
///
/// 全局查询：结果按版本号缓存（主窗口 / 添加实例窗口共用），
/// 同版本并发查询单飞共享，每查完一个加载器发一次进度事件（前端显示进度条）。
///
/// **中断自动重试**：用户改代理时 `mml_net` 会中断全部在途请求，这一次查询随之失败。
/// 此时**自动用新客户端重跑一次**——因为旧请求绑定的是发出时那份客户端，
/// 只有重新发起才会走新代理。最多重试 [`RETRY_ON_ABORT`] 次，避免代理不通时死循环。
#[gui_macros::ipc_group("add")]
#[tauri::command]
pub async fn add_get_support_loaders(app: AppHandle, mc: String) -> Result<Vec<String>, String> {
    use std::sync::LazyLock;
    use tokio::sync::{Mutex, OnceCell};

    /// 因"中断"（改代理）而失败时允许的重试次数
    const RETRY_ON_ABORT: usize = 2;

    /// 单个版本的查询单飞单元（并发请求共享同一次查询）
    type LoaderQueryCell = Arc<OnceCell<Vec<String>>>;
    /// 「版本号 → 查询单飞单元」缓存类型
    type LoaderQueryCache = LazyLock<Mutex<HashMap<String, LoaderQueryCell>>>;

    /// 支持列表查询结果缓存（版本号 -> 查询单飞单元）
    static SUPPORT_LOADERS_CACHE: LoaderQueryCache = LazyLock::new(|| Mutex::new(HashMap::new()));

    let mut attempt = 0usize;
    loop {
        // 取/建该版本的查询单元：并发请求共享同一次查询
        let cell = {
            let mut cache = SUPPORT_LOADERS_CACHE.lock().await;
            cache.entry(mc.clone()).or_default().clone()
        };
        let gui_app = app.clone();
        let mc_clone = mc.clone();
        let result = cell
            .get_or_try_init(|| async move {
                let gui: ProgressGui =
                    Some(Arc::new(SupportLoadersProgressGui { app: gui_app })
                        as Arc<dyn IProgressGui>);
                mml_game::loader::loader_versions::get_support_loaders(&mc_clone, gui).await
            })
            .await;

        match result {
            Ok(list) => return Ok(list.clone()),
            Err(e) => {
                // 失败不缓存：把这个单元摘掉，下次查询重新发起（拿到新代理 / 重新联网）
                {
                    let mut cache = SUPPORT_LOADERS_CACHE.lock().await;
                    // 只摘掉还是同一个单元的那条：期间可能已被别的请求换成新的，别误删
                    if cache
                        .get(&mc)
                        .is_some_and(|current| Arc::ptr_eq(current, &cell))
                    {
                        cache.remove(&mc);
                    }
                }

                // 被中断（用户改了代理）→ 用新客户端重跑一次
                if mml_net::is_aborted(&e) && attempt < RETRY_ON_ABORT {
                    attempt += 1;
                    mml_log::info(format!(
                        "加载器查询被中断，用新客户端重试（第 {attempt} 次）：mc={mc}"
                    ));
                    continue;
                }

                return Err(e.to_string());
            }
        }
    }
}

/// 获取压缩包类型 ID 列表（添加实例窗口的整合包类型下拉）
#[gui_macros::ipc_group("add")]
#[tauri::command]
pub fn add_get_pack_types() -> Vec<&'static str> {
    PackType::ids()
}

/// 获取游戏版本类型列表（添加实例窗口的版本类型下拉，release / snapshot / old_beta / old_alpha）
#[gui_macros::ipc_group("add")]
#[tauri::command]
pub fn add_get_version_types() -> Vec<&'static str> {
    mml_game::get_version_types()
}
