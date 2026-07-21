use std::{
    fs::File,
    future::Future,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use mcml_base::{
    file_item::{FileHash, LaterRun},
    hash_helper::{self, HashType},
    path_helper,
};
use mcml_names::i18_items::error_type::{
    CoreResult, DownloadFileHashErrorData, DownloadFileOverFailData, DownloadFileSizeErrorData,
    ErrorData,
    ErrorType::{self, StreamError},
};
use reqwest::Response;
use semka::Sem;

use crate::{DownloadObj, download_item::DownloadItemState, later_tasks};

/// 用于在下载线程中执行异步任务的运行时
static RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

/// 在当前线程中阻塞执行异步任务
fn block_on<F: Future>(f: F) -> F::Output {
    RT.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
    })
    .block_on(f)
}

pub struct DownloadThread {
    handle: Option<JoinHandle<()>>,
    sem: Arc<Sem>,
    is_stop: Arc<AtomicBool>,
}

impl DownloadThread {
    pub fn new(index: u32) -> Self {
        let sem = Arc::new(Sem::new(0).unwrap());
        let is_stop = Arc::new(AtomicBool::new(false));

        let sem_clone = Arc::clone(&sem);
        let is_stop_clone = Arc::clone(&is_stop);

        let handle = thread::spawn(move || {
            loop {
                if is_stop_clone.load(Ordering::SeqCst) {
                    break;
                }

                sem_clone.wait();

                if is_stop_clone.load(Ordering::SeqCst) {
                    break;
                }

                let item = crate::get_item();
                if let Some(item) = item {
                    download(index, item);
                }
            }
        });

        Self {
            handle: Some(handle),
            sem,
            is_stop,
        }
    }

    // 开始下载
    pub fn run(&self) {
        self.sem.signal();
    }

