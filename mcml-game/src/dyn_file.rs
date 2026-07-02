use std::{
    io::Read,
    path::{Path, PathBuf},
};

use mcml_base::path_helper;
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use tokio::io::{AsyncRead, AsyncWriteExt};

/// 输入文件
pub enum InputFile {
    /// 实际存在的文件
    Path(PathBuf),
    /// 网络文件
    Url(String),
    /// 数据
    Data(Vec<u8>),
    /// 同步流
    Stream(Box<dyn Read>),
    /// 异步流
    StreamAsync(Box<dyn AsyncRead + Unpin>),
}

impl InputFile {
    /// 保存到文件
    /// - `path`: 需要保存的路径
    pub async fn save_file<P: AsRef<Path>>(self, path: P) -> CoreResult<()> {
        match self {
            InputFile::Path(path_buf) => {
                path_helper::copy_file_async(path_buf, path.as_ref().to_path_buf()).await?;
            }
            InputFile::Url(url) => {
                let mut stream = mcml_net::get_work_client().get(&url).await?;
                let mut file = path_helper::open_write_async(path.as_ref()).await?;

                loop {
                    match stream.chunk().await {
                        Ok(None) => break,
                        Ok(Some(data)) => {
                            // 写入文件
                            file.write_all(&data).await.map_err(|err| {
                                ErrorType::StreamError(ErrorData {
                                    error: err.to_string(),
                                })
                            })?;
                        }
                        Err(e) => {
                            return Err(ErrorType::StreamError(ErrorData {
                                error: e.to_string(),
                            }));
                        }
                    }
                }
            }
            InputFile::Data(items) => {
                path_helper::write_bytes_async(path.as_ref(), &items).await?;
            }
            InputFile::Stream(read) => {
                path_helper::write_stream(path.as_ref(), read)?;
            }
            InputFile::StreamAsync(async_read) => {
                path_helper::write_stream_async(path.as_ref(), async_read).await?;
            }
        }

        Ok(())
    }
}
