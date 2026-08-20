use std::{path::PathBuf, sync::LazyLock};

use clipboard_rs::{Clipboard, ClipboardContext};

static CONTEXT: LazyLock<ClipboardContext> = LazyLock::new(|| ClipboardContext::new().unwrap());

/// 将文本复制到剪贴板
pub fn copy_text(text: &str) {
    _ = CONTEXT.set_text(text.to_string());
}

/// 将文本复制到剪贴板
pub fn copy_files(files: Vec<PathBuf>) {
    _ = CONTEXT.set_files(
        files
            .iter()
            .map(|item| item.to_string_lossy().to_string())
            .collect(),
    );
}
