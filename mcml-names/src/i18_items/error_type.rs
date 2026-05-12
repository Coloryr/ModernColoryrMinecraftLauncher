/// 配置文件保存时错误信息
#[derive(Clone, Debug)]
pub struct ConfigErrorData {
    pub file: String,
    pub error: String,
}

/// 配置文件保存时错误信息
#[derive(Clone, Debug)]
pub struct HttpReqErrorData {
    pub url: String,
    pub error: String,
}

/// 配置文件保存时错误信息
#[derive(Clone, Debug)]
pub struct HttpReadErrorData {
    pub url: String,
    pub error: String,
}

/// 配置文件保存时错误信息
#[derive(Clone, Debug)]
pub struct JsonErrorData {
    pub error: String,
}

/// 配置文件保存时错误信息
#[derive(Clone, Debug)]
pub struct FileErrorData {
    pub file: String,
}

#[derive(Clone, Debug)]
pub enum ErrorType {
    /// 配置文件保存时出错
    ConfigSaveError(ConfigErrorData),
    /// 配置文件读取时出错
    ConfigReadError(ConfigErrorData),

    /// Http请求出错
    HttpReqError(HttpReqErrorData),
    /// Http请求出错
    HttpReadError(HttpReadErrorData),

    /// Json处理错误
    JsonError(JsonErrorData),

    /// 文件不存在
    FileNotExists(FileErrorData),

    /// 登录返回数据错误
    AuthDataError(String),
    /// 登录错误
    AuthLoginFail(String),
    /// 登录没有账户返回
    AuthLoginNoProfile,
    /// 登录刷新错误
    AuthRefreshFail(String),
    /// 登录刷新没有账户返回
    AuthRefreshNoProfile,

    /// OAuth服务器密钥未设置
    OAuthKeyIsNull,
    /// OAuth标识请求超时
    OAuthTokenTimeout,

    TaskCancel,
    TaskTimeout,
}
