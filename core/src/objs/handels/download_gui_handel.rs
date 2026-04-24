use crate::objs::{enums::update_type::UpdateType, file_item_obj::FileItemObj};

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
    fn update_item(&self, thread: i32, file: &FileItemObj);
}
