use std::{path::PathBuf, sync::atomic::AtomicUsize};

use mcml_names::i18_items::error_type::{ArchiveErrorData, ErrorType};

use crate::archives::IArchiveGui;

pub struct R7zProcess {
    gui: Option<Box<dyn IArchiveGui + Send + Sync>>,
    size: AtomicUsize,
    now: AtomicUsize,
}

impl R7zProcess {
    pub fn new(gui: Option<Box<dyn IArchiveGui + Send + Sync>>) -> Self {
        Self {
            gui,
            size: AtomicUsize::new(0),
            now: AtomicUsize::new(0),
        }
    }

    // /// 将整个目录压缩为 7z 文件
    // pub fn compress(
    //     &self,
    //     zip_file: &PathBuf,
    //     pack_dir: &PathBuf,
    //     root_path: &PathBuf,
    //     filter: &Option<Vec<String>>,
    // ) -> Result<(), ErrorType> {
    //     compress_to_path(src_dir, dst_path).map_err(|e| {
    //         ErrorType::ArchiveError(ArchiveErrorData {
    //             source: src_dir.to_string_lossy().to_string(),
    //             target: dst_path.to_string_lossy().to_string(),
    //             error: e.to_string(),
    //         })
    //     })
    // }

    // /// 解压 7z 文件到指定目录
    // pub fn decompress(
    //     &self, archive_file: &PathBuf, output_dir: &PathBuf,
    // ) -> Result<(), ErrorType> {
    //     match password {
    //         Some(pwd) => decompress_file_with_password(src_path, dst_dir, pwd).map_err(|e| {
    //             ErrorType::ArchiveError(ArchiveErrorData {
    //                 source: src_path.to_string_lossy().to_string(),
    //                 target: dst_dir.to_string_lossy().to_string(),
    //                 error: e.to_string(),
    //             })
    //         }),
    //         None => decompress_file(src_path, dst_dir).map_err(|e| {
    //             ErrorType::ArchiveError(ArchiveErrorData {
    //                 source: src_path.to_string_lossy().to_string(),
    //                 target: dst_dir.to_string_lossy().to_string(),
    //                 error: e.to_string(),
    //             })
    //         }),
    //     }
    // }
}
