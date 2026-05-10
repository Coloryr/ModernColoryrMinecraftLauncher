#[derive(Clone, Debug)]
pub enum ErrorType {
    ConfigSaveError(String, String),
    ConfigReadError(String, String),
    ColoryrApiGetError(String),
    ColoryrApiServerError(i32),
    HttpReqError(String),
    HttpReadError(String),
    JsonDecError(String),
    FileNotExists(String),
    LoginFail(String),
    LoginNoProfile
}
