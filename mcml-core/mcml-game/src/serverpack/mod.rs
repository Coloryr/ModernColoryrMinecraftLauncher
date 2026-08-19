use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use mcml_base::{
    archives::{ArchiveType, BaseArchive},
    file_item::{FileHash, FileItemObj, LaterRun},
    serialize_tools,
};
use mcml_config::config_save;
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorType, FileSystemErrorData},
    names, uuids,
};
use mcml_net::{
    curseforge_api,
    modrinth_api::{self, version_obj::ModrinthVersionObj},
};
use mcml_sys::path_helper;
use tokio_util::sync::CancellationToken;

use crate::{
    launcher::instance_setting_obj::InstanceSettingObj,
    serverpack::serverpack_obj::{ServerArchiveItemObj, ServerItemObj, ServerPackObj},
};

pub mod serverpack_obj;

impl InstanceSettingObj {
    /// 将服务器实例信息标记为旧版
    pub fn move_serverpack_to_old(&self) -> CoreResult<()> {
        path_helper::move_file(self.get_server_pack_file(), self.get_server_pack_old_file())
    }

    /// 读取旧版服务器实例信息
    fn get_old_serverpack(&self) -> CoreResult<Option<ServerPackObj>> {
        let file = self.get_server_pack_old_file();
        if !file.exists() || file.is_dir() {
            return Ok(None);
        }
        let obj = serialize_tools::json_from_file::<ServerPackObj>(file)?;

        Ok(Some(obj))
    }

    /// 保存服务器包信息
    pub fn save_serverpack(&self, pack: &ServerPackObj) {
        config_save::save(
            uuids::mix_uuid(self.uuid, uuids::SERVERPACK_FILE_UUID),
            pack,
            self.get_server_pack_file(),
        );
    }

    /// 执行升级操作
    ///
    /// 对比旧版服务器包信息，删除已移除的文件，下载并解压新增的文件，
    /// 最后保存新版本信息并清理旧文件。
    pub async fn upgrade_serverpack(
        &self,
        new_pack: ServerPackObj,
        cancel: CancellationToken,
    ) -> CoreResult<()> {
        let old = self.get_old_serverpack()?;

        let game_path = self.get_game_path();

        if let Some(old) = &old {
            // 删除已移除或已更换路径的文件（`file` 是游戏目录下的相对路径）
            for item in &old.online_list {
                if cancel.is_cancelled() {
                    return Err(ErrorType::TaskCancel);
                }
                let matched = new_pack
                    .online_list
                    .iter()
                    .find(|n| mod_key(n) == mod_key(item));
                match matched {
                    // 旧包中已移除的文件
                    None => delete_with_disabled(game_path.join(&item.file)),
                    // 同一文件但路径改变，删除旧文件
                    Some(n) if n.file != item.file => {
                        delete_with_disabled(game_path.join(&item.file))
                    }
                    _ => {}
                }
            }

            // 删除已移除的配置文件（仅删除由该配置独享的目录）
            for item in &old.archive_list {
                if cancel.is_cancelled() {
                    return Err(ErrorType::TaskCancel);
                }
                let removed = !new_pack.archive_list.iter().any(|n| n.file == item.file);
                if removed && item.delete_old && !item.dir.is_empty() {
                    path_helper::move_to_trash(game_path.join(&item.dir))?;
                }
            }
        }

        if cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }

        // 在线文件的 url 为空时从 pid/fid 解析；配置文件暂存到实例临时目录，下载完成后解压
        let need_resolve: Vec<&ServerItemObj> = new_pack
            .online_list
            .iter()
            .filter(|i| i.url.as_deref().map_or(true, |u| u.is_empty()))
            .collect();
        let url_map = resolve_download_urls(&need_resolve, &cancel).await;

        let mut downloads: Vec<FileItemObj> = Vec::new();
        let mut archives: Vec<(&ServerArchiveItemObj, PathBuf)> = Vec::new();

        for item in &new_pack.online_list {
            let url = item
                .url
                .clone()
                .filter(|u| !u.is_empty())
                .or_else(|| item.fid.as_ref().and_then(|f| url_map.get(f).cloned()))
                .unwrap_or_default();
            downloads.push(FileItemObj {
                url,
                name: item.file.clone(),
                file: game_path.join(&item.file),
                hash: make_hash(&item.sha1, &item.sha256),
                later: LaterRun::None,
            });
        }

        for item in &new_pack.archive_list {
            let temp = self.get_temp_path().join(&item.file);
            downloads.push(FileItemObj {
                url: item.url.clone(),
                name: item.file.clone(),
                file: temp.clone(),
                hash: make_hash(&item.sha1, &item.sha256),
                later: LaterRun::None,
            });
            archives.push((item, temp));
        }

        if !downloads.is_empty() && !mcml_downloader::start_download_task(downloads).await {
            return Err(ErrorType::DownloadFileFail);
        }

