//! 账户窗口：账户列表 + 微软设备码登录 + Token 刷新
//!
//! 数据由 `mml_auth::auths` 持有（`accounts.json`），这里只做命令与事件转发。

use std::fmt::Display;
use std::path::Path;
use std::sync::RwLock;

use mml_auth::{
    AuthType, LoginObj, auths,
    legacy::{authlib_injector, little_skin, nide8},
    oauth,
};
use mml_names::i18_items::error_type::{CoreResult, ErrorType};
use mml_net::{
    mojang_api::{self, MinecraftProfileObj, SkinObj},
    urls,
};
use mml_sys::open_helper;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::dtos::account_dto::{
    AccountOAuthDto, AccountOAuthStateDto, AccountStoreDto, AccountStoreViewDto, auth_type_from_str,
};
use crate::dtos::skin_dto::{TextureItemDto, TexturesDto};
use crate::listens;

/// 进行中的微软登录轮询取消句柄（None = 无登录进行中）
static OAUTH_NOW: RwLock<Option<CancellationToken>> = RwLock::new(None);

/// 账户变更事件（跨窗口同步刷新列表）
#[gui_macros::emit]
fn emit_account_change(app: &AppHandle) {
    let _ = app.emit(listens::ACCOUNT_CHANGE, ());
}

/// 微软登录设备码事件（前端弹出设备码弹窗）
#[gui_macros::emit]
fn emit_account_oauth(app: &AppHandle, dto: AccountOAuthDto) {
    let _ = app.emit(listens::ACCOUNT_OAUTH, dto);
}

/// 微软登录阶段事件（state：waiting / xbox / xsts / token / profile / ok / fail）
#[gui_macros::emit]
fn emit_account_oauth_state(app: &AppHandle, dto: AccountOAuthStateDto) {
    let _ = app.emit(listens::ACCOUNT_OAUTH_STATE, dto);
}

/// 发送微软登录阶段事件（包装：按 state + message 组装 DTO）
fn emit_oauth_state(app: &AppHandle, state: &str, message: Option<String>) {
    emit_account_oauth_state(
        app,
        AccountOAuthStateDto {
            state: state.to_string(),
            message,
        },
    );
}

/// OAuth 步骤失败：结束本次登录并向前端发送失败状态
fn oauth_fail(app: &AppHandle, err: impl Display) -> String {
    *OAUTH_NOW.write().unwrap() = None;
    let msg = err.to_string();
    emit_oauth_state(app, "fail", Some(msg.clone()));
    msg
}