    // 停止线程
    pub fn stop(&mut self) {
        self.is_stop.store(true, Ordering::SeqCst);
        self.sem.signal();

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn download(index: u32, mut obj: DownloadObj) {
    if obj.item.base.file.exists() {
        if obj.item.overwrite {
            if let Err(err) = path_helper::delete(&obj.item.base.file) {
                mcml_log::error_type(ErrorType::DownloadFileOverFail(DownloadFileOverFailData {
                    file: obj.item.base.file.clone(),
                    error: Box::new(err),
                }));
                obj.task.fail();
                return;
            }
        } else {
            let config = mcml_config::read_config();
            let mut file = path_helper::open_read(&obj.item.base.file).unwrap();
            if config.http.check_file {
                let check = check_hash(&obj.item.base.file, &obj.item.base.hash, &mut file);
                if let Err(err) = check {
                    mcml_log::error_type(err);
                } else {
                    obj.task.done();
                    return;
                }
            }
        }
    }

    let mut times = 0;
    let mut use_break = false;
    let mut server_ranges = true;
    let mut is_keep = false;

    fn is_need_err(err: ErrorType, times: &mut i32) -> bool {
        *times += 1;
        mcml_log::error_type(err);
        if *times > 5 {
            return true;
        }

        return false;
    }

    let mut temp_file;

    loop {
        temp_file = crate::gen_temp_file();

        let file = if is_keep {
            path_helper::open_append(&temp_file)
        } else {
            path_helper::open_write(&temp_file)
        };

        if let Err(err) = file {
            obj.item.add_error();
            if is_need_err(err, &mut times) {
                break;
            } else {
                continue;
            }
        }

        let mut file = file.unwrap();

        let mut resp = if use_break && server_ranges {
            let result = block_on(
                mcml_net::get_work_client().get_ranges(&obj.item.base.url, obj.item.get_now_size()),
            );
            if let Err(err) = result {
                obj.item.add_error();
                if is_need_err(err, &mut times) {
                    break;
                } else {
                    continue;
                }
            }

            let resp = result.unwrap();

            if !resp.status().is_success() {
                server_ranges = false;
                continue;
            }

            is_keep = true;

            resp
        } else {
            let result = block_on(mcml_net::get_work_client().get(&obj.item.base.url));
            if let Err(err) = result {
                obj.item.add_error();
                if is_need_err(err, &mut times) {
                    break;
                } else {
                    continue;
                }
            }

            let resp = result.unwrap();

            obj.item.set_now_size(0);
            obj.item.set_all_size(match resp.content_length() {
                Some(data) => data,
                None => 0,
            });

            resp
        };

        if let Some(range) = resp.headers().get("Accept-Ranges")
            && range.to_str().unwrap().starts_with("bytes")
        {
            use_break = true;
        }

        obj.item.set_state(DownloadItemState::GetInfo);
        crate::update(index, &obj.item);

        let result = block_on(write_file(index, &mut obj, &mut resp, &mut file));
        if let Err(err) = result {
            obj.item.add_error();
            if is_need_err(err, &mut times) {
                break;
            } else {
                continue;
            }
        }

        break;
    }

    path_helper::move_file(&temp_file, &obj.item.base.file).unwrap();

    match &obj.item.base.later {
        LaterRun::None => {}
        LaterRun::UnpackNative(path_buf) => {
            let file = path_helper::open_read(&obj.item.base.file).unwrap();
            later_tasks::unpack_native(path_buf, file).unwrap();
        }
    }

    obj.item.set_state(DownloadItemState::Done);
    crate::update(index, &obj.item);
    obj.task.done();
}

async fn write_file(
    index: u32,
    obj: &mut DownloadObj,
    resp: &mut Response,
    file: &mut File,
) -> CoreResult<()> {
    loop {
        match resp.chunk().await {
            Ok(None) => break,
            Ok(Some(data)) => {
                // 写入文件
                file.write_all(&data).map_err(|err| {
                    StreamError(ErrorData {
                        error: err.to_string(),
                    })
                })?;

                obj.item.set_state(DownloadItemState::Download);
                obj.item.add_progress(data.len() as u64);

                crate::update(index, &obj.item);
            }
            Err(e) => {
                return Err(StreamError(ErrorData {
                    error: e.to_string(),
                }));
            }
        }
    }

    let config = mcml_config::read_config();
    if config.http.check_file {
        let now = file.stream_position().unwrap();
        if now != obj.item.get_all_size() {
            obj.item.set_state(DownloadItemState::Error);

            crate::update(index, &obj.item);

            return Err(ErrorType::DownloadFileSizeError(
                DownloadFileSizeErrorData {
                    file: obj.item.base.file.clone(),
                    url: obj.item.base.url.clone(),
                    now: now,
                    size: obj.item.get_all_size(),
                },
            ));
        }

        file.seek(SeekFrom::Start(0)).unwrap();
        return check_hash(&obj.item.base.file, &obj.item.base.hash, file);
    }

    Ok(())
}

/// 检查文件
fn check_hash<R: Read + Seek>(file: &PathBuf, hash: &FileHash, stream: &mut R) -> CoreResult<()> {
    match hash {
        FileHash::None => Ok(()),
        FileHash::Md5(md5) => match hash_helper::gen_hash_from_reader(HashType::Md5, stream) {
            Ok(hash) => {
                if hash.eq_ignore_ascii_case(md5) {
                    Ok(())
                } else {
                    Err(ErrorType::DownloadFileHashError(
                        DownloadFileHashErrorData {
                            file: file.clone(),
                            now: hash.clone(),
                            hash: md5.clone(),
                        },
                    ))
                }
            }
            Err(err) => Err(err),
        },
        FileHash::Sha1(sha1) => match hash_helper::gen_hash_from_reader(HashType::Sha1, stream) {
            Ok(hash) => {
                if hash.eq_ignore_ascii_case(sha1) {
                    Ok(())
                } else {
                    Err(ErrorType::DownloadFileHashError(
                        DownloadFileHashErrorData {
                            file: file.clone(),
                            now: hash.clone(),
                            hash: sha1.clone(),
                        },
                    ))
                }
            }
            Err(err) => Err(err),
        },
        FileHash::Sha256(sha256) => {
            match hash_helper::gen_hash_from_reader(HashType::Sha256, stream) {
                Ok(hash) => {
                    if hash.eq_ignore_ascii_case(sha256) {
                        Ok(())
                    } else {
                        Err(ErrorType::DownloadFileHashError(
                            DownloadFileHashErrorData {
                                file: file.clone(),
                                now: hash.clone(),
                                hash: sha256.clone(),
                            },
                        ))
                    }
                }
                Err(err) => Err(err),
            }
        }
        FileHash::Sha1Sha256(sha1, sha256) => {
            let sha1 = match hash_helper::gen_hash_from_reader(HashType::Sha1, stream) {
                Ok(hash) => {
                    if hash.eq_ignore_ascii_case(sha1) {
                        Ok(())
                    } else {
                        Err(ErrorType::DownloadFileHashError(
                            DownloadFileHashErrorData {
                                file: file.clone(),
                                now: hash.clone(),
                                hash: sha1.clone(),
                            },
                        ))
                    }
                }
                Err(err) => Err(err),
            };

            if sha1.is_err() {
                return sha1;
            }

            stream.seek(SeekFrom::Start(0)).map_err(|err| {
                ErrorType::StreamError(ErrorData {
                    error: err.to_string(),
                })
            })?;

            let sha256 = match hash_helper::gen_hash_from_reader(HashType::Sha256, stream) {
                Ok(hash) => {
                    if hash.eq_ignore_ascii_case(sha256) {
                        Ok(())
                    } else {
                        Err(ErrorType::DownloadFileHashError(
                            DownloadFileHashErrorData {
                                file: file.clone(),
                                now: hash.clone(),
                                hash: sha256.clone(),
                            },
                        ))
                    }
                }
                Err(err) => Err(err),
            };

            if sha256.is_err() {
                return sha256;
            }

            Ok(())
        }
        FileHash::Sha512(sha512) => {
            match hash_helper::gen_hash_from_reader(HashType::Sha512, stream) {
                Ok(hash) => {
                    if hash.eq_ignore_ascii_case(sha512) {
                        Ok(())
                    } else {
                        Err(ErrorType::DownloadFileHashError(
                            DownloadFileHashErrorData {
                                file: file.clone(),
                                now: hash.clone(),
                                hash: sha512.clone(),
                            },
                        ))
                    }
                }
                Err(err) => Err(err),
            }
        }
        FileHash::Sha1Sha512(sha1, sha512) => {
            let sha1 = match hash_helper::gen_hash_from_reader(HashType::Sha1, stream) {
                Ok(hash) => {
                    if hash.eq_ignore_ascii_case(sha1) {
                        Ok(())
                    } else {
                        Err(ErrorType::DownloadFileHashError(
                            DownloadFileHashErrorData {
                                file: file.clone(),
                                now: hash.clone(),
                                hash: sha1.clone(),
                            },
                        ))
                    }
                }
                Err(err) => Err(err),
            };

            if sha1.is_err() {
                return sha1;
            }

            stream.seek(SeekFrom::Start(0)).map_err(|err| {
                ErrorType::StreamError(ErrorData {
                    error: err.to_string(),
                })
            })?;

            let sha512 = match hash_helper::gen_hash_from_reader(HashType::Sha512, stream) {
                Ok(hash) => {
                    if hash.eq_ignore_ascii_case(sha512) {
                        Ok(())
                    } else {
                        Err(ErrorType::DownloadFileHashError(
                            DownloadFileHashErrorData {
                                file: file.clone(),
                                now: hash.clone(),
                                hash: sha512.clone(),
                            },
                        ))
                    }
                }
                Err(err) => Err(err),
            };

            if sha512.is_err() {
                return sha512;
            }

            Ok(())
        }
    }
}
