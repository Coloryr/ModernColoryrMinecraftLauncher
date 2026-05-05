#[derive(Clone, Debug)]
pub enum ErrorType {
    ConfigSaveError(String, String),
    ConfigReadError(String, String),
    AdoptiumGetError(String),
    ColoryrApiGetError(String),
    ColoryrApiServerError(i32),
    HttpReqError(String),
    JsonDecError(String)
}
