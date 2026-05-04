#[derive(Clone, Debug)]
pub enum PanicType {
    CoreArgLocalEmpty,
    CoreArgLocalError(String),
    LogOpenFail(String, String)
}