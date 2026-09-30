//! 网络请求模块
//!
//! 本模块是启动器的 HTTP 客户端层，封装了 `reqwest` 库，
//! 提供统一的网络请求接口，支持代理配置和多种请求方式。
//!
//! # 双客户端设计
//!
//! 本模块维护两个独立的 HTTP 客户端实例：
//!
//! - **WORK_CLIENT** — 用于一般网络请求（下载资源、API 调用等）
//! - **LOGIN_CLIENT** — 用于登录相关请求（OAuth、Yggdrasil 认证等）
//!
//! 两者可独立配置不同的代理策略，确保登录流量和下载流量可以走不同的网络通道。
//!
//! # 子模块
//!
//! | 模块 | 用途 |
//! |------|------|
//! | [`mojang_api`] | Mojang 官方 API |
//! | [`curseforge_api`] | CurseForge API |
//! | [`modrinth_api`] | Modrinth API |
//! | [`fabric_api`] / [`quilt_api`] | 模组加载器 API |
//! | [`optifine_api`] | OptiFine 下载 |
//! | [`authlib_api`] | Authlib-Injector 下载 |
//! | [`adoptium_api`] | Adoptium Java 下载 |
//! | [`nide8_api`] | 统一通行证 API |
//! | [`liteloader_api`] | LiteLoader API |
//! | [`openfrp_api`] / [`sakurafrp_api`] | 联机平台（OpenFrp / SakuraFrp）节点列表 |
//! | [`chunkbase_api`] | Chunkbase 相关接口 |
//! | [`coloryr_api`] | ColorYr 自有服务接口 |
//! | [`urls`] | URL 常量定义 |
//! | [`url_helper`] | URL 构建辅助函数 |
//! | [`maven_utils`] | Maven 坐标工具 |
//! | [`input_file`] | 输入文件抽象 |

use mml_base::serialize_tools;
use mml_config::config_obj::{HttpObj, ProxyState, ProxyType};
use mml_names::i18_items::error_type::{CoreResult, ErrorType, HttpErrorData};
use reqwest::header::{HeaderMap, HeaderValue, IF_NONE_MATCH, ETAG, USER_AGENT};
use reqwest::{Proxy, Request, Response, StatusCode};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub mod adoptium_api;
pub mod authlib_api;
pub mod chunkbase_api;
pub mod coloryr_api;
pub mod curseforge_api;
pub mod fabric_api;
pub mod input_file;
pub mod liteloader_api;
pub mod maven_utils;
pub mod modrinth_api;
pub mod mojang_api;
pub mod foojay_api;
pub mod nide8_api;
pub mod openfrp_api;
pub mod openj9_api;
pub mod optifine_api;
pub mod quilt_api;
pub mod sakurafrp_api;
pub mod url_helper;
pub mod urls;
pub mod zulu_api;

/// 默认 HTTP 超时时间（秒）：连接超时与单次读取超时共用
///
/// 注意是 `read_timeout`（单次读空闲超时）而非请求总超时——
/// 总超时会掐断慢速网络下的大文件下载（客户端 jar 40+MB）
const DEFAULT_TIMEOUT: u64 = 10;

/// 默认 User-Agent 标识
const DEFAULT_USER_AGENT: &str = "mml/1.0.0";

/// 将 reqwest 错误映射为项目统一的 ErrorType
fn map_err(error: reqwest::Error) -> ErrorType {
    ErrorType::HttpError(HttpErrorData {
        error: error.to_string(),
        url: match error.url() {
            Some(url) => url.to_string(),
            None => Default::default(),
        },
        status: None,
    })
}

/// 请求速率限制器
///
/// 基于滑动时间窗口实现，限制每分钟的最大请求数。
/// 当达到上限后，后续请求将等待下一个时间窗口。
struct RateLimiter {
    /// 每分钟允许的最大请求数
    max_requests: u32,
    /// 当前时间窗口的起始时刻
    window_start: Instant,
    /// 当前窗口内已发出的请求数
    request_count: u32,
}

impl RateLimiter {
    /// 创建新的速率限制器
    ///
    /// # 参数
    ///
    /// - `max_requests`: 每分钟允许的最大请求数
    fn new(max_requests: u32) -> Self {
        Self {
            max_requests,
            window_start: Instant::now(),
            request_count: 0,
        }
    }

    /// 更新最大请求数限制
    ///
    /// - `max_requests`: 每分钟允许的最大请求数
    fn update_limit(&mut self, max_requests: u32) {
        self.max_requests = max_requests;
    }

    /// 尝试获取一个请求槽位。
    ///
    /// 如果当前窗口内请求数已达上限，则等待至下一个时间窗口。
    /// 如果已经过去了一分钟，则自动重置窗口计数器。
    async fn acquire(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.window_start);

        // 如果已经过去了一分钟，重置窗口
        if elapsed >= Duration::from_secs(60) {
            self.window_start = now;
            self.request_count = 0;
        }

