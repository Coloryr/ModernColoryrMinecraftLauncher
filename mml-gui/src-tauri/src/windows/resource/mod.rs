//! 资源管理窗口：实例的模组 / 材质包 / 存档 / 截图 / 服务器 / 光影包 / 结构 / 数据包列表与操作
//!
//! 列表复用 mml-game 的扫描（模组元数据、材质包 pack.mcmeta、存档 level.dat、
//! servers.dat、光影包语言文件、结构文件 NBT）；模组启用 / 禁用 / 删除按 uuid 定位后走
//! `ModObj` 方法；材质包启用 / 禁用按文件名定位后走 `InstanceSettingObj` 的
//! enable / disable_resourcepacks（options.txt 的读写在那儿）；其余类型按「目录 + 纯文件名」
//! 直接操作（回收站）。图标在列表 DTO 里转 base64 data URL（条目少、体积小，不走图片协议）。

// 带命令的子模块必须是 pub(crate)：生成的 	auri_commands! 从 crate 根引用
// windows::resource::screenshots::...，私有模块在这里会 E0603
pub(crate) mod datapacks;
pub(crate) mod folder;
pub(crate) mod groups;
pub(crate) mod mods;
pub(crate) mod packs;
pub(crate) mod saves;
pub(crate) mod schematics;
pub(crate) mod screenshots;
pub(crate) mod servers;
pub(crate) mod shaders;
pub(crate) mod view;

use std::path::{Path, PathBuf};

use mml_base::hash_helper;
use mml_game::GameInstance;
use mml_game::game_saves::SaveObj;
use mml_game::launcher::instance_setting_obj::InstanceSettingObj;
use uuid::Uuid;

/// 实例资源目录类别（open_folder 的 kind 入参）
const KIND_MODS: &str = "mods";
/// 材质包目录
pub(super) const KIND_RESOURCEPACKS: &str = "resourcepacks";
/// 存档目录
pub(super) const KIND_SAVES: &str = "saves";
/// 截图目录（screenshots 子模块也要用）
pub(super) const KIND_SCREENSHOTS: &str = "screenshots";
/// 光影包目录
pub(super) const KIND_SHADERPACKS: &str = "shaderpacks";
/// 结构文件目录
pub(super) const KIND_SCHEMATICS: &str = "schematics";
/// 服务器（打开游戏根目录）
const KIND_SERVERS: &str = "servers";
/// 数据包（存档子页，需配合 parent = 存档目录名）
pub(super) const KIND_DATAPACKS: &str = "datapacks";

/// 解析实例 uuid
pub(super) fn parse_instance(uuid: &str) -> Result<GameInstance, String> {
    let uuid = Uuid::parse_str(uuid).map_err(|_| "err.uuid".to_string())?;
    mml_game::get_instance(&uuid).ok_or_else(|| "err.gameNotFound".to_string())
}

/// 实例下的资源目录路径（锁内只取路径，立即释放）
pub(super) fn instance_dir(instance: &GameInstance, kind: &str) -> Result<PathBuf, String> {
    let game = instance.read().unwrap();
    Ok(match kind {
        KIND_MODS => game.get_mods_path(),
        KIND_RESOURCEPACKS => game.get_resourcepacks_path(),
        KIND_SAVES => game.get_saves_path(),
        KIND_SCREENSHOTS => game.get_screenshots_path(),
        KIND_SHADERPACKS => game.get_shaderpacks_path(),
        KIND_SCHEMATICS => game.get_schematics_path(),
        KIND_SERVERS => game.get_game_path(),
        _ => return Err("err.fileTypeNotFound".to_string()),
    })
}

/// 目录 + 纯文件名定位一个资源文件（防路径穿越）
pub(super) fn resource_file(
    instance: &GameInstance,
    kind: &str,
    name: &str,
) -> Result<PathBuf, String> {
    let dir = instance_dir(instance, kind)?;
    let name_path = Path::new(name);
    if name.is_empty() || name_path.file_name() != Some(name_path.as_os_str()) {
        return Err("err.fileName".to_string());
    }
    let file = dir.join(name_path);
    if !file.starts_with(&dir) {
        return Err("err.fileName".to_string());
    }
    Ok(file)
}

/// 图片字节转 data URL（无图返回空串）
pub(super) fn data_url(bytes: Option<&Vec<u8>>) -> String {
    match bytes {
        Some(data) if !data.is_empty() => format!(
            "data:{};base64,{}",
            image_mime(data),
            hash_helper::gen_base64_bytes(data)
        ),
        _ => String::new(),
    }
}

