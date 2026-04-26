pub trait ProgressGuiHandel: Send + Sync {
    /// 设置当前进度
    /// now 当前进度
    /// all 所有进度
    fn set_now_progress(&self, now: i64, all: i64);
}
