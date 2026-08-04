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
pub struct PathNotExistsData {
    pub path: PathBuf,
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
pub enum ArgEmptyData {
    /// 名字参数为空
    Name,
    /// 标识
    UUID,
    /// 版本
    Version,
}

#[derive(Clone, Debug)]
pub enum ArgErrorData {
    ArchiveType
}

#[derive(Clone, Debug)]
pub enum DataNotFoundData {
    /// 信息
    Info,
    /// 注册表
    RegistryKey(String),
    /// 网址
    Url,
    /// 游戏实例
    GameInstance,
    /// 游戏版本
    Version(String),
}

/// mcml执行结果
pub type CoreResult<T> = result::Result<T, ErrorType>;

/// mcml错误类型
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
    FileNotExists(PathNotExistsData),
    /// 目录不存在
    DirNotExists(PathNotExistsData),

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

    /// NBT类型错误
    NbtTypeError,
    /// NBT读取失败
    NbtReadError,

    /// 输入参数为空
    ArgEmpty(ArgEmptyData),
    /// 输入参数错误
    ArgError(ArgErrorData),
    /// 所需文件未能找到
    DataNotFound(DataNotFoundData),
    /// 找不到合适的Java
    JavaNotFound,

    /// 下载文件覆盖错误
    DownloadFileOverFail(DownloadFileOverFailData),
    /// 下载文件的预期大小不符合
    DownloadFileSizeError(DownloadFileSizeErrorData),
    /// 下载文件校验失败
    DownloadFileHashError(DownloadFileHashErrorData),
    /// 文件下载失败
    DownloadFileFail,

    /// 错误的操作
    InvalidOperation,

    /// Socket处理出错
    SocketError(ErrorData),
    /// 线程启动错误
    ThreadError(ErrorData),
    /// 进程启动错误
    ProcessError(ErrorData),
    /// 版本号错误
    InstanceVersionError,
    /// BASE64错误
    Base64Error(ErrorData),
    /// 流处理异常
    StreamError(ErrorData),

    /// 密钥未设置
    KeyIsNull,
}