        if self.request_count >= self.max_requests {
            // 等待当前窗口结束
            let wait_time = Duration::from_secs(60) - elapsed;
            tokio::time::sleep(wait_time).await;
            self.window_start = Instant::now();
            self.request_count = 0;
        }

        self.request_count += 1;
    }
}

/// HTTP 客户端封装
///
/// 对 `reqwest::Client` 的二次包装，增加了超时配置、默认请求头、代理支持
/// 以及可选的请求速率限制。
pub struct Client {
    inner: reqwest::Client,
    /// 可选的速率限制器，锁内为 `None` 表示不限制
    rate_limiter: Arc<Mutex<Option<RateLimiter>>>,
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Client")
            .field("inner", &self.inner)
            .field("rate_limiter", &"Arc<Mutex<Option<RateLimiter>>>")
            .finish()
    }
}

impl Client {
    /// 创建 HTTP 客户端
    ///
    /// # 参数
    ///
    /// - `proxy`: 代理策略
    ///   - `Auto` / `User` — 使用系统代理或（后续）配置代理
    ///   - `None` — 显式禁用代理
    pub fn new(proxy: ProxyState) -> Self {
        let mut headers = HeaderMap::new();

        headers.insert(
            USER_AGENT,
            HeaderValue::try_from(DEFAULT_USER_AGENT).unwrap(),
        );

        let builder = reqwest::Client::builder()
            .read_timeout(Duration::from_secs(DEFAULT_TIMEOUT))
            .connect_timeout(Duration::from_secs(DEFAULT_TIMEOUT))
            .default_headers(headers);

        let builder = if proxy == ProxyState::None {
            builder.no_proxy()
        } else {
            builder
        };

        Client {
            inner: builder.build().unwrap(),
            rate_limiter: Arc::new(Mutex::new(None)),
        }
    }

    /// 发送带速率限制的 GET 请求，返回反序列化的 JSON
    ///
    /// 每次调用时根据传入的 `max_per_minute` 动态维护速率限制，
    /// 确保每分钟请求数不超过限制。如果之前没有限制或限制值不同，
    /// 则自动创建或更新限制器。
    ///
    /// # 参数
    ///
    /// - `url`: 请求地址
    /// 限速等待（可被全局中断打断）
    ///
    /// 抽出来给三个 `*_limited` 共用：改代理时如果卡在配额等待上，也应该立刻放弃。
    async fn acquire_quota(&self, url: &str, max_per_minute: u32) -> CoreResult<()> {
        let mut guard = self.rate_limiter.lock().await;
        match *guard {
            Some(ref mut limiter) => limiter.update_limit(max_per_minute),
            None => *guard = Some(RateLimiter::new(max_per_minute)),
        }
        let Some(limiter) = guard.as_mut() else {
            return Ok(());
        };

        let generation = ABORT_GEN.load(Ordering::SeqCst);
        let mut rx = abort_rx();
        // 订阅前就已中断：立刻放弃，不用再等配额
        if *rx.borrow() != generation {
            return Err(ErrorType::HttpError(HttpErrorData {
                url: url.to_string(),
                error: ABORT_MSG.to_string(),
                status: None,
            }));
        }
        tokio::select! {
            _ = limiter.acquire() => Ok(()),
            changed = rx.changed() => {
                if changed.is_err() || *rx.borrow() != generation {
                    Err(ErrorType::HttpError(HttpErrorData {
                        url: url.to_string(),
                        error: ABORT_MSG.to_string(),
                        status: None,
                    }))
                } else {
                    // 无关通知：把配额等完（acquire 已消费一次，补一次等待即可）
                    limiter.acquire().await;
                    Ok(())
                }
            }
        }
    }

    /// 发送带速率限制的 GET 请求，返回反序列化的 JSON
    ///
    /// # 参数
    ///
    /// - `url`: 请求地址
    /// - `max_per_minute`: 每分钟最大请求数
    pub async fn get_json_limited<T: DeserializeOwned + Send + 'static>(
        &self,
        url: &str,
        max_per_minute: u32,
    ) -> CoreResult<T> {
        self.acquire_quota(url, max_per_minute).await?;
        // 先在本方法里把请求构造好：RequestBuilder 已经拥有 URL / 序列化后的 body，
        // 之后 spawn 的 future 不再借用任何局部变量（`abortable` 要求 'static）
        let req = self.inner.get(url);
        let resp = abortable(async move { req.send().await.map_err(map_err) }).await?;
        abortable(handle_response(resp)).await
    }

    /// 发送带速率限制的 GET 请求，返回响应体文本
    ///
    /// # 参数
    ///
    /// - `url`: 请求地址
    /// - `max_per_minute`: 每分钟最大请求数
    pub async fn get_text_limited(&self, url: &str, max_per_minute: u32) -> CoreResult<String> {
        self.acquire_quota(url, max_per_minute).await?;
        let req = self.inner.get(url);
        let resp = abortable(async move { req.send().await.map_err(map_err) }).await?;
        abortable(async move { resp.text().await.map_err(map_err) }).await
    }

    /// 发送带速率限制的 GET 请求，返回响应体字节
    ///
    /// # 参数
    ///
    /// - `url`: 请求地址
    /// - `max_per_minute`: 每分钟最大请求数
    pub async fn get_bytes_limited(&self, url: &str, max_per_minute: u32) -> CoreResult<Vec<u8>> {
        self.acquire_quota(url, max_per_minute).await?;
        let req = self.inner.get(url);
        let resp = abortable(async move { req.send().await.map_err(map_err) }).await?;
        abortable(async move { resp.bytes().await.map_err(map_err) })
            .await
            .map(|data| data.to_vec())
    }

    /// 创建一个使用自定义代理的客户端
    ///
    /// # 参数
    ///
    /// - `ptype`: 代理类型（HTTP/SOCKS4/SOCKS5）
    /// - `ip`: 代理服务器 IP
    /// - `port`: 代理服务器端口
    /// - `user`: 代理认证用户名（空字符串表示无认证）
    /// - `pass`: 代理认证密码
    pub fn new_proxy(
        ptype: ProxyType,
        ip: &String,
        port: u16,
        user: &String,
        pass: &String,
    ) -> Self {
        let mut headers = HeaderMap::new();

        headers.insert(
            USER_AGENT,
            HeaderValue::try_from(DEFAULT_USER_AGENT).unwrap(),
        );

        let proxy = match ptype {
            ProxyType::Http => Proxy::all(format!("http://{}:{}", ip, port)).unwrap(),
            ProxyType::Sock4 => Proxy::all(format!("socks4://{}:{}", ip, port)).unwrap(),
            ProxyType::Sock5 => Proxy::all(format!("socks5://{}:{}", ip, port)).unwrap(),
        };

        let proxy = if !user.is_empty() {
            proxy.basic_auth(user, pass)
        } else {
            proxy
        };

        let builder = reqwest::Client::builder()
            .read_timeout(Duration::from_secs(DEFAULT_TIMEOUT))
            .connect_timeout(Duration::from_secs(DEFAULT_TIMEOUT))
            .default_headers(headers)
            .proxy(proxy);

        Client {
            inner: builder.build().unwrap(),
            rate_limiter: Arc::new(Mutex::new(None)),
        }
    }

    /// 发送自定义 HTTP 请求
    pub async fn send(&self, build: Request) -> CoreResult<Response> {
        let inner = self.inner.clone();
        abortable(async move { inner.execute(build).await.map_err(map_err) }).await
    }

    /// 发送 GET 请求，失败时自动重试一次
    ///
    /// 连接池中的空闲连接可能已被服务端/代理关闭，复用时会立即报
    /// "error sending request"，此时换新连接重试一次即可。
    ///
    /// **被全局中断时不重试**：中断（用户改代理）意味着这次请求已经作废，
    /// 重试会发出一条带着新世代号、不受本次中断影响的请求，导致调用方卡住。
    async fn get_with_retry(&self, url: &str) -> CoreResult<Response> {
        // 先取出需要的所有权：`abortable` 内部会 spawn，闭包必须 'static
        let client = self.inner.clone();
        let url = url.to_string();
        abortable(async move {
            match client.get(&url).send().await {
                Ok(resp) => Ok(resp),
                Err(err) => {
                    let mapped = map_err(err);
                    if is_aborted(&mapped) {
                        return Err(mapped);
                    }
                    client.get(&url).send().await.map_err(map_err)
                }
            }
        })
        .await
    }

    /// 发送 GET 请求，返回原始响应
    pub async fn get(&self, url: &str) -> CoreResult<Response> {
        self.get_with_retry(url).await
    }

    /// 带 If-None-Match 缓存校验的 GET 下载：发完整请求并取回响应头与响应体
    ///
    /// 本地存有资源的 ETag 时带上做校验；服务器返回 304 表示资源没变
    /// （响应体为空），返回 200 时响应头里会带新的 ETag。
    /// 失败时自动重试一次（同 `get_with_retry`）。
    ///
    /// # 返回值
    ///
    /// 返回 [`AssetCheckResponse`]（304 标记、新 ETag、响应体）；请求失败返回对应错误
    pub async fn get_bytes_with_check(
        &self,
        url: &str,
        etag: Option<&str>,
    ) -> CoreResult<AssetCheckResponse> {
        // 两个 RequestBuilder 都在 spawn 之前构造好（带 ETag 的那个用于首次尝试，
        // 重试时重新发一条同样的请求）；构造后它们已不借用 url / etag
        let first = {
            let mut req = self.inner.get(url);
            if let Some(etag) = etag {
                req = req.header(IF_NONE_MATCH, etag);
            }
            req
        };
        let retry = {
            let mut req = self.inner.get(url);
            if let Some(etag) = etag {
                req = req.header(IF_NONE_MATCH, etag);
            }
            req
        };

        let resp = abortable(async move {
            match first.send().await {
                Ok(resp) => Ok(resp),
                Err(err) => {
                    // 中断不重试（同 get_with_retry：重试会带新世代号，本次中断对它无效）
                    let mapped = map_err(err);
                    if is_aborted(&mapped) {
                        return Err(mapped);
                    }
                    retry.send().await.map_err(map_err)
                }
            }
        })
        .await?;

        let not_modified = resp.status() == StatusCode::NOT_MODIFIED;
        let etag = resp
            .headers()
            .get(ETAG)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string());
        let data = if not_modified {
            Vec::new()
        } else {
            abortable(async move { resp.bytes().await.map_err(map_err) })
                .await?
                .to_vec()
        };

        Ok(AssetCheckResponse {
            not_modified,
            etag,
            data,
        })
    }

    /// 发送 GET 请求，返回响应体文本
    pub async fn get_text(&self, url: &str) -> CoreResult<String> {
        let resp = self.get_with_retry(url).await?;
        let out = abortable(async move { resp.text().await.map_err(map_err) }).await;
        out
    }

    /// 发送 GET 请求，返回响应体字节
    pub async fn get_bytes(&self, url: &str) -> CoreResult<Vec<u8>> {
        let resp = self.get_with_retry(url).await?;
        let out = abortable(async move { resp.bytes().await.map_err(map_err) })
            .await
            .map(|data| data.to_vec());
        out
    }

    /// 发送 GET 请求，返回反序列化的 JSON
    pub async fn get_json<T: DeserializeOwned + Send + 'static>(&self, url: &str) -> CoreResult<T> {
        let resp = self.get_with_retry(url).await?;
        abortable(handle_response(resp)).await
    }

    /// 发送带有 Range 头的 GET 请求（断点续传）
    ///
    /// # 参数
    ///
    /// - `url`: 请求地址
    /// - `pos`: 已下载的字节数，从该位置继续下载
    pub async fn get_ranges(&self, url: &str, pos: u64) -> CoreResult<Response> {
        let req = self
            .inner
            .get(url)
            .header("Range", format!("bytes={}-", pos));
        abortable(async move { req.send().await.map_err(map_err) }).await
    }

    /// 发送 POST 请求，JSON 请求体，返回原始响应
    pub async fn post_json_get_req<B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> CoreResult<reqwest::Response> {
        // `.json()` 在此就把 body 序列化成拥有的字节，builder 不借用 `body`
        let req = self.inner.post(url).json(body);
        abortable(async move { req.send().await.map_err(map_err) }).await
    }

    /// 发送 POST 请求，multipart 表单（文件上传），带 Bearer 鉴权，返回原始响应
    pub async fn post_multipart(
        &self,
        url: &str,
        form: reqwest::multipart::Form,
        token: &str,
    ) -> CoreResult<reqwest::Response> {
        // `bearer_auth` / `multipart` 都是即刻转换（token 拷进 header、form 被 move），
        // builder 构造完成后不借用任何入参
        let req = self.inner.post(url).bearer_auth(token).multipart(form);
        abortable(async move { req.send().await.map_err(map_err) }).await
    }

    /// 发送 POST 请求，JSON 请求体，返回响应文本
    pub async fn post_json_get_text<B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> CoreResult<String> {
        let req = self.inner.post(url).json(body);
        let resp = abortable(async move { req.send().await.map_err(map_err) }).await?;
        abortable(async move { resp.text().await.map_err(map_err) }).await
    }

    /// 发送 POST 请求，JSON 请求体，返回响应字节
    pub async fn post_json_get_bytes<B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> CoreResult<Vec<u8>> {
        let req = self.inner.post(url).json(body);
        let resp = abortable(async move { req.send().await.map_err(map_err) }).await?;
        abortable(async move { resp.bytes().await.map_err(map_err) })
            .await
            .map(|data| data.to_vec())
    }

    /// 发送 POST 请求，JSON 请求体，返回反序列化的 JSON
    pub async fn post_json_get_json<B: Serialize, T: DeserializeOwned + Send + 'static>(
        &self,
        url: &str,
        json: &B,
    ) -> CoreResult<T> {
        let req = self.inner.post(url).json(json);
        let resp = abortable(async move { req.send().await.map_err(map_err) }).await?;
        abortable(handle_response(resp)).await
    }

    /// 发送带速率限制的 POST 请求，JSON 请求体，返回反序列化的 JSON
    ///
    /// # 参数
    ///
    /// - `url`: 请求地址
    /// - `json`: JSON 请求体
    /// - `max_per_minute`: 每分钟最大请求数
    pub async fn post_json_get_json_limited<B: Serialize, T: DeserializeOwned + Send + 'static>(
        &self,
        url: &str,
        json: &B,
        max_per_minute: u32,
    ) -> CoreResult<T> {
        self.acquire_quota(url, max_per_minute).await?;
        let req = self.inner.post(url).json(json);
        let resp = abortable(async move { req.send().await.map_err(map_err) }).await?;
        abortable(handle_response(resp)).await
    }

    /// 发送 POST 请求，表单请求体，返回反序列化的 JSON
    pub async fn post_form_get_json<T: DeserializeOwned + Send + 'static>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> CoreResult<T> {
        // `.form()` 即刻把参数编码进 body，builder 不借用 `params`
        let req = self.inner.post(url).form(params);
        let resp = abortable(async move { req.send().await.map_err(map_err) }).await?;
        abortable(handle_response(resp)).await
    }

    /// 发送 POST 请求，表单请求体，返回原始响应
    ///
    /// 用于需要对非 2xx 响应自行解析 body 的场景（如 OAuth 设备码轮询中
    /// 400 + `authorization_pending` 属正常中间态，不应视为请求失败）
    pub async fn post_form_get_req(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> CoreResult<reqwest::Response> {
        let req = self.inner.post(url).form(params);
        abortable(async move { req.send().await.map_err(map_err) }).await
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new(ProxyState::Auto)
    }
}

/// 全局通用 HTTP 客户端（下载资源、一般 API 调用）
/// 带 ETag 缓存校验的下载结果（[`Client::get_bytes_with_check`] 返回）
pub struct AssetCheckResponse {
    /// 服务器返回 304：本地缓存的资源仍然有效
    pub not_modified: bool,
    /// ETag 响应头（S3 / R2 类存储在非分片上传时就是内容的 MD5）
    pub etag: Option<String>,
    /// 响应体（304 时为空）
    pub data: Vec<u8>,
}

static WORK_CLIENT: OnceLock<RwLock<Arc<Client>>> = OnceLock::new();
/// 全局登录 HTTP 客户端（OAuth、Yggdrasil 认证）
static LOGIN_CLIENT: OnceLock<RwLock<Arc<Client>>> = OnceLock::new();

/// 当前"请求世代"：每次 [`rebuild`] 自增，用于一次性中断所有在途请求
///
/// 机制：每个请求 `select!` 监听世代通道 [`abort_rx`] 的变化，[`rebuild`] 自增后
/// 通过 [`abort_tx`] 广播新世代号。
///
/// **为什么用 `watch` 而不是 `Notify`**：`Notify::notify_waiters()` 有一个致命语义——
/// 它只唤醒**当时已经在等待队列里**的 waiter，**不保存许可**；而 `Notified` 是
/// **惰性登记**的（要到第一次被 poll 才真正进队列）。于是会出现这个竞态：
///
/// ```text
/// 请求 A：创建 Notified（尚未 poll，还没进队列）
/// 用户  ：改代理 → notify_waiters() → 队列为空 → 信号丢弃
/// 请求 A：select! 开始等待 → 永远等不到这次中断
/// ```
///
/// `watch` 保存**最新值**，晚订阅者 `changed()` 会立刻返回，天然没有信号丢失。
/// （实测症状：六路加载器查询卡在"读 body"阶段，中断日志有、请求却永不返回。）
static ABORT_GEN: AtomicU64 = AtomicU64::new(0);
/// 世代广播通道（发送端）
static ABORT_TX: OnceLock<tokio::sync::watch::Sender<u64>> = OnceLock::new();

