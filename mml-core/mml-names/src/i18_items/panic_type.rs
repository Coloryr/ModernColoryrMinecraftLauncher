#[derive(Clone, Debug)]
pub enum PanicType {
    CoreArgLocalEmpty,
    CoreArgLocalError,
    LogOpenFail(String, String)
}