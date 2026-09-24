//! 实例目录文件监视
//!
//! 监听实例根目录的创建 / 删除事件（`notify` crate），
//! 供实例列表在目录被外部改动时刷新。

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self},
    },
    thread,
    time::Duration,
};

use mml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData};
use notify::{
    EventKind, RecursiveMode, Watcher,
    event::{CreateKind, RemoveKind},
};

use crate::launcher_path::instance_path;

/// 监听是否生效开关
static ENABLE_WATCHER: AtomicBool = AtomicBool::new(false);

/// 初始化实例目录监视
///
/// # 返回值
///
/// 成功返回 `Ok(())`；创建监听器失败返回对应错误
pub(crate) fn init_watch() -> CoreResult<()> {
    let (tx, rx) = mpsc::channel();

    let mut watcher = notify::recommended_watcher(tx).map_err(|err| {
        ErrorType::TaskError(ErrorData {
            error: err.to_string(),
        })
    })?;

    watcher
        .watch(
            &instance_path::get_instance_dir(),
            RecursiveMode::NonRecursive,
        )
        .map_err(|err| {
            ErrorType::TaskError(ErrorData {
                error: err.to_string(),
            })
        })?;

    thread::spawn(move || {
        loop {
            for event in &rx {
                match event {
                    Ok(event) => {
                        if !ENABLE_WATCHER.load(Ordering::Acquire) {
                            continue;
                        }

                        match event.kind {
                            EventKind::Create(create_kind) => {
                                if create_kind == CreateKind::Folder {}
                            }
                            EventKind::Remove(remove_kind) => {
                                if remove_kind == RemoveKind::Folder {}
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        mml_log::error_type(ErrorType::FileSystemError(FileSystemErrorData {
                            path: instance_path::get_instance_dir(),
                            error: e.to_string(),
                        }));
                    }
                }
            }

            thread::sleep(Duration::from_secs(1));
        }
    });

    Ok(())
}

/// 开始监听（实例创建 / 删除期间暂停，完成后恢复）
pub(crate) fn start_watch() {
    ENABLE_WATCHER.store(true, Ordering::Release);
}

/// 停止监听
pub(crate) fn stop_watch() {
    ENABLE_WATCHER.store(false, Ordering::Release);
}
