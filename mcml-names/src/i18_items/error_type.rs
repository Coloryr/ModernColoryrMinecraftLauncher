use std::{path::PathBuf, result};

#[derive(Clone, Debug)]
pub struct ErrorData {
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
    pub status: u16,
}

/// 文件找不到
#[derive(Clone, Debug)]
pub struct FileNotExistsData {
    pub file: PathBuf,
}

/// 文件系统错误
#[derive(Clone, Debug)]
pub struct FileSystemErrorData {
    pub path: PathBuf,
    pub error: String,
}

#[derive(Clone, Debug)]
pub struct ArchiveErrorData {
    pub source: String,
    pub target: String,
    pub error: String,
}

#[derive(Clone, Debug)]
pub struct DownloadFileSizeErrorData {
    pub file: PathBuf,
    pub url: String,
    pub now: u64,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct DownloadFileOverFailData {
    pub file: PathBuf,
    pub error: Box<ErrorType>,
}

#[derive(Clone, Debug)]
pub struct DownloadFileHashErrorData {
    pub file: PathBuf,
    pub now: String,
    pub hash: String,
}

#[derive(Clone, Debug)]
pub struct ArgEmptyData {
    pub arg: String,
}

pub type CoreResult<T> = result::Result<T, ErrorType>;

#[derive(Clone, Debug)]
pub enum ErrorType {
    /// 配置文件保存时出错
    ConfigSaveError(FileSystemErrorData),
    /// 配置文件读取时出错
    ConfigReadError(FileSystemErrorData),

    /// Http请求出错
    HttpReqError(HttpReqErrorData),
    /// Http请求出错
    HttpReadError(HttpReadErrorData),

    /// 序列化处理错误
    SerializerError(ErrorData),

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
    OAuthGetTokenError(ErrorData),
    /// OAuth获取不到登录码
    OAuthGetTokenEmpty,

    /// 文件系统处理错误
    FileSystemError(FileSystemErrorData),
    /// 文件获取错误
    FileReadError(ErrorData),
    /// 文件不存在
    FileNotExists(FileNotExistsData),

    /// 压缩包打开错误
    ArchiveOpenError(FileSystemErrorData),
    /// 压缩包读取错误
    ArchiveReadError(ErrorData),
    /// 压缩文件处理错误
    ArchiveError(ArchiveErrorData),
    /// 压缩文件写错误
    ArchiveWriteError(ErrorData),

    /// 任务取消
    TaskCancel,
    /// 任务执行超时
    TaskTimeout,
    /// 任务出错
    TaskError(ErrorData),

    GetVersionMetaFail,

    /// 流处理异常
    StreamError(ErrorData),

    /// NBT类型错误
    NbtTypeError,
    /// NBT读取失败
    NbtReadError,

    /// 项目不存在
    DataNotFound,

    /// BASE64错误
    Base64Error(ErrorData),

    /// 所需文件未能找到
    InfoNotFound,

    /// 下载文件覆盖错误
    DownloadFileOverFail(DownloadFileOverFailData),
    /// 下载文件的预期大小不符合
    DownloadFileSizeError(DownloadFileSizeErrorData),
    /// 下载文件校验失败
    DownloadFileHashError(DownloadFileHashErrorData),
    /// 文件下载失败
    DownloadFileFail,

    /// 进程启动错误
    ProcessError(ErrorData),

    /// 版本号错误
    VersionInfoError,
    /// 找不到合适的Java
    JavaNotFound,

    /// 输入参数错误
    ArgEmpty(ArgEmptyData),
}
