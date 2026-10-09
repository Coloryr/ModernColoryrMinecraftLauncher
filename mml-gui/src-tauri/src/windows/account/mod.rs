//! 账户窗口：账户列表 + 微软设备码登录 + Token 刷新
//!
//! 数据由 `mml_auth::auths` 持有（`accounts.json`），这里只做命令与事件转发。

use mml_auth::{
    AuthType, LoginObj, auths,
    legacy::{authlib_injector, little_skin, nide8},
};

use mml_sys::open_helper;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dtos::account_dto::{AccountStoreDto, AccountStoreViewDto, auth_type_from_str};
use crate::listens;

// 带命令的子模块必须 `pub(crate)`：生成的 `tauri_commands!` 从 crate 根引用它们
pub(crate) mod oauth;
pub(crate) mod skin;

use self::oauth::microsoft_login;

/// 账户变更事件（跨窗口同步刷新列表）
#[gui_macros::emit]
pub(super) fn emit_account_change(app: &AppHandle) {
    let _ = app.emit(listens::ACCOUNT_CHANGE, ());
}

/// 按 UUID 在账户列表中查找账户（auths 以 UUID + 类型为键，此处 UUID 即可唯一定位当前场景）
pub(super) fn find_by_uuid(uuid: &str) -> Option<LoginObj> {
    auths::get_all().into_iter().find(|a| a.uuid == uuid)
}

/// 获取账户列表 + 当前账户
#[tauri::command]
pub fn account_get_accounts() -> AccountStoreViewDto {
    AccountStoreViewDto {
        accounts: auths::get_all()
            .iter()
            .map(AccountStoreDto::from_login)
            .collect(),
        current_uuid: auths::get_current().map(|k| k.uuid),
    }
}

/// 添加账户（离线 / 皮肤站等由前端传账户类型与凭据；微软走设备码登录流程）
///
/// 皮肤站类（LittleSkin / 外置登录 / 统一通行证）会向对应认证服务器发起真实
/// Yggdrasil 登录，服务器地址由认证层存入 `LoginObj.text1` 供后续刷新与皮肤拉取；
/// 离线账户本地直接创建。
#[tauri::command]
pub async fn account_add_account(
    app: AppHandle,
    name: String,
    account_type: String,
    mut server: Option<String>,
    password: Option<String>,
) -> Result<AccountStoreDto, String> {
    // 登录方式锁定（客户端设置）：总开关打开且锁定了条目列表时，只允许添加
    // 其中的类型；同一类型可有多个锁定条目（各自带服务器），此时传入的服务器
    // 必须命中其中一个条目；条目都没配服务器时只按类型锁定
    let lock = &crate::gui_config::get().client;
    let ty_locks: Vec<&crate::gui_config::LoginLockObj> = if lock.login_lock_on {
        lock.login_lock
            .iter()
            .filter(|s| s.ty == account_type)
            .collect()
    } else {
        Vec::new()
    };
    if lock.login_lock_on && !lock.login_lock.is_empty() && ty_locks.is_empty() {
        return Err("err.loginLocked".to_string());
    }
    let locked_servers: Vec<&str> = ty_locks
        .iter()
        .map(|s| s.server.as_str())
        .filter(|s| !s.is_empty())
        .collect();
    if !locked_servers.is_empty() {
        let entered = server.as_deref().unwrap_or("").trim();
        match locked_servers.iter().find(|s| **s == entered) {
            Some(s) => server = Some((*s).to_string()),
            None => return Err("err.loginLocked".to_string()),
        }
    }

    let auth_type = auth_type_from_str(&account_type);
    if auth_type == AuthType::OAuth {
        return microsoft_login(&app).await;
    }

    let server = server
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let user = name.trim().to_string();
    let password = password.unwrap_or_default();

    let mut login = match auth_type {
        AuthType::LittleSkin | AuthType::SelfLittleSkin => {
            // 自建皮肤站必须填服务器地址；LittleSkin 的 server 为 None 时使用官方站
            if auth_type == AuthType::SelfLittleSkin && server.is_none() {
                return Err("err.serverEmpty".to_string());
            }
            little_skin::authenticate(Uuid::new_v4().to_string(), user, password, server, None)
                .await
        }
        AuthType::AuthlibInjector => {
            let Some(server) = server else {
                return Err("err.serverEmpty".to_string());
            };
            authlib_injector::authenticate(Uuid::new_v4().to_string(), user, password, server, None)
                .await
        }
        AuthType::Nide8 => {
            let Some(server) = server else {
                return Err("err.serverEmpty".to_string());
            };
            nide8::authenticate(Uuid::new_v4().to_string(), user, password, server).await
        }
        _ => {
            // 离线账户：本地生成，无凭据
            if user.is_empty() {
                return Err("err.nameEmpty".to_string());
            }
            Ok(LoginObj::new(
                user,
                Uuid::new_v4().to_string(),
                String::new(),
                String::new(),
            ))
        }
    }
    .map_err(|e| e.to_string())?;

    // 皮肤站类的 auth_type 与 text1（服务器地址）由认证层设置，此处不覆盖
    if auth_type == AuthType::Offline {
        login.auth_type = AuthType::Offline;
    }
    if auths::get_current().is_none() {
        auths::set_current(Some(login.get_key()));
    }
    login.save();
    emit_account_change(&app);
    Ok(AccountStoreDto::from_login(&login))
}