/// TEMP 诊断：给每个 abortable 等待点编号，便于对照日志
static ABORT_DEBUG_SEQ: AtomicU64 = AtomicU64::new(0);

/// 在途请求任务的取消句柄表
///
/// [`abortable`] 每提交一个请求就把 `AbortHandle` 登记进来，任务结束后清理。
/// [`abort_all`] 遍历整表 `abort()`——这是**唯一不依赖"目标 future 被 poll"**的
/// 取消手段，用于覆盖"请求从未被调度"的场景（实测出现过）。
static ABORT_TASKS: OnceLock<std::sync::Mutex<Vec<tokio::task::AbortHandle>>> = OnceLock::new();

fn abort_tasks() -> &'static std::sync::Mutex<Vec<tokio::task::AbortHandle>> {
    ABORT_TASKS.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// 登记一个在途任务
fn register_task(handle: tokio::task::AbortHandle) {
    if let Ok(mut list) = abort_tasks().lock() {
        // 顺手清掉已结束的，避免表无限增长（AbortHandle 未实现 PartialEq，
        // 没法精确移除指定项，用 is_finished 过滤是可行且够用的做法）
        list.retain(|h| !h.is_finished());
        list.push(handle);
    }
}

/// 清理已结束的任务（请求正常完成时调用）
fn prune_tasks() {
    if let Ok(mut list) = abort_tasks().lock() {
        list.retain(|h| !h.is_finished());
    }
}

/// 强制取消所有在途请求：遍历登记表逐个 `abort()`
///
/// 返回被取消的任务数（用于日志）。
fn abort_all_tasks() -> usize {
    let tasks: Vec<tokio::task::AbortHandle> = match abort_tasks().lock() {
        Ok(mut list) => std::mem::take(&mut *list),
        Err(_) => return 0,
    };
    let count = tasks.len();
    for handle in tasks {
        handle.abort();
    }
    count
}

