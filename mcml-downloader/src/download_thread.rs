pub struct DownloadThread {
    index: u32,
}

impl DownloadThread {
    pub fn new(index: u32) -> Self {
        Self { index }
    }

    pub fn download_stop(&self) {}
}
