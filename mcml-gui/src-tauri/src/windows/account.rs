//! 账户窗口
use std::sync::RwLock;

use mcml_auth::{AuthType, LoginObj, auths, oauth};
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dtos::account_dto::{
    AccountOAuthDto, AccountStoreDto, AccountStoreViewDto, auth_type_from_str,
};
use crate::listens;

static OAUTH_NOW: RwLock<Option<CancellationToken>> = RwLock::new(None);

fn emit_account_change(app: &AppHandle) {
    app.emit(listens::ACCOUT_CHANGE, ());
}

/// 获取账户列表 + 当前账户
#[tauri::command]
pub fn account_get_accounts() -> AccountStoreViewDto {
    AccountStoreViewDto {
        accounts: auths::get_all()
            .iter()
            .map(AccountStoreDto::from_login)
            .collect(),
        current_uuid: auths::get_current(),
    }
}

/// 添加账户（离线 / 皮肤站等由前端传账户类型与名称）
#[tauri::command]
pub async fn account_add_account(
    app: AppHandle,
    name: String,
    account_type: String,
) -> Result<AccountStoreDto, String> {
    let auth_type = auth_type_from_str(&account_type);
    if auth_type == AuthType::OAuth {
        let code = oauth::get_code().await;
        match code {
            Ok(ok) => {
                let dto = AccountOAuthDto {
                    code: ok.code.clone(),
                    url: format!("{}?otc={}", ok.url, ok.code),
                };

                let cancel = CancellationToken::new();

                *OAUTH_NOW.write().unwrap() = Some(cancel.clone());

                app.emit(listens::ACCOUT_OAUTH, dto);

                let res = oauth::run_get_code(&ok, &cancel).await;
                match res {
                    Ok(ok) => todo!(),
                    Err(err) => todo!(),
                }
            }
            Err(err) => return Err(err.to_string()),
        }
    }

    let n = name.trim().to_string();
    if n.is_empty() {
        return Err("账户名不能为空".to_string());
    }
    let uuid = Uuid::new_v4().to_string();
    let mut login = LoginObj::new(n, uuid.clone(), String::new(), String::new());
    login.auth_type = auth_type_from_str(&account_type);
    if auths::get_current().is_none() {
        auths::set_current(Some(uuid.clone()));
    }
    login.save();
    emit_account_change(&app);
    Ok(AccountStoreDto::from_login(&login))
}

/// 删除账户
#[tauri::command]
pub fn account_remove_account(app: AppHandle, uuid: String) -> Result<bool, String> {
    let Some(login) = auths::get_by_uuid(uuid.clone()) else {
        return Ok(false);
    };
    login.delete();
    if auths::get_current().as_deref() == Some(&uuid) {
        auths::set_current(auths::get_all().first().map(|a| a.uuid.clone()));
    }
    emit_account_change(&app);
    Ok(true)
}

/// 刷新账户 Token（占位：离线占位账户无真实令牌，接入登录后走 LoginObj::refresh）
#[tauri::command]
pub fn account_refresh_account_token(app: AppHandle, uuid: String) -> Result<bool, String> {
    if auths::get_by_uuid(uuid).is_none() {
        return Ok(false);
    }
    emit_account_change(&app);
    Ok(true)
}

/// 设置当前使用账户
#[tauri::command]
pub fn account_set_current_account(app: AppHandle, uuid: String) -> Result<bool, String> {
    if auths::get_by_uuid(uuid.clone()).is_none() {
        return Ok(false);
    }
    auths::set_current(Some(uuid));
    emit_account_change(&app);
    Ok(true)
}