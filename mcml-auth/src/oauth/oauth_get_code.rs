/// OAuth获取登陆码结果
pub struct OAuthGetCodeRes {
    /// 登录码
    pub code: String,
    /// 登录网址
    pub url: String,
    /// 设备码
    pub device_code: String,
    /// 请求间隔
    pub expires_in: i64,
}

impl OAuthGetCodeRes {
    pub fn new(code: String, url: String, device_code: String, expires_in: i64) -> Self {
        OAuthGetCodeRes {
            code,
            url,
            device_code,
            expires_in,
        }
    }
}
