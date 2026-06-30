use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self},
    },
    thread,
    time::Duration,
};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileSystemErrorData};
use notify::{
    EventKind, RecursiveMode, Watcher,
    event::{CreateKind, RemoveKind},
};

use crate::launcher_path::instance_path;

static ENABLE_WATCHER: AtomicBool = AtomicBool::new(false);

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
                                if create_kind == CreateKind::Folder {

                                }
                            }
                            EventKind::Remove(remove_kind) => {
                                if remove_kind == RemoveKind::Folder {
                                    
                                }
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        mcml_log::error_type(ErrorType::FileSystemError(FileSystemErrorData {
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

/// 开始监听
pub(crate) fn start_watch() {
    ENABLE_WATCHER.store(true, Ordering::Release);
}

/// 停止监听
pub(crate) fn stop_watch() {
    ENABLE_WATCHER.store(true, Ordering::Release);
}
