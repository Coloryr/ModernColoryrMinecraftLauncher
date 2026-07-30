//! 下载后处理模块
//!
//! 处理文件下载完成后的后续操作，如解压 native 库等。

use std::{
    io::{self, Read, Seek},
    path::Path,
};

use mcml_base::path_helper;
use mcml_names::i18_items::error_type::{
    ArchiveErrorData, CoreResult, ErrorData, ErrorType, FileSystemErrorData,
};
use zip::ZipArchive;

/// 解压 Minecraft native 库
///
/// 从下载的 JAR 包中提取 `META-INF` 目录下的原生库文件。
/// 这些文件是 LWJGL 等底层库在各平台上的本地实现（.dll / .so / .dylib）。
///
/// # 参数
///
/// - `native`: 解压目标目录
/// - `read`: 可读取 + 可定位的输入流（通常是下载的 jar 文件）
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

        // 仅提取 META-INF 目录下的文件（native 库的存放位置）
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
