use std::{
    fs::File,
    future::Future,
    io::{Read, Seek, SeekFrom, Write},
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use mcml_base::{
    file_item::FileHash,
    hash_helper::{self, HashType},
    path_helper,
};
use mcml_names::i18_items::error_type::{
    CoreResult, DownloadFileOverFailData, DownloadFileSizeErrorData, ErrorData,
    ErrorType::{self, StreamError},
};
use mcml_net::WORK_CLIENT;
use reqwest::Response;
use semka::Sem;

use crate::{DownloadObj, download_item::DownloadItemState, gen_temp_file, get_item, update};

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
    index: u32,
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

                let item = get_item();
                if let Some(item) = item {
                    download(index, item);
                }
            }
        });

        Self {
            index,
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
            let config = mcml_config::CONFIG.get().unwrap().read().unwrap();
            let mut file = path_helper::open_read(&obj.item.base.file).unwrap();
            if config.http.check_file && check_hash(&obj.item.base.hash, &mut file) {
                obj.task.done();
                return;
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

    loop {
        let temp_file = gen_temp_file();
        let file = if is_keep {
            path_helper::open_append(&temp_file)
        } else {
            path_helper::open_write(&temp_file)
        };

        if let Err(err) = file {
            if is_need_err(err, &mut times) {
                break;
            } else {
                continue;
            }
        }

        let mut file = file.unwrap();

        let mut resp = if use_break && server_ranges {
            let result = block_on(
                WORK_CLIENT
                    .get()
                    .unwrap()
                    .get_ranges(&obj.item.base.url, obj.item.get_now_size()),
            );
            if let Err(err) = result {
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
            let result = block_on(WORK_CLIENT.get().unwrap().get(&obj.item.base.url));
            if let Err(err) = result {
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

        let result = block_on(write_file(index, &mut obj, &mut resp, &mut file));
    }
}

async fn write_file(
    index: u32,
    obj: &mut DownloadObj,
    resp: &mut Response,
    file: &mut File,
) -> CoreResult<bool> {
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

                update(index, &obj.item);
            }
            Err(e) => {
                return Err(StreamError(ErrorData {
                    error: e.to_string(),
                }));
            }
        }
    }

    let config = mcml_config::CONFIG.get().unwrap().read().unwrap();
    if config.http.check_file {
        let now = file.stream_position().unwrap();
        if now != obj.item.get_all_size() {
            obj.item.set_state(DownloadItemState::Error);

            update(index, &obj.item);

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
        return Ok(check_hash(&obj.item.base.hash, file));
    }

    Ok(true)
}

fn check_hash<R: Read + Seek>(hash: &FileHash, stream: &mut R) -> bool {
    match hash {
        FileHash::None => true,
        FileHash::Md5(md5) => {
            if let Ok(hash) = hash_helper::gen_hash_from_reader(HashType::Md5, stream) {
                hash.eq_ignore_ascii_case(md5)
            } else {
                false
            }
        }
        FileHash::Sha1(sha1) => {
            if let Ok(hash) = hash_helper::gen_hash_from_reader(HashType::Sha1, stream) {
                hash.eq_ignore_ascii_case(sha1)
            } else {
                false
            }
        }
        FileHash::Sha256(sha256) => {
            if let Ok(hash) = hash_helper::gen_hash_from_reader(HashType::Sha256, stream) {
                hash.eq_ignore_ascii_case(sha256)
            } else {
                false
            }
        }
        FileHash::Sha1Sha256(sha1, sha256) => {
            if let Ok(hash) = hash_helper::gen_hash_from_reader(HashType::Sha1, stream)
                && hash.eq_ignore_ascii_case(sha1)
            {
                stream.seek(SeekFrom::Start(0)).unwrap();
                if let Ok(hash) = hash_helper::gen_hash_from_reader(HashType::Sha256, stream) {
                    hash.eq_ignore_ascii_case(sha256)
                } else {
                    false
                }
            } else {
                false
            }
        }
    }
}
