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
//! | [`fabric_api`] / [`quilt_api`] | 模组加载器 API |
//! | [`optifine_api`] | OptiFine 下载 |
//! | [`authlib_api`] | Authlib-Injector 下载 |
//! | [`adoptium_api`] | Adoptium Java 下载 |
//! | [`nide8_api`] | 统一通行证 API |
//! | [`liteloader_api`] | LiteLoader API |
//! | [`urls`] | URL 常量定义 |
//! | [`url_helper`] | URL 构建辅助函数 |
//! | [`maven_utils`] | Maven 坐标工具 |
//! | [`input_file`] | 输入文件抽象 |

use mcml_base::serialize_tools;
use mcml_config::config_obj::{ProxyState, ProxyType};
use mcml_names::i18_items::error_type::{
    CoreResult, ErrorType, HttpReadErrorData, HttpReqErrorData,
};
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use reqwest::{Proxy, Request, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::fmt;
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub mod adoptium_api;
pub mod authlib_api;
pub mod openfrp_api;
pub mod coloryr_api;
pub mod curseforge_api;
pub mod sakurafrp_api;
pub mod fabric_api;
pub mod input_file;
pub mod liteloader_api;
pub mod maven_utils;
pub mod mojang_api;
pub mod nide8_api;
pub mod modrinth_api;
pub mod optifine_api;
pub mod quilt_api;
pub mod url_helper;
pub mod urls;
pub mod chunkbase_api;

/// 默认 HTTP 超时时间（秒）
const DEFAULT_TIMEOUT: u64 = 10;

/// 默认 User-Agent 标识
const DEFAULT_USER_AGENT: &str = "mcml/1.0.0";

/// 将 reqwest 错误映射为项目统一的 ErrorType
fn map_err(error: reqwest::Error) -> ErrorType {
    ErrorType::HttpReqError(HttpReqErrorData {
        error: error.to_string(),
        url: match error.url() {
            Some(url) => url.to_string(),
            None => Default::default(),
        },
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
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT))
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
    /// - `max_per_minute`: 每分钟最大请求数
    pub async fn get_json_limited<T: DeserializeOwned>(
        &self,
        url: &str,
        max_per_minute: u32,
    ) -> CoreResult<T> {
        // 动态维护速率限制器
        {
            let mut guard = self.rate_limiter.lock().await;
            match *guard {
                Some(ref mut limiter) => limiter.update_limit(max_per_minute),
                None => *guard = Some(RateLimiter::new(max_per_minute)),
            }
            if let Some(ref mut limiter) = *guard {
                limiter.acquire().await;
            }
        }
        let resp = self.inner.get(url).send().await.map_err(map_err)?;
        handle_response(resp).await
    }

    /// 发送带速率限制的 GET 请求，返回响应体文本
    ///
    /// # 参数
    ///
    /// - `url`: 请求地址
    /// - `max_per_minute`: 每分钟最大请求数
    pub async fn get_text_limited(
        &self,
        url: &str,
        max_per_minute: u32,
    ) -> CoreResult<String> {
        {
            let mut guard = self.rate_limiter.lock().await;
            match *guard {
                Some(ref mut limiter) => limiter.update_limit(max_per_minute),
                None => *guard = Some(RateLimiter::new(max_per_minute)),
            }
            if let Some(ref mut limiter) = *guard {
                limiter.acquire().await;
            }
        }
        self.inner
            .get(url)
            .send()
            .await
            .map_err(map_err)?
            .text()
            .await
            .map_err(map_err)
    }

    /// 发送带速率限制的 GET 请求，返回响应体字节
    ///
    /// # 参数
    ///
    /// - `url`: 请求地址
    /// - `max_per_minute`: 每分钟最大请求数
    pub async fn get_bytes_limited(
        &self,
        url: &str,
        max_per_minute: u32,
    ) -> CoreResult<Vec<u8>> {
        {
            let mut guard = self.rate_limiter.lock().await;
            match *guard {
                Some(ref mut limiter) => limiter.update_limit(max_per_minute),
                None => *guard = Some(RateLimiter::new(max_per_minute)),
            }
            if let Some(ref mut limiter) = *guard {
                limiter.acquire().await;
            }
        }
        self.inner
            .get(url)
            .send()
            .await
            .map_err(map_err)?
            .bytes()
            .await
            .map_err(map_err)
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
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT))
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
        self.inner.execute(build).await.map_err(map_err)
    }

    /// 发送 GET 请求，返回原始响应
    pub async fn get(&self, url: &str) -> CoreResult<Response> {
        self.inner.get(url).send().await.map_err(map_err)
    }

    /// 发送 GET 请求，返回响应体文本
    pub async fn get_text(&self, url: &str) -> CoreResult<String> {
        self.inner
            .get(url)
            .send()
            .await
            .map_err(map_err)?
            .text()
            .await
            .map_err(map_err)
    }

    /// 发送 GET 请求，返回响应体字节
    pub async fn get_bytes(&self, url: &str) -> CoreResult<Vec<u8>> {
        self.inner
            .get(url)
            .send()
            .await
            .map_err(map_err)?
            .bytes()
            .await
            .map_err(map_err)
            .map(|data| data.to_vec())
    }

    /// 发送 GET 请求，返回反序列化的 JSON
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> CoreResult<T> {
        let resp = self.inner.get(url).send().await.map_err(map_err)?;
        handle_response(resp).await
    }

    /// 发送带有 Range 头的 GET 请求（断点续传）
    ///
    /// # 参数
    ///
    /// - `url`: 请求地址
    /// - `pos`: 已下载的字节数，从该位置继续下载
    pub async fn get_ranges(&self, url: &str, pos: u64) -> CoreResult<Response> {
        self.inner
            .get(url)
            .header("Range", format!("bytes={}-", pos))
            .send()
            .await
            .map_err(map_err)
    }

    /// 发送 POST 请求，JSON 请求体，返回原始响应
    pub async fn post_json_get_req<B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> CoreResult<reqwest::Response> {
        self.inner
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(map_err)
    }

    /// 发送 POST 请求，JSON 请求体，返回响应文本
    pub async fn post_json_get_text<B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> CoreResult<String> {
        self.inner
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(map_err)?
            .text()
            .await
            .map_err(map_err)
    }

    /// 发送 POST 请求，JSON 请求体，返回响应字节
    pub async fn post_json_get_bytes<B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> CoreResult<Vec<u8>> {
        self.inner
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(map_err)?
            .bytes()
            .await
            .map_err(map_err)
            .map(|data| data.to_vec())
    }

    /// 发送 POST 请求，JSON 请求体，返回反序列化的 JSON
    pub async fn post_json_get_json<B: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        json: &B,
    ) -> CoreResult<T> {
        let resp = self
            .inner
            .post(url)
            .json(json)
            .send()
            .await
            .map_err(map_err)?;
        handle_response(resp).await
    }

    /// 发送带速率限制的 POST 请求，JSON 请求体，返回反序列化的 JSON
    ///
    /// # 参数
    ///
    /// - `url`: 请求地址
    /// - `json`: JSON 请求体
    /// - `max_per_minute`: 每分钟最大请求数
    pub async fn post_json_get_json_limited<B: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        json: &B,
        max_per_minute: u32,
    ) -> CoreResult<T> {
        {
            let mut guard = self.rate_limiter.lock().await;
            match *guard {
                Some(ref mut limiter) => limiter.update_limit(max_per_minute),
                None => *guard = Some(RateLimiter::new(max_per_minute)),
            }
            if let Some(ref mut limiter) = *guard {
                limiter.acquire().await;
            }
        }
        let resp = self
            .inner
            .post(url)
            .json(json)
            .send()
            .await
            .map_err(map_err)?;
        handle_response(resp).await
    }

    /// 发送 POST 请求，表单请求体，返回反序列化的 JSON
    pub async fn post_form_get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> CoreResult<T> {
        let resp = self
            .inner
            .post(url)
            .form(params)
            .send()
            .await
            .map_err(map_err)?;
        handle_response(resp).await
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new(ProxyState::Auto)
    }
}