/// 世代广播通道（发送端）
fn abort_tx() -> &'static tokio::sync::watch::Sender<u64> {
    ABORT_TX.get_or_init(|| {
        let (tx, _rx) = tokio::sync::watch::channel(0u64);
        tx
    })
}

/// 世代广播通道（接收端）
///
/// 每次调用都拿一个新的接收者（克隆自同一个通道）：`changed()` 只在**调用方持有的
/// 这个接收者**还没见过的新值出现时才返回，所以不同请求之间互不影响。
fn abort_rx() -> tokio::sync::watch::Receiver<u64> {
    abort_tx().subscribe()
}

/// 中断所有在途请求：发起新请求方拿到的错误文案
pub const ABORT_MSG: &str = "请求已中断（代理/网络设置已更新）";

/// 自轮询间隔：请求每隔这么久自查一次世代号
///
/// 取 100ms：用户点保存后最多 0.1 秒内所有在途请求都会停，感知不到延迟；
/// 定时器由各请求自己的 runtime 驱动，不依赖跨线程唤醒，因此**一定能生效**。
const ABORT_POLL_INTERVAL: std::time::Duration = std::time::Duration::from_millis(100);

/// 这个错误是不是"被全局中断"造成的
///
/// 供上层区分两种失败：**中断**（用户改了代理，应当整体放弃、让用户重试）与
/// **普通网络错误**（某一项查不到，按原有逻辑跳过即可）。
/// 像加载器支持列表那种"六路并发、单路失败不算失败"的地方必须用这个判断，
/// 否则中断错误会被 `unwrap_or(false)` 吞掉，外面既看不到错误、进度条也继续爬。
pub fn is_aborted(err: &ErrorType) -> bool {
    match err {
        ErrorType::HttpError(data) => data.error == ABORT_MSG,
        _ => false,
    }
}

/// 把一次请求与"全局中断"绑在一起
///
/// 请求正常完成就返回它的结果；中途 [`rebuild`] 被调用（用户改了代理）则立刻返回
/// [`ABORT_MSG`]。
///
/// # 为什么必须用 `spawn` + `AbortHandle`
///
/// reqwest 没有"取消全部在途请求"的接口（C# 的 `HttpClient.Dispose()` 在 Rust 侧
/// 没有等价物），而写在 future 内部的任何检查（`select!` + `watch` / token / sleep）
/// 都有一个共同前提：**这个 future 还得被 poll**。
///
/// 实测中出现过请求**从未被 poll** 的情况（mml_log 有"开始等待"、stderr 无任何 poll
/// 记录），此时上述手段全部失效。`AbortHandle::abort()` 是 **runtime 层面的强制取消**：
/// 不需要目标 future 配合、也不要求它被 poll，因此是唯一能覆盖该场景的手段。
///
/// 实现上把请求放进独立任务，`select!` 只等一个轻量的 `JoinHandle`（必然可唤醒）：
/// - 正常完成 → 返回结果；
/// - 收到中断 → `abort()` 该任务并返回中断错误；
/// - 任务被 `abort_all()` 从别处强制取消 → `JoinError`，同样报中断。
///
/// **注意**：不引入 `Client::timeout()`（它会让大文件下载被误杀）；
/// 卡死兜底仍由 builder 上的 `read_timeout` 负责（实测 10 秒后自动失败）。
async fn abortable<F, T>(fut: F) -> CoreResult<T>
where
    F: std::future::Future<Output = CoreResult<T>> + Send + 'static,
    T: Send + 'static,
{
    let entered_gen = ABORT_GEN.load(Ordering::SeqCst);
    let started = std::time::Instant::now();
    let token = ABORT_DEBUG_SEQ.fetch_add(1, Ordering::SeqCst);

    // 进入时就已经过期（中断发生在提交之前）：直接失败，不用起任务
    if ABORT_GEN.load(Ordering::SeqCst) != entered_gen {
        return Err(ErrorType::HttpError(HttpErrorData {
            url: String::new(),
            error: ABORT_MSG.to_string(),
            status: None,
        }));
    }

    // 把请求放进独立任务：这样即使它卡住不让出，也能被 abort() 强制取消
    let handle = tokio::spawn(fut);
    let abort_handle = handle.abort_handle();
    register_task(abort_handle.clone());

    let result = tokio::select! {
        joined = handle => {
            // 任务结束：清理登记表里已完成项
            prune_tasks();
            match joined {
                Ok(res) => {
                    res
                }
                // 被 abort()：JoinError::is_cancelled
                Err(err) if err.is_cancelled() => {
                    Err(ErrorType::HttpError(HttpErrorData {
                        url: String::new(),
                        error: ABORT_MSG.to_string(),
                        status: None,
                    }))
                }
                Err(err) => {
                    // panic 等异常：当成请求失败，别把启动器带崩
                    Err(ErrorType::HttpError(HttpErrorData {
                        url: String::new(),
                        error: err.to_string(),
                        status: None,
                    }))
                }
            }
        }
        // 世代变化（保存代理）：强制取消任务并立即返回
        _ = wait_abort(entered_gen) => {
            abort_handle.abort();
            prune_tasks();
            Err(ErrorType::HttpError(HttpErrorData {
                url: String::new(),
                error: ABORT_MSG.to_string(),
                status: None,
            }))
        }
    };

    result
}

