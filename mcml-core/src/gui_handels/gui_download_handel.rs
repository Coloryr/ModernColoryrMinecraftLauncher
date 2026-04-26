use crate::net::downloader::download_item::DownloadItem;

pub enum UpdateType {
    /// 添加下载项目
    /// 添加了多少个任务
    AddItem(i32),
    /// 下载项目完成
    ItemDone
}

pub trait DownloadGuiHandel: Send + Sync {
    /// 下载器更新
    /// thread 下载线程
    /// state 是否在下载
    /// count 下载任务总数
    fn update(&self, thread: i32, state: bool, count: i32);

    /// 下载任务更新
    /// task_type 更新类型
    fn update_task(&self, task_type: UpdateType);

    /// 下载项目状态更新
    /// thread 下载线程
    /// file 下载项目 
    fn update_item(&self, thread: i32, file: &DownloadItem);
}
