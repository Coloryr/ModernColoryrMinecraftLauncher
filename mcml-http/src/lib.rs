use mcml_config::config_obj::{ProxyState, ProxyType};
use mcml_names::i18;
use mcml_names::i18_items::error_type::ErrorType;
use reqwest::Proxy;
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// 默认超时时间（秒）
const DEFAULT_TIMEOUT: u64 = 10;

/// 默认 User-Agent
const DEFAULT_USER_AGENT: &str = "mcml/1.0.0";

/// HTTP 请求的通用结果类型
pub type NetResult<T> = Result<T, NetError>;

/// 网络请求错误
#[derive(Debug)]
pub enum NetError {
    /// reqwest 库错误
    Reqwest(reqwest::Error),
    /// JSON 解析错误
    Json(serde_json::Error),
    /// 自定义错误消息
    Custom(String),
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::Reqwest(e) => {
                write!(
                    f,
                    "{}",
                    i18::get_error(ErrorType::HttpReqError(e.to_string()))
                )
            }
            NetError::Json(e) => write!(
                f,
                "{}",
                i18::get_error(ErrorType::JsonDecError(e.to_string()))
            ),
            NetError::Custom(msg) => write!(f, "{msg}"),
        }
    }
}

impl std::error::Error for NetError {}

impl From<reqwest::Error> for NetError {
    fn from(e: reqwest::Error) -> Self {
        NetError::Reqwest(e)
    }
}

impl From<serde_json::Error> for NetError {
    fn from(e: serde_json::Error) -> Self {
        NetError::Json(e)
    }
}

impl From<NetError> for ErrorType {
    fn from(value: NetError) -> Self {
        match value {
            NetError::Custom(error) => ErrorType::HttpReadError(error),
            NetError::Reqwest(error) => ErrorType::HttpReqError(error.to_string()),
            NetError::Json(error) => ErrorType::JsonDecError(error.to_string()),
        }
    }
}

/// HTTP 客户端
#[derive(Debug)]
pub struct Client {
    inner: reqwest::Client,
}

impl Client {
    pub fn new(proxy: ProxyState) -> Self {
        let mut headers = HeaderMap::new();

        headers.insert(
            USER_AGENT,
            HeaderValue::try_from(DEFAULT_USER_AGENT).unwrap(),
        );

        let builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT))
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

    /// 创建一个使用默认配置的客户端
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
            .default_headers(headers)
            .proxy(proxy);

        Client {
            inner: builder.build().unwrap(),
        }
    }

    /// 发送 GET 请求，返回反序列化后的 JSON 响应
    ///
    /// # 参数
    /// - `url`: 请求地址
    pub async fn get<T: DeserializeOwned>(&self, url: &str) -> NetResult<T> {
        let resp = self.inner.get(url).send().await?;
        Self::handle_response(resp).await
    }

    /// 发送 GET 请求，返回原始文本响应
    pub async fn get_text(&self, url: &str) -> NetResult<String> {
        let resp = self.inner.get(url).send().await?;
        Ok(resp.text().await?)
    }

    /// 发送 GET 请求，返回原始字节响应
    pub async fn get_bytes(&self, url: &str) -> NetResult<Vec<u8>> {
        let resp = self.inner.get(url).send().await?;
        Ok(resp.bytes().await?.to_vec())
    }

    /// 发送 GET 请求，返回反序列化后的 JSON 响应
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> NetResult<T> {
        let resp = self.inner.get(url).send().await?;
        Self::handle_response(resp).await
    }

    /// 发送 POST 请求，请求体为 JSON
    ///
    /// # 参数
    /// - `url`: 请求地址
    /// - `body`: 请求体，需要实现 `Serialize`
    pub async fn post<B: Serialize>(&self, url: &str, body: &B) -> NetResult<reqwest::Response> {
        Ok(self.inner.post(url).json(body).send().await?)
    }

    /// 发送 POST 请求，请求体为 JSON，返回原始文本响应
    pub async fn post_text<B: Serialize>(&self, url: &str, body: &B) -> NetResult<String> {
        let resp = self.inner.post(url).json(body).send().await?;
        Ok(resp.text().await?)
    }

    /// 发送 POST 请求，请求体为 JSON，返回原始字节响应
    pub async fn post_bytes<B: Serialize>(&self, url: &str, body: &B) -> NetResult<Vec<u8>> {
        let resp = self.inner.post(url).json(body).send().await?;
        Ok(resp.bytes().await?.to_vec())
    }

    /// 发送 POST 请求，请求体为原始字符串（如 JSON 字符串）
    pub async fn post_raw<T: DeserializeOwned>(&self, url: &str, body: &str) -> NetResult<T> {
        let resp = self
            .inner
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .body(body.to_owned())
            .send()
            .await?;
        Self::handle_response(resp).await
    }

    /// 发送 POST 请求，请求体为 JSON，返回JSON
    pub async fn post_json<B: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        json: &B,
    ) -> NetResult<T> {
        let resp = self.inner.post(url).json(json).send().await?;
        Self::handle_response(resp).await
    }

    /// 发送 POST 请求，请求体为表单数据
    pub async fn post_form<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> NetResult<T> {
        let resp = self.inner.post(url).form(params).send().await?;
        Self::handle_response(resp).await
    }

    /// 获取底层 reqwest 客户端的引用
    pub fn inner(&self) -> &reqwest::Client {
        &self.inner
    }

    /// 处理响应，检查状态码并解析 JSON
    async fn handle_response<T: DeserializeOwned>(resp: reqwest::Response) -> NetResult<T> {
        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(NetError::Custom(format!(
                "HTTP {}: {}",
                status.as_u16(),
                text
            )));
        }
        let bytes = resp.bytes().await?;
        let value: T = serde_json::from_slice(&bytes)?;
        Ok(value)
    }
}

impl Default for Client {
    fn default() -> Self {
        Self::new(ProxyState::Auto)
    }
}

pub static WORK_CLIENT: OnceLock<Arc<Client>> = OnceLock::new();
pub static LOGIN_CLIENT: OnceLock<Arc<Client>> = OnceLock::new();

pub fn init() {
    let binding = mcml_config::CONFIG.read().unwrap();
    let config = binding.get().unwrap();
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