/// 等到"世代号变得与 `entered_gen` 不同"（可被 `abort_all` 唤醒）
///
/// 用"定时自查 + 通道通知"双路：通道是快路径，定时器保证即使通知没送达也一定能发现。
async fn wait_abort(entered_gen: u64) {
    let mut rx = abort_rx();
    loop {
        if ABORT_GEN.load(Ordering::SeqCst) != entered_gen {
            return;
        }
        tokio::select! {
            _ = tokio::time::sleep(ABORT_POLL_INTERVAL) => {}
            _ = rx.changed() => {}
        }
    }
}



/// 按当前配置构建一个客户端
///
/// 抽出来是为了让 [`init`] 与 [`rebuild`] 共用同一套「读配置 → 选代理」逻辑：
/// `work_proxy` / `login_proxy` 为 `User` 时用用户填的地址，否则按策略交给 `Client::new`
/// （`None` 显式禁用代理，`Auto` 跟随系统）。
fn build_client(state: ProxyState, ptype: ProxyType, http: &HttpObj) -> Client {
    if state == ProxyState::User {
        Client::new_proxy(
            ptype,
            &http.proxy_ip,
            http.proxy_port,
            &http.proxy_user,
            &http.proxy_password,
        )
    } else {
        Client::new(state)
    }
}

/// 把代理配置描述成一行日志文本（不含密码）
fn describe_proxy(state: ProxyState, ptype: ProxyType, http: &HttpObj) -> String {
    match state {
        ProxyState::User => format!(
            "User/{} {}:{}（认证={}）",
            match ptype {
                ProxyType::Http => "http",
                ProxyType::Sock4 => "socks4",
                ProxyType::Sock5 => "socks5",
            },
            http.proxy_ip,
            http.proxy_port,
            if http.proxy_user.is_empty() { "无" } else { "有" }
        ),
        ProxyState::None => "None（禁用代理）".to_string(),
        ProxyState::Auto => "Auto（跟随系统）".to_string(),
    }
}

/// 初始化 HTTP 客户端
///
/// 根据配置中的代理设置分别创建通用客户端和登录客户端。
/// 应在程序启动时调用一次；之后改代理设置请用 [`rebuild`]。
pub fn init() {
    let config = mml_config::read_config();
    let http = &config.http;

    WORK_CLIENT.get_or_init(|| {
        RwLock::new(Arc::new(build_client(
            http.work_proxy,
            http.work_proxy_type,
            http,
        )))
    });
    LOGIN_CLIENT.get_or_init(|| {
        RwLock::new(Arc::new(build_client(
            http.login_proxy,
            http.login_proxy_type,
            http,
        )))
    });

    mml_log::info(format!(
        "HTTP 客户端已初始化：通用[{}] 登录[{}]",
        describe_proxy(http.work_proxy, http.work_proxy_type, http),
        describe_proxy(http.login_proxy, http.login_proxy_type, http)
    ));
}

/// 按当前配置**重建**两个 HTTP 客户端（改代理设置后调用）
///
/// 背景：客户端原先用 `OnceLock` 只建一次，导致「设置里改了代理，本次运行却不生效，
/// 必须重启启动器」。现在客户端存在 `RwLock<Arc<Client>>` 里，保存网络设置后调本函数
/// 即可立刻换上新配置。
///
/// 已经取走的 `Arc` 不受影响（正在进行的下载会继续用旧客户端跑完），
/// 之后新取的请求一律用新客户端。
///
/// [`init`] 尚未调用时什么都不做（配置还没读，此时也没有请求在跑）。
pub fn rebuild() {
    // 先中断所有在途请求（含限速等待），再换客户端：顺序反了的话，
    // 旧请求可能刚醒过来又用旧客户端发出去
    let aborted_gen = abort_all();
    mml_log::info(format!(
        "HTTP 客户端重启：已中断在途请求（世代 -> {aborted_gen}）"
    ));

    let config = mml_config::read_config();
    let http = &config.http;

    if let Some(lock) = WORK_CLIENT.get() {
        let client = Arc::new(build_client(
            http.work_proxy,
            http.work_proxy_type,
            http,
        ));
        if let Ok(mut guard) = lock.write() {
            *guard = client;
        }
    }
    if let Some(lock) = LOGIN_CLIENT.get() {
        let client = Arc::new(build_client(
            http.login_proxy,
            http.login_proxy_type,
            http,
        ));
        if let Ok(mut guard) = lock.write() {
            *guard = client;
        }
    }

    mml_log::info(format!(
        "HTTP 客户端重启完成：通用[{}] 登录[{}]",
        describe_proxy(http.work_proxy, http.work_proxy_type, http),
        describe_proxy(http.login_proxy, http.login_proxy_type, http)
    ));
}