        for (item, temp) in archives.iter() {
            if cancel.is_cancelled() {
                return Err(ErrorType::TaskCancel);
            }

            let output = game_path.join(&item.dir);
            // 覆盖解压时先删除旧目录
            if item.delete_old && output.exists() {
                path_helper::move_to_trash(&output)?;
            }

            let archive_type = ArchiveType::try_from_path(temp).ok_or_else(|| {
                ErrorType::ArchiveOpenError(FileSystemErrorData {
                    path: temp.clone(),
                    error: String::new(),
                })
            })?;
            BaseArchive::decompress(archive_type, temp, &output, None)?;

            // 清理临时压缩包
            path_helper::delete(temp)?;
        }

        // 保存新版本信息，清理旧文件
        self.save_serverpack(&new_pack);
        path_helper::delete(self.get_server_pack_old_file())?;

        Ok(())
    }
}

/// 模组身份标识：优先使用项目编号，否则回退到文件名
fn mod_key(item: &ServerItemObj) -> String {
    item.pid.clone().unwrap_or_else(|| item.file.clone())
}

/// 根据校验值构建下载哈希
fn make_hash(sha1: &Option<String>, sha256: &Option<String>) -> FileHash {
    match (sha1, sha256) {
        (Some(sha1), Some(sha256)) => FileHash::Sha1Sha256(sha1.clone(), sha256.clone()),
        (Some(sha1), None) => FileHash::Sha1(sha1.clone()),
        (None, Some(sha256)) => FileHash::Sha256(sha256.clone()),
        (None, None) => FileHash::None,
    }
}

/// 删除文件，若文件已被禁用（追加了 `.disable`/`.disabled` 后缀）则一并删除。
fn delete_with_disabled<P: AsRef<Path>>(file: P) {
    let file = file.as_ref();
    // `delete` 在文件不存在时是无操作
    let _ = path_helper::delete(file);
    let _ = path_helper::delete(format!("{}{}", file.display(), names::DISABLE_DOT_EXT));
    let _ = path_helper::delete(format!("{}{}", file.display(), names::DISABLED_DOT_EXT));
}

/// 解析空 url 的文件下载地址（通过 pid/fid 从 CurseForge 或 Modrinth 获取）。
///
/// 按编号格式判断来源：纯数字为 CurseForge，否则为 Modrinth。
/// 优先批量获取，失败或缺失的文件再逐文件降级，返回 `fid → url` 映射。
async fn resolve_download_urls(
    items: &[&ServerItemObj],
    cancel: &CancellationToken,
) -> HashMap<String, String> {
    let mut urls: HashMap<String, String> = HashMap::new();
    if items.is_empty() {
        return urls;
    }

    // 拆分来源
    let mut curseforge: Vec<&ServerItemObj> = Vec::new();
    let mut modrinth: Vec<&ServerItemObj> = Vec::new();
    for item in items {
        if is_curseforge(item) {
            curseforge.push(item);
        } else {
            modrinth.push(item);
        }
    }

    // ── CurseForge：批量获取文件信息 ──
    let cf_ids: Vec<u64> = curseforge
        .iter()
        .filter_map(|i| i.fid.as_ref().and_then(|f| f.parse().ok()))
        .collect();
    if !cf_ids.is_empty() {
        if let Ok(files) = curseforge_api::get_files(cf_ids).await {
            for mut data in files {
                data.fix_download_url();
                if let Some(url) = data.download_url {
                    urls.insert(data.id.to_string(), url);
                }
            }
        }
    }

    // ── Modrinth：批量获取版本信息 ──
    let mo_ids: Vec<String> = modrinth.iter().filter_map(|i| i.fid.clone()).collect();
    if !mo_ids.is_empty() {
        if let Ok(versions) = modrinth_api::get_versions(mo_ids).await {
            for version in versions {
                if let Some(url) = modrinth_file_url(&version) {
                    urls.insert(version.id, url);
                }
            }
        }
    }

    // 批量失败或缺失的，逐文件降级
    for item in items {
        if cancel.is_cancelled() {
            break;
        }
        let Some(pid) = &item.pid else { continue };
        let Some(fid) = &item.fid else { continue };
        if urls.contains_key(fid) {
            continue;
        }
        if is_curseforge(item) {
            if let Ok(res) = curseforge_api::get_mod(pid, fid).await {
                let mut data = res.data;
                data.fix_download_url();
                if let Some(url) = data.download_url {
                    urls.insert(fid.clone(), url);
                }
            }
        } else if let Ok(version) = modrinth_api::get_version(pid, fid).await {
            if let Some(url) = modrinth_file_url(&version) {
                urls.insert(fid.clone(), url);
            }
        }
    }

    urls
}

/// 判断文件来源：CurseForge 的项目/文件编号是纯数字，Modrinth 是 base62 字符串
fn is_curseforge(item: &ServerItemObj) -> bool {
    let Some(pid) = &item.pid else {
        return item
            .fid
            .as_deref()
            .map_or(false, |f| f.parse::<u64>().is_ok());
    };
    pid.parse::<u64>().is_ok()
}

/// 取 Modrinth 版本的主文件下载地址（无 primary 标记时取第一个）
fn modrinth_file_url(version: &ModrinthVersionObj) -> Option<String> {
    version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .map(|f| f.url.clone())
}
