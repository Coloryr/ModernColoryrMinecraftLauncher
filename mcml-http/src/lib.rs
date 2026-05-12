use mcml_config::config_obj::{ProxyState, ProxyType};
use mcml_names::i18_items::error_type::{
    ErrorType, HttpReadErrorData, HttpReqErrorData, JsonErrorData,
};
use reqwest::Proxy;
use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::Serialize;
use serde::de::DeserializeOwned;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

/// 默认超时时间（秒）
const DEFAULT_TIMEOUT: u64 = 10;

/// 默认 User-Agent
const DEFAULT_USER_AGENT: &str = "mcml/1.0.0";

/// HTTP 请求的通用结果类型
pub type NetResult<T> = Result<T, ErrorType>;

/// 网络请求错误
#[derive(Debug)]
pub enum NetError {
    Reqwest(reqwest::Error),
    Json(serde_json::Error),
}

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
            NetError::Reqwest(error) => ErrorType::HttpReqError(HttpReqErrorData {
                error: error.to_string(),
                url: match error.url() {
                    Some(url) => url.to_string(),
                    None => Default::default(),
                },
            }),
            NetError::Json(error) => ErrorType::JsonError(JsonErrorData {
                error: error.to_string(),
            }),
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

    /// 发送 GET 请求，返回原始文本响应
    pub async fn get_text(&self, url: &str) -> NetResult<String> {
        let resp = self
            .inner
            .get(url)
            .send()
            .await
            .map_err(NetError::Reqwest)?;
        Ok(resp.text().await.map_err(NetError::Reqwest)?)
    }

    /// 发送 GET 请求，返回原始字节响应
    pub async fn get_bytes(&self, url: &str) -> NetResult<Vec<u8>> {
        let resp = self
            .inner
            .get(url)
            .send()
            .await
            .map_err(NetError::Reqwest)?;
        Ok(resp.bytes().await.map_err(NetError::Reqwest)?.to_vec())
    }

    /// 发送 GET 请求，返回反序列化后的 JSON 响应
    pub async fn get_json<T: DeserializeOwned>(&self, url: &str) -> NetResult<T> {
        let resp = self
            .inner
            .get(url)
            .send()
            .await
            .map_err(NetError::Reqwest)?;
        Self::handle_response(resp).await
    }

    /// 发送 POST 请求，请求体为 JSON
    ///
    /// # 参数
    /// - `url`: 请求地址
    /// - `body`: 请求体，需要实现 `Serialize`
    pub async fn post_json_get_req<B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> NetResult<reqwest::Response> {
        Ok(self
            .inner
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(NetError::Reqwest)?)
    }

    /// 发送 POST 请求，请求体为 JSON，返回原始文本响应
    pub async fn post_json_get_text<B: Serialize>(&self, url: &str, body: &B) -> NetResult<String> {
        let resp = self
            .inner
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(NetError::Reqwest)?;
        Ok(resp.text().await.map_err(NetError::Reqwest)?)
    }

    /// 发送 POST 请求，请求体为 JSON，返回原始字节响应
    pub async fn post_json_get_bytes<B: Serialize>(
        &self,
        url: &str,
        body: &B,
    ) -> NetResult<Vec<u8>> {
        let resp = self
            .inner
            .post(url)
            .json(body)
            .send()
            .await
            .map_err(NetError::Reqwest)?;
        Ok(resp.bytes().await.map_err(NetError::Reqwest)?.to_vec())
    }

    /// 发送 POST 请求，请求体为 JSON，返回JSON
    pub async fn post_json_get_json<B: Serialize, T: DeserializeOwned>(
        &self,
        url: &str,
        json: &B,
    ) -> NetResult<T> {
        let resp = self
            .inner
            .post(url)
            .json(json)
            .send()
            .await
            .map_err(NetError::Reqwest)?;
        Self::handle_response(resp).await
    }

    /// 发送 POST 请求，请求体为表单数据
    pub async fn post_form_get_json<T: DeserializeOwned>(
        &self,
        url: &str,
        params: &[(&str, &str)],
    ) -> NetResult<T> {
        let resp = self
            .inner
            .post(url)
            .form(params)
            .send()
            .await
            .map_err(NetError::Reqwest)?;
        Self::handle_response(resp).await
    }

    /// 处理响应，检查状态码并解析 JSON
    async fn handle_response<T: DeserializeOwned>(resp: reqwest::Response) -> NetResult<T> {
        let status = resp.status();
        if !status.is_success() {
            let url = resp.url().to_string();
            let error = resp.text().await.unwrap_or_default();
            return Err(ErrorType::HttpReadError(HttpReadErrorData {
                error,
                url
            }));
        }
        let bytes = resp.bytes().await.map_err(NetError::Reqwest)?;
        let value: T = serde_json::from_slice(&bytes).map_err(NetError::Json)?;
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