/// 按魔术字节认图片 MIME（认不出的按 `image/png`）
///
/// 以前一律写死 `image/png`，但模组 logo 里 JPG / GIF 很常见：声明错了
/// 只能靠浏览器嗅探兜底（WebView2 会兜，但不该指望）。
fn image_mime(data: &[u8]) -> &'static str {
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        "image/jpeg"
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        "image/gif"
    } else if data.starts_with(b"BM") {
        "image/bmp"
    } else if data.len() >= 12 && data.starts_with(b"RIFF") && &data[8..12] == b"WEBP" {
        "image/webp"
    } else {
        "image/png"
    }
}

/// 异步实例方法放阻塞线程执行的通用包装（锁卫不跨 await，future 保持 Send）
async fn block_on_instance<T: Send + 'static>(
    instance: GameInstance,
    f: impl FnOnce(InstanceSettingObj) -> T + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(move || {
        // 取一份设置快照后**立刻放锁**，再把快照交给 f。
        //
        // 不能让读锁卫活到 f 里：好几个调用点会在 f 内 `block_on` 跑异步方法
        // （如 get_saves），持锁等异步会和写锁互相卡死 —— clippy 的
        // await_holding_lock 报的就是这条路径。快照克隆只发生在这几个命令里，
        // 都不是热路径。
        let game = instance.read().unwrap().clone();
        f(game)
    })
    .await
    .map_err(|err| err.to_string())
}

/// 获取实例的存档列表（异步），阻塞线程内执行
pub(super) async fn load_saves(
    instance: GameInstance,
) -> Result<Vec<mml_game::game_saves::SaveObj>, String> {
    block_on_instance(instance, |game| {
        tokio::runtime::Handle::current().block_on(async { game.get_saves().await })
    })
    .await
}

/// 按目录名（saves 下的目录）找存档
pub(super) async fn find_save(instance: GameInstance, dir: &str) -> Result<SaveObj, String> {
    let dir = dir.to_string();
    let saves = load_saves(instance).await?;
    saves
        .into_iter()
        .find(|item| {
            item.path
                .file_name()
                .map(|n| *n.to_string_lossy() == dir)
                .unwrap_or(false)
        })
        .ok_or_else(|| "err.saveNotFound".to_string())
}

#[cfg(test)]
mod tests {
    use super::image_mime;
    use super::mods::{mod_note, mod_note_key};
    use std::collections::HashMap;

    /// 按魔术字节认 MIME；认不出来的一律 png（PNG 自己的魔数也走这条）
    #[test]
    fn test_image_mime() {
        assert_eq!(
            image_mime(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            "image/png"
        );
        assert_eq!(image_mime(&[0xFF, 0xD8, 0xFF, 0xE0]), "image/jpeg");
        assert_eq!(image_mime(b"GIF89a..."), "image/gif");
        assert_eq!(image_mime(b"GIF87a..."), "image/gif");
        assert_eq!(image_mime(b"BM\x00\x00"), "image/bmp");
        assert_eq!(image_mime(b"RIFF\x00\x00\x00\x00WEBPVP8 "), "image/webp");
        // 太短 / 认不出来：退回 png，不 panic
        assert_eq!(image_mime(b""), "image/png");
        assert_eq!(image_mime(b"RIFF"), "image/png");
        assert_eq!(image_mime(b"\x00\x01\x02\x03"), "image/png");
    }

    /// 备注的键：禁用后缀要剥掉（启用 / 禁用来回切时备注才是同一份）
    #[test]
    fn test_mod_note_key() {
        assert_eq!(mod_note_key("sodium.jar"), "sodium.jar");
        assert_eq!(mod_note_key("sodium.jar.disabled"), "sodium.jar");
        assert_eq!(mod_note_key("sodium.jar.disable"), "sodium.jar");
        // 名字里本来就带 disabled 的不许误伤（只认结尾）
        assert_eq!(mod_note_key("disabled.jar"), "disabled.jar");
    }

    /// 读备注：归一键优先，退化到原始文件名（ColorMC 在禁用状态下写的）
    #[test]
    fn test_mod_note() {
        let mut notes: HashMap<String, Option<String>> = HashMap::new();
        notes.insert("sodium.jar".into(), Some("优化".into()));
        assert_eq!(mod_note(&notes, "sodium.jar"), "优化");
        assert_eq!(mod_note(&notes, "sodium.jar.disabled"), "优化");

        // 只有带后缀那个键（别的启动器写的）也能读出来
        let mut legacy: HashMap<String, Option<String>> = HashMap::new();
        legacy.insert("old.jar.disabled".into(), Some("旧备注".into()));
        assert_eq!(mod_note(&legacy, "old.jar.disabled"), "旧备注");

        // 没有 / 值为 None / 文件名为空：都返回空串，不 panic
        assert_eq!(mod_note(&notes, "missing.jar"), "");
        assert_eq!(mod_note(&notes, ""), "");
        let mut empty: HashMap<String, Option<String>> = HashMap::new();
        empty.insert("none.jar".into(), None);
        assert_eq!(mod_note(&empty, "none.jar"), "");
    }
}