/// 用系统浏览器打开微软授权页面
#[tauri::command]
pub fn account_open_browser(url: String) {
    open_helper::open_url(&url);
}

/// 删除账户
#[tauri::command]
pub fn account_remove_account(app: AppHandle, uuid: String) -> Result<bool, String> {
    let Some(login) = find_by_uuid(&uuid) else {
        return Ok(false);
    };
    login.delete();
    if auths::get_current().map(|k| k.uuid) == Some(uuid) {
        auths::set_current(auths::get_all().first().map(|a| a.get_key()));
    }
    emit_account_change(&app);
    Ok(true)
}

/// 刷新账户 Token（按认证类型走 LoginObj::refresh，微软账户走完整 OAuth 刷新链）
#[tauri::command]
pub async fn account_refresh_account_token(app: AppHandle, uuid: String) -> Result<bool, String> {
    let Some(mut login) = find_by_uuid(&uuid) else {
        return Ok(false);
    };
    login
        .refresh(CancellationToken::new())
        .await
        .map_err(|e| e.to_string())?;
    login.save();
    emit_account_change(&app);
    Ok(true)
}

/// 刷新账户皮肤：清掉内存里的纹理缓存与源地址登记，下次取图重新从会话服务器拉取并渲染
///
/// 本地皮肤文件按内容 SHA1 缓存，服务器换肤后贴图内容变化自动落新文件；
/// webview 侧旧图由前端 bump 图片版本号破掉
#[tauri::command]
pub fn account_refresh_skin(app: AppHandle, uuid: String) -> Result<(), String> {
    find_by_uuid(&uuid).ok_or("err.accountMissing")?;
    crate::image_manager::clear_texture_images();
    emit_account_change(&app);
    Ok(())
}

/// 设置当前使用账户
#[tauri::command]
pub fn account_set_current_account(app: AppHandle, uuid: String) -> Result<bool, String> {
    // 空串 = 取消选择（登录方式锁定把当前账户滤掉时前端调这个）
    if uuid.is_empty() {
        auths::set_current(None);
        emit_account_change(&app);
        return Ok(true);
    }
    let Some(login) = find_by_uuid(&uuid) else {
        return Ok(false);
    };
    auths::set_current(Some(login.get_key()));
    emit_account_change(&app);
    Ok(true)
}

/// 编辑离线账户的名字与 UUID
///
/// 离线账户没有服务器凭据，改名/改 UUID 就是改账户本身（存储键随之更新）。
#[tauri::command]
pub fn account_edit_offline(
    app: AppHandle,
    uuid: String,
    new_name: String,
    new_uuid: String,
) -> Result<(), String> {
    let mut login = auths::get(&uuid, AuthType::Offline).ok_or("err.accountMissing")?;

    let new_name = new_name.trim().to_string();
    let new_uuid = new_uuid.trim().to_string();
    if new_name.is_empty() {
        return Err("err.nameEmpty".to_string());
    }
    if new_uuid.is_empty() {
        return Err("err.uuidEmpty".to_string());
    }
    // 新 UUID 不能与其它离线账户冲突（同键会静默覆盖）
    if new_uuid != uuid && auths::get(&new_uuid, AuthType::Offline).is_some() {
        return Err("err.uuidConflict".to_string());
    }

    let was_current = auths::get_current()
        .map(|k| k.uuid == uuid && k.auth_type == AuthType::Offline)
        .unwrap_or(false);

    login.delete();
    login.user_name = new_name;
    login.uuid = new_uuid;
    login.save();

    if was_current {
        auths::set_current(Some(login.get_key()));
    }
    emit_account_change(&app);
    Ok(())
}

// ---- 皮肤 / 披风纹理（原皮肤窗口的查询与正版装备操作） ----