/// 中断所有在途请求
///
/// 自增世代号并**广播**给所有正在等待的请求；被中断的请求返回 [`ABORT_MSG`]
/// 对应的错误，调用方按"请求失败"处理即可（用户改代理后重试就是想要的结果）。
///
/// 用 `watch` 广播（存值，不丢信号），而不是 `Notify::notify_waiters`（只唤醒当时
/// 已在队列里的 waiter，晚一步订阅的请求会永远错过——见 [`ABORT_GEN`] 的说明）。
///
/// 单独暴露出来是为了让 [`rebuild`] 之外的场景（例如退出登录、切源）也能用。
///
/// # 返回值
///
/// 返回自增后的世代号（用于日志排查"到底有没有真的中断"）
pub fn abort_all() -> u64 {
    let generation = ABORT_GEN.fetch_add(1, Ordering::SeqCst) + 1;
    let tx = abort_tx();
    let receivers = tx.receiver_count();
    let send_result = tx.send(generation);
    // **强制取消全部在途任务**：这是不依赖"请求被 poll"的硬手段
    let aborted = abort_all_tasks();

    // - 若这个任务也被卡住 → 全局性阻塞（锁 / 计划器被占死）
    // - 若它正常完成 → 只是"原来那些 future 没被调度"，范围收窄到它们的 runtime
    // 用独立线程是为了不受调用方所在 runtime 影响。
    let (probe_tx, _probe_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let outcome = match rt {
            Ok(rt) => rt.block_on(async {
                // 纯异步任务（不碰网络），能跑完说明新 runtime 健康
                tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                "ok"
            }),
            Err(_) => "runtime_build_failed",
        };
        let _ = probe_tx.send(outcome);
    });

    generation
}

/// TEMP 诊断②：探测当前异步上下文是否还能被调度
///
/// 在 [`abort_all`] 里通过独立线程起一个新的 `current_thread` runtime 跑一个
/// 立即完成的任务：若它也卡住，说明是全局性阻塞。
pub fn probe_scheduler() -> (String, std::time::Duration) {
    let t = std::time::Instant::now();
    let outcome = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt.block_on(async {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            "ok".to_string()
        }),
        Err(e) => format!("build_failed: {e}"),
    };
    (outcome, t.elapsed())
}

/// 当前世代号（排查用：两次读取不同即说明期间发生过中断）
pub fn abort_generation() -> u64 {
    ABORT_GEN.load(Ordering::SeqCst)
}

/// 获取全局通用 HTTP 客户端（用于资源下载和一般 API 请求）
pub fn get_work_client() -> Arc<Client> {
    WORK_CLIENT
        .get()
        .and_then(|lock| lock.read().ok().map(|guard| guard.clone()))
        .unwrap()
}

/// 获取全局登录 HTTP 客户端（用于 OAuth/Yggdrasil 认证请求）
pub fn get_login_client() -> Arc<Client> {
    LOGIN_CLIENT
        .get()
        .and_then(|lock| lock.read().ok().map(|guard| guard.clone()))
        .unwrap()
}

/// 处理 HTTP 响应：检查状态码并解析 JSON
///
/// 如果状态码表示失败（非 2xx），返回 `HttpError`。
/// 成功时反序列化 JSON 为指定类型；解析失败时把请求地址一并写进错误信息。
///
/// **这里不加 `Send + 'static`**：`curseforge_api::send<T: DeserializeOwned>` 等
/// 别的模块会直接调用本函数，给泛型加约束会把它们一起卡死。需要经过
/// [`abortable`]（`tokio::spawn`）的调用点改为在各自方法上要求 `T: Send + 'static`。
pub async fn handle_response<T: DeserializeOwned>(resp: reqwest::Response) -> CoreResult<T> {
    let status = resp.status();
    let url = resp.url().to_string();
    if !status.is_success() {
        let error = resp.text().await.unwrap_or_default();
        return Err(ErrorType::HttpError(HttpErrorData {
            error,
            url,
            status: Some(status.as_u16()),
        }));
    }
    let bytes = resp.bytes().await.map_err(map_err)?;
    serialize_tools::json_from_bytes(&bytes).map_err(|err| serialize_err_context::<T>(err, &url))
}

/// 给 JSON 解析错误补上请求地址与目标解析类型
///
/// serde 的报错只有字段级描述（如 `invalid type: null at line 1 column 42`），
/// 定位不到是哪个接口、哪个结构体出的问题，网络侧解析失败时统一补在后面。
pub fn serialize_err_context<T>(err: ErrorType, url: &str) -> ErrorType {
    match err {
        ErrorType::SerializerError(mut data) => {
            data.error = format!(
                "{} (url: {url}, target: {})",
                data.error,
                std::any::type_name::<T>()
            );
            ErrorType::SerializerError(data)
        }
        other => other,
    }
}

