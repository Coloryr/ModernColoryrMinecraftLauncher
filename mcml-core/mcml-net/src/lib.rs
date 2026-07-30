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
use std::sync::{Arc, OnceLock};
use std::time::Duration;

pub mod adoptium_api;
pub mod authlib_api;
pub mod coloryr_api;
pub mod curseforge_api;
pub mod fabric_api;
pub mod input_file;
pub mod liteloader_api;
pub mod maven_utils;
pub mod mojang_api;
pub mod nide8_api;
pub mod optifine_api;
pub mod quilt_api;
pub mod url_helper;
pub mod urls;

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

/// HTTP 客户端封装
///
/// 对 `reqwest::Client` 的二次包装，增加了超时配置、默认请求头和代理支持。
#[derive(Debug)]
pub struct Client {
    inner: reqwest::Client,
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
        }
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
    let value: T = serialize_tools::json_from_bytes(&bytes)?;
    Ok(value)
}
