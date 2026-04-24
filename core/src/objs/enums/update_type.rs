pub enum UpdateType {
    /// 添加下载项目
    /// 添加了多少个任务
    AddItem(i32),
    /// 下载项目完成
    ItemDone
}