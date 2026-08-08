//! 统一通行证（Nide8）登录模块
//!
//! 统一通行证是国内流行的 Minecraft 第三方认证服务，
//! 使用 Yggdrasil 兼容 API，通过服务器 UUID 区分不同的认证服务器节点。

use mcml_names::i18_items::error_type::{CoreResult, ErrorType};
use mcml_net::urls;
use tokio_util::sync::CancellationToken;

use crate::{
    AuthType, LoginObj,
    legacy::{self},
};

/// 统一通行证登录认证
///
/// # 参数
///
/// - `client_token`: 客户端标识令牌
/// - `user`: 用户名
/// - `password`: 密码
/// - `server`: Nide8 服务器 UUID，用于构建认证 URL（`{NIDE8_URL}{server}`）
///
/// # 返回值
///
/// 返回已认证的 `LoginObj`，其 `auth_type` 为 `Nide8`，
/// `text1` 保存服务器 UUID
pub async fn authenticate(
    client_token: String,
    user: String,
    password: String,
    server: String,
) -> CoreResult<LoginObj> {
    let url = String::from(urls::NIDE8_URL) + &server;

    let obj = legacy::authenticate(&url, client_token, user, password, false).await?;

    let mut auth = obj.auth;
    auth.auth_type = AuthType::Nide8;
    auth.text1 = Some(server.clone());

    Ok(auth)
}

impl LoginObj {
    /// 刷新统一通行证登录令牌
    ///
    /// 先验证令牌有效性，有效则刷新，无效则返回超时错误。
    ///
    /// # 参数
    ///
    /// - `cancel`: 取消令牌
    pub async fn refresh_nide8(&mut self, cancel: CancellationToken) -> CoreResult<()> {
        let server = String::from(urls::NIDE8_URL) + &self.text1.clone().unwrap();

        if legacy::validate(&server, self).await? {
            if cancel.is_cancelled() {
                return Err(ErrorType::TaskCancel);
            }

            Ok(legacy::refresh(&server, self, false).await?)
        } else {
            Err(ErrorType::AuthTokenTimeout)
        }
    }
}
