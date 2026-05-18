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
    pub status: u16
}

/// 配置文件保存时错误信息
#[derive(Clone, Debug)]
pub struct JsonErrorData {
    pub error: String,
}

/// 文件找不到
#[derive(Clone, Debug)]
pub struct FileNotExistsData {
    pub file: String,
}

/// OAuth处理错误
#[derive(Clone, Debug)]
pub struct OAuthErrorData {
    pub error: String,
}

/// 文件系统错误
#[derive(Clone, Debug)]
pub struct FileSystemErrorData {
    pub dir: String,
    pub error: String,
}

#[derive(Clone, Debug)]
pub struct ZipErrorData {
    pub source: String,
    pub target: String,
    pub error: String,
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
    FileNotExists(FileNotExistsData),

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
    /// 登录密钥过期
    AuthTokenTimeout,

    /// OAuth服务器密钥未设置
    OAuthKeyIsNull,
    /// OAuth标识请求超时
    OAuthTokenTimeout,
    /// OAuth获取登录码错误
    OAuthGetTokenError(OAuthErrorData),
    /// OAuth获取不到登录码
    OAuthGetTokenEmpty,

    /// 文件系统处理错误
    FileSystemError(FileSystemErrorData),

    /// 压缩文件处理错误
    ZipError(ZipErrorData),

    TaskCancel,
    TaskTimeout,

    GetVersionMetaFail,
}