/// 全局通用 HTTP 客户端（下载资源、一般 API 调用）
static WORK_CLIENT: OnceLock<Arc<Client>> = OnceLock::new();
/// 全局登录 HTTP 客户端（OAuth、Yggdrasil 认证）
static LOGIN_CLIENT: OnceLock<Arc<Client>> = OnceLock::new();

/// 初始化 HTTP 客户端
///
/// 根据配置中的代理设置分别创建通用客户端和登录客户端。
/// 应在程序启动时调用一次。
pub fn init() {
    let config = mcml_config::read_config();
    let http = &config.http;

    let client = if http.work_proxy == ProxyState::User {
        Client::new_proxy(
            http.work_proxy_type,
            &http.proxy_ip,
            http.proxy_port,
            &http.proxy_user,
            &http.proxy_password,
        )
    } else {
        Client::new(http.work_proxy)
    };

    WORK_CLIENT.get_or_init(|| Arc::new(client));

    let client = if http.login_proxy == ProxyState::User {
        Client::new_proxy(
            http.login_proxy_type,
            &http.proxy_ip,
            http.proxy_port,
            &http.proxy_user,
            &http.proxy_password,
        )
    } else {
        Client::new(http.login_proxy)
    };

    LOGIN_CLIENT.get_or_init(|| Arc::new(client));
}

/// 获取全局通用 HTTP 客户端（用于资源下载和一般 API 请求）
pub fn get_work_client() -> Arc<Client> {
    WORK_CLIENT.get().unwrap().clone()
}

/// 获取全局登录 HTTP 客户端（用于 OAuth/Yggdrasil 认证请求）
pub fn get_login_client() -> Arc<Client> {
    LOGIN_CLIENT.get().unwrap().clone()
}

/// 处理 HTTP 响应：检查状态码并解析 JSON
///
/// 如果状态码表示失败（非 2xx），返回 `HttpReadError`。
/// 成功时反序列化 JSON 为指定类型。
pub async fn handle_response<T: DeserializeOwned>(resp: reqwest::Response) -> CoreResult<T> {
    let status = resp.status();
    if !status.is_success() {
        let url = resp.url().to_string();
        let error = resp.text().await.unwrap_or_default();
        return Err(ErrorType::HttpReadError(HttpReadErrorData {
            error,
            url,
            status: status.as_u16(),
        }));
    }
    let bytes = resp.bytes().await.map_err(map_err)?;
    serialize_tools::json_from_bytes(&bytes)
}
