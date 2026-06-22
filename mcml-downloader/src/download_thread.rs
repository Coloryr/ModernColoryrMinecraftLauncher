use std::{
    io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use mcml_base::{
    file_item::{FileHash, FileItemObj},
    hash_helper::{self, HashType},
    path_helper,
};
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType::{self, StreamError}};
use semka::Sem;

use crate::{DownloadObj, download_item::DownloadItemState, get_item, update};

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
                if let Some(item) = item {}
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

fn write_file<R: Read + Seek>(
    index: u32,
    file: &mut DownloadObj,
    stream: &mut R,
    keep: bool,
) -> CoreResult<bool> {
    let mut file_handle = if keep {
        path_helper::open_append(&file.item.base.file)?
    } else {
        path_helper::open_write(&file.item.base.file)?
    };

    const BUF_SIZE: usize = 8192;
    let mut buffer = vec![0u8; BUF_SIZE];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                // 写入文件
                file_handle.write_all(&buffer[..n]).map_err(|err| {
                    StreamError(ErrorData {
                        error: err.to_string(),
                    })
                })?;

                file.item.set_state(DownloadItemState::Download);
                file.item.add_progress(n as u64);

                update(index, &file.item);
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
        if file_handle.stream_position().unwrap() != file.item.get_all_size() {
            file.item.set_state(DownloadItemState::Error);

            update(index, &file.item);

            return Err(ErrorType::FileDownloadError);
        }
    }

    Ok(false)
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
