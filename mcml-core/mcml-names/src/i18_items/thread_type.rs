#[derive(Clone, Debug)]
pub enum ThreadType {
    LogThread,
    ConfigSaveThread,
    LanClientV4,
    LanClientV6,
    LanServer,
}