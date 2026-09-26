use std::{path::PathBuf, result};

use crate::i18_items::panic_type::PanicType;

#[derive(Clone, Debug)]
pub struct ErrorData {
    pub error: String,
}

/// HTTP请求错误信息（status = 响应状态码，请求阶段失败时为None）
#[derive(Clone, Debug)]
pub struct HttpErrorData {
    pub url: String,
    pub error: String,
    pub status: Option<u16>,
}

/// 路径找不到
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

/// 皮肤方块错误
#[derive(Clone, Debug)]
pub enum SkinBlockErrorData {
    /// 名字非法（只允许英文字母数字-_，≤64字符），值 = 传入的名字
    NameIllegal(String),
    /// 皮肤尺寸不符（须64×64或旧版64×32）
    SkinSize { width: u32, height: u32 },
    /// 找不到玩家或玩家没有皮肤
    PlayerNotFound,
    /// 皮肤PNG解码失败
    DecodeFail,
    /// 图标渲染失败
    RenderFail,
    /// 皮肤方块不存在，值 = 方块ID
    NotFound(String),
}

/// mml执行结果
pub type CoreResult<T> = result::Result<T, ErrorType>;

/// mml错误类型
#[derive(Clone, Debug)]
pub enum ErrorType {
    /// 严重错误，直接结束程序
    Panic(PanicType),

    /// 配置文件处理时出错
    ConfigError(FileSystemErrorData),

    /// Http请求出错
    HttpError(HttpErrorData),

    /// 序列化处理错误
    SerializerError(ErrorData),

    /// 账户操作错误
    AuthFail(String),
    /// 账户操作没有返回档案
    AuthNoProfile,
    /// 登录密钥过期
    AuthTokenTimeout,
    /// 账户缺少服务器地址（旧版数据未保存，需重新添加）
    AuthServerNull,

    /// OAuth获取登录码错误
    OAuthGetTokenError(ErrorData),
    /// OAuth获取不到登录码
    OAuthGetTokenEmpty,

    /// 文件系统处理错误
    FileSystemError(FileSystemErrorData),
    /// 文件获取错误
    FileReadError(ErrorData),
    /// 路径不存在
    PathNotExists(PathNotExistsData),

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
    /// 没有可用的GPU后端
    GpuNotAvailable,
    /// 实例名字已存在，值 = 名字
    InstanceNameExists(String),
    /// 皮肤方块错误
    SkinBlockError(SkinBlockErrorData),
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
