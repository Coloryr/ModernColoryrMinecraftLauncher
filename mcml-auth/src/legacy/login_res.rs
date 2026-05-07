use crate::LoginObj;

/// 旧版方式登录结果
pub struct LegacyLoginRes {
    /// 选中的账户
    pub auth: LoginObj,
    /// 可选的账户列表
    pub logins: Option<LoginObj>
}