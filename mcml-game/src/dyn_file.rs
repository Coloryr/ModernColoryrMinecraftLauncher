use std::{
    io::Read,
    path::{Path, PathBuf},
};

use mcml_base::path_helper;
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};
use tokio::io::{AsyncRead, AsyncWriteExt};

pub enum InputFileType {
    Path(PathBuf),
    Url(String),
    Data(Vec<u8>),
    Stream(Box<dyn Read>),
    StreamAsync(Box<dyn AsyncRead + Unpin>),
}

impl InputFileType {
    pub async fn save_file<P: AsRef<Path>>(self, path: P) -> CoreResult<()> {
        match self {
            InputFileType::Path(path_buf) => {
                path_helper::copy_file_async(path_buf, path.as_ref().to_path_buf()).await?;
            }
            InputFileType::Url(url) => {
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
            InputFileType::Data(items) => {
                path_helper::write_bytes_async(path.as_ref(), &items).await?;
            }
            InputFileType::Stream(read) => {
                path_helper::write_stream(path.as_ref(), read)?;
            }
            InputFileType::StreamAsync(async_read) => {
                path_helper::write_stream_async(path.as_ref(), async_read).await?;
            }
        }

        Ok(())
    }
}
