use std::{
    io::{self, Read, Seek},
    path::Path,
};

use mcml_base::path_helper;
use mcml_names::i18_items::error_type::{
    ArchiveErrorData, CoreResult, ErrorData, ErrorType, FileSystemErrorData,
};
use zip::ZipArchive;

/// 解压native库
/// - `native`: 解压的位置
/// - `read`: 压缩包
pub fn unpack_native<R: Read + Seek>(native: &Path, read: R) -> CoreResult<()> {
    let mut archive = ZipArchive::new(read).map_err(|err| {
        ErrorType::ArchiveOpenError(FileSystemErrorData {
            path: Default::default(),
            error: err.to_string(),
        })
    })?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|err| {
            ErrorType::ArchiveReadError(ErrorData {
                error: err.to_string(),
            })
        })?;

        if file.is_dir() {
            continue;
        }

        if let Some(name) = file.enclosed_name()
            && name.starts_with("META-INF")
        {
            let outpath = native.join(name.file_name().unwrap());

            let mut outfile = path_helper::open_write(&outpath)?;
            io::copy(&mut file, &mut outfile).map_err(|err| {
                ErrorType::ArchiveError(ArchiveErrorData {
                    source: file.name().to_string(),
                    target: outpath.display().to_string(),
                    error: err.to_string(),
                })
            })?;
        }
    }

    Ok(())
}