/// 按 UUID 在账户列表中查找账户（auths 以 UUID + 类型为键，此处 UUID 即可唯一定位当前场景）
fn find_by_uuid(uuid: &str) -> Option<LoginObj> {
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

/// 微软账户登录：设备码授权 → Xbox → XSTS → Minecraft Token → Profile → 保存账户
///
/// 各阶段通过 [`emit_oauth_state`] 通知前端展示进度，
/// 用户可经 `account_cancel_oauth` 中断等待。
async fn microsoft_login(app: &AppHandle) -> Result<AccountStoreDto, String> {
    // 取设备码失败（网络不通等）也要走 oauth_fail：让前端弹出失败提示而不是无响应
    let code = match oauth::get_code().await {
        Ok(code) => code,
        Err(err) => return Err(oauth_fail(app, err)),
    };

    let cancel = CancellationToken::new();
    *OAUTH_NOW.write().unwrap() = Some(cancel.clone());

    let dto = AccountOAuthDto {
        code: code.code.clone(),
        url: format!("{}?otc={}", code.url, code.code),
    };
    emit_oauth_state(app, "waiting", None);
    emit_account_oauth(app, dto);

    let oauth_res = match oauth::run_get_code(&code, &cancel).await {
        Ok(ok) => ok,
        Err(err) => return Err(oauth_fail(app, err)),
    };

    emit_oauth_state(app, "xbox", None);
    let xbox = match oauth::get_xbox(&oauth_res.access_token).await {
        Ok(ok) => ok,
        Err(err) => return Err(oauth_fail(app, err)),
    };

    emit_oauth_state(app, "xsts", None);
    let xsts = match oauth::get_xsts(&xbox.xbl_token).await {
        Ok(ok) => ok,
        Err(err) => return Err(oauth_fail(app, err)),
    };

    emit_oauth_state(app, "token", None);
    let (token, expires_in) =
        match mojang_api::get_minecraft_token(&xsts.xbl_uhs, &xsts.xbl_token).await {
            Ok(ok) => ok,
            Err(err) => return Err(oauth_fail(app, err)),
        };

    emit_oauth_state(app, "profile", None);
    let profile = match mojang_api::get_minecraft_profile(&token).await {
        Ok(ok) => ok,
        Err(err) => return Err(oauth_fail(app, err)),
    };

    *OAUTH_NOW.write().unwrap() = None;

    // Profile 返回的 UUID 无连字符，转为 Minecraft 常规带连字符格式
    let uuid = Uuid::parse_str(&profile.id)
        .map(|u| u.to_string())
        .unwrap_or(profile.id);

    let mut login = LoginObj::new(profile.name, uuid, token, String::new());
    login.auth_type = AuthType::OAuth;
    login.text1 = Some(oauth_res.refresh_token);
    login.set_expire_in(expires_in);
    login.save();

    if auths::get_current().is_none() {
        auths::set_current(Some(login.get_key()));
    }

    emit_account_change(app);
    emit_oauth_state(app, "ok", None);
    Ok(AccountStoreDto::from_login(&login))
}

/// 取消进行中的微软登录轮询
#[tauri::command]
pub fn account_cancel_oauth() {
    if let Some(cancel) = OAUTH_NOW.write().unwrap().take() {
        cancel.cancel();
    }
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

/// 查询正版档案，令牌失效（401 / 403）时自动刷新令牌重试一次
///
/// 微软访问令牌约 24 小时过期，长期挂机后查纹理 / 装备会整体 401。
/// 刷新走完整 OAuth 链并把新令牌写回账户存储（内存 + accounts.json），
/// 后续请求直接用新令牌；令牌不展示给前端、账户可见信息不变，无需广播 account-change。
async fn get_oauth_profile(auth: &mut LoginObj) -> CoreResult<MinecraftProfileObj> {
    let first = mojang_api::get_minecraft_profile(&auth.access_token).await;
    let expired = matches!(
        &first,
        Err(ErrorType::HttpError(data)) if matches!(data.status, Some(401) | Some(403))
    );
    if !expired {
        return first;
    }
    auth.refresh(CancellationToken::new()).await?;
    auth.save();
    mojang_api::get_minecraft_profile(&auth.access_token).await
}

/// 查询账户的皮肤 / 披风纹理列表
///
/// OAuth 账户走 minecraft services 档案（多皮肤 + 多披风，`active` 是服务端
/// 当前选中项）；其余在线账户走会话服务器（单皮肤 + 单披风，恒 `active`）；
/// 离线账户返回空列表（前端走占位图）。取到的纹理同时按内容 SHA1 缓存并登记
/// 源 URL，前端拿到 sha1 后经 `mml-image` 的 sha1 型 URI 取图。
#[tauri::command]
pub async fn account_get_textures(
    account_type: String,
    uuid: String,
) -> Result<TexturesDto, String> {
    let auth_type = auth_type_from_str(&account_type);
    if auth_type == AuthType::Offline {
        return Ok(TexturesDto::default());
    }

    let Some(mut auth) = auths::get(&uuid, auth_type) else {
        return Err(String::from("err.accountMissing"));
    };

    if auth_type.is_oauth() {
        let profile = get_oauth_profile(&mut auth)
            .await
            .map_err(|err| err.to_string())?;
        let skins = textures_from(&profile.skins).await;
        let capes = textures_from(&profile.capes).await;
        return Ok(TexturesDto { skins, capes });
    }

    // 非 OAuth：会话服务器只有单皮肤 + 单披风
    let res = mml_game::player_skin::download_skin(&auth).await;
    let skins = match &res.skin {
        Some(tex) => vec![TextureItemDto {
            name: auth.uuid.clone(),
            sha1: tex.sha1.clone(),
            model: if res.is_new_slim {
                String::from("slim")
            } else {
                String::new()
            },
            active: true,
        }],
        None => Vec::new(),
    };
    let capes = match &res.cape {
        Some(tex) => vec![TextureItemDto {
            name: auth.uuid.clone(),
            sha1: tex.sha1.clone(),
            model: String::new(),
            active: true,
        }],
        None => Vec::new(),
    };

    if let Some(tex) = &res.skin {
        crate::image_manager::push_texture_url(&tex.sha1, &tex.url);
    }
    if let Some(tex) = &res.cape {
        crate::image_manager::push_texture_url(&tex.sha1, &tex.url);
    }
    Ok(TexturesDto { skins, capes })
}

/// 把 OAuth 档案的纹理列表转成 DTO（逐个下载定内容 SHA1 并登记源 URL）
///
/// 各项并行下载（都是等网络的 IO，串行会随皮肤/披风数量线性变慢）；
/// 某个纹理下载失败时跳过该项（不整体失败），结果保持档案里的顺序
async fn textures_from(items: &[SkinObj]) -> Vec<TextureItemDto> {
    let handles: Vec<_> = items
        .iter()
        .map(|item| {
            // 披风项用服务器给的披风名（alias），皮肤项没有 alias，退回条目 UUID
            let name = item.alias.clone().unwrap_or_else(|| item.id.clone());
            let state = item.state.clone();
            let url = item.url.clone();
            let variant = item.variant.clone();
            tokio::spawn(async move {
                let Some(tex) = mml_game::player_skin::load_texture(&url).await else {
                    return None;
                };
                crate::image_manager::push_texture_url(&tex.sha1, &url);
                Some(TextureItemDto {
                    name,
                    sha1: tex.sha1,
                    model: variant.to_lowercase(),
                    active: state == "ACTIVE",
                })
            })
        })
        .collect();

    // 按提交顺序收集，保持档案原有排序
    let mut out = Vec::new();
    for handle in handles {
        if let Ok(Some(dto)) = handle.await {
            out.push(dto);
        }
    }
    out
}

/// 装备账户的皮肤 / 披风（正版专用，第三方规范无多纹理接口）
///
/// 先重查档案，按内容 SHA1 找到对应条目再调服务端装备接口；
/// 成功后清纹理渲染缓存并广播账户变更（列表预览跟随刷新）。
///
/// # 参数
///
/// - `kind`: 纹理种类，`skin` 走换肤接口、`cape` 走显示披风接口
/// - `sha1`: 要装备的纹理内容 SHA1
async fn equip_texture(
    app: &AppHandle,
    account_type: &str,
    uuid: &str,
    kind: &str,
    sha1: &str,
) -> Result<(), String> {
    let auth_type = auth_type_from_str(account_type);
    if !auth_type.is_oauth() {
        return Err(String::from("err.notOauth"));
    }
    let Some(mut auth) = auths::get(uuid, auth_type) else {
        return Err(String::from("err.accountMissing"));
    };

    let profile = get_oauth_profile(&mut auth)
        .await
        .map_err(|err| err.to_string())?;

    // 按内容 SHA1 匹配条目（load_texture 有进程内 URL→SHA1 缓存，重复查询开销小）
    let mut target = None;
    let items = if kind == "skin" {
        &profile.skins
    } else {
        &profile.capes
    };
    for item in items {
        let Some(tex) = mml_game::player_skin::load_texture(&item.url).await else {
            continue;
        };
        if tex.sha1 == sha1 {
            target = Some(item);
            break;
        }
    }
    let Some(item) = target else {
        return Err(String::from("err.textureMissing"));
    };

    if kind == "skin" {
        mojang_api::set_minecraft_skin(&auth.access_token, &item.variant.to_lowercase(), &item.url)
            .await
    } else {
        mojang_api::set_minecraft_cape(&auth.access_token, &item.id).await
    }
    .map_err(|err| err.to_string())?;

    crate::image_manager::clear_texture_images();
    emit_account_change(app);
    Ok(())
}

/// 装备账户选中的皮肤（正版）
#[tauri::command]
pub async fn account_set_active_skin(
    app: AppHandle,
    account_type: String,
    uuid: String,
    sha1: String,
) -> Result<(), String> {
    equip_texture(&app, &account_type, &uuid, "skin", &sha1).await
}

/// 装备账户选中的披风（正版）
#[tauri::command]
pub async fn account_set_active_cape(
    app: AppHandle,
    account_type: String,
    uuid: String,
    sha1: String,
) -> Result<(), String> {
    equip_texture(&app, &account_type, &uuid, "cape", &sha1).await
}

/// 上传本地皮肤文件并装备（正版专用，第三方规范无上传接口）
///
/// 文件路径来自前端系统文件对话框（仅支持 PNG）；上传成功后服务端
/// 自动装备新皮肤，这里清纹理渲染缓存并广播账户变更。
///
/// # 参数
///
/// - `variant`: 皮肤型号，`classic`（经典）或 `slim`（纤细）
/// - `path`: 本地皮肤 PNG 文件路径
#[tauri::command]
pub async fn account_upload_skin(
    app: AppHandle,
    account_type: String,
    uuid: String,
    variant: String,
    path: String,
) -> Result<(), String> {
    let auth_type = auth_type_from_str(&account_type);
    if !auth_type.is_oauth() {
        return Err(String::from("err.notOauth"));
    }
    if variant != "classic" && variant != "slim" {
        return Err(String::from("err.variantInvalid"));
    }

    let data = mml_sys::path_helper::read_byte(&path).map_err(|err| err.to_string())?;
    // PNG 文件签名（89 50 4E 47 0D 0A 1A 0A），服务端只收 PNG，提前拦下明显不对的文件
    if !data.starts_with(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Err(String::from("err.notPng"));
    }

    let auth = auths::get(&uuid, auth_type).ok_or("err.accountMissing")?;
    let file_name = Path::new(&path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("skin")
        .to_string();

    mojang_api::upload_minecraft_skin(&auth.access_token, &variant, &file_name, data)
        .await
        .map_err(|err| err.to_string())?;

    crate::image_manager::clear_texture_images();
    emit_account_change(&app);
    Ok(())
}

/// 打开第三方账户对应皮肤站的网页（皮肤上传 / 装备在皮肤站的网页面板完成）
///
/// LittleSkin 开官网，统一通行证开对应服务器页，自建皮肤站 / 外置登录开
/// 服务器根地址（外置登录存的是 Yggdrasil API 根，剥掉 `/api/yggdrasil` 后缀
/// 得到站点根）。
#[tauri::command]
pub fn account_open_skin_site(account_type: String, uuid: String) -> Result<(), String> {
    let auth_type = auth_type_from_str(&account_type);
    if auth_type == AuthType::Offline || auth_type.is_oauth() {
        return Err(String::from("err.notOauth"));
    }
    let auth = auths::get(&uuid, auth_type).ok_or("err.accountMissing")?;
    let server = auth.text1.clone().unwrap_or_default();

    let url = match auth_type {
        AuthType::LittleSkin => urls::LITTLE_SKIN_URL.to_string(),
        AuthType::Nide8 => format!("{}{}", urls::NIDE8_URL, server),
        AuthType::AuthlibInjector => server
            .trim_end_matches('/')
            .trim_end_matches("/api/yggdrasil")
            .to_string(),
        AuthType::SelfLittleSkin => server.trim_end_matches('/').to_string(),
        _ => return Err(String::from("err.notOauth")),
    };
    if !url.starts_with("http") {
        return Err(String::from("err.serverEmpty"));
    }
    open_helper::open_url(&url);
    Ok(())
}
