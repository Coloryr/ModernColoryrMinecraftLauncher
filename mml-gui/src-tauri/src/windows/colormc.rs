//! 从 ColorMC 迁移（首次启动问一次：复制 / 移动 / 不迁移）
//!
//! **迁移本身是原样的文件夹搬运**：把 ColorMC 工作目录的顶层条目复制（或移动）到 M²L 的
//! 运行目录，不做字段级转换、不做合并。两边目录结构同构 —— `minecraft/{instances,assets,
//! libraries,versions}`、`config.json`、`collect.json` 语义一致，所以实例搬过来 M²L 直接能用
//! （两边实例都在 `minecraft/instances/<名>/game.json`）。
//!
//! 搬完再做一次**兼容性检测**（见 [`check`]）：哪些能直接用、哪些用起来有偏差、哪些 ColorMC
//! 有而 M²L 读不懂。实例是不是"真的能用"用 **M²L 自己的解析器**判定（`game.json` →
//! [`InstanceSettingObj`]）—— M²L 只认"目录里有能解析的 `game.json`"的实例，所以解析失败
//! 就等于"搬过来了也看不见"。
//!
//! 实例目录里那份 `guisetting.json`（ColorMC 的界面设置）**M²L 不读**：M²L 自己那份叫
//! `gui_setting.json`（改名就是为了不撞它，理由见 `crate::gui_setting` 的文件头）。
//! 于是 ColorMC 的模组分组 / 备注 / 日志设置不会迁移过来 —— 但**文件本身留着、不会被覆盖**，
//! 弹窗里会把这件事说出来。
//!
//! 两条与用户约定好的口径（弹窗上也要写清楚）：
//! - **一律覆盖**：运行目录里同名的文件 / 目录直接被 ColorMC 的版本盖掉。Windows 的
//!   `rename` 不覆盖已存在的目标，所以移动时先删掉目标条目；
//! - **移动 = 重命名**：按顶层条目 `rename`，同盘瞬时完成；跨盘（`CrossesDevices`）回退成
//!   "复制 + 删源"。**全部条目都搬完**才删空的源目录，任何一条失败都保留源目录。
//!
//! 单个文件失败（被别的进程占用、无权限）**不中断**整次迁移，记进报告的 `failed` 里 ——
//! 真实场景下 `logs.log` / `config.json` / `count.dat` 可能正被 ColorMC 或本进程打开。
//!
//! 首次启动的"问过了"状态存在运行目录下的 `colormc_migrate.json`（见 [`mark`]）：
//! 它表达的是"这个运行目录已经问过了"，删掉它就能重新触发询问。

use std::{
    fs,
    io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

use mml_base::{get_base_dir, serialize_tools};
use mml_game::launcher::instance_setting_obj::InstanceSettingObj;
use mml_names::names;
use mml_sys::path_helper;

use crate::dtos::{ColorMcCompatDto, ColorMcInfoDto, ColorMcProgressDto, ColorMcReportDto};
use crate::listens;

/// ColorMC 的数据目录名（`%LOCALAPPDATA%\<它>` 与 `%APPDATA%\<它>`）
const COLORMC_DIR: &str = "ColorMC";
/// ColorMC 在 exe 旁边的默认工作目录名（Windows 上 `<exe 目录>\colormc\`）
const COLORMC_INNER_DIR: &str = "colormc";
/// ColorMC 记"运行路径"的文件名（内容就是路径字符串，在 `%LOCALAPPDATA%\ColorMC\` 下）
const COLORMC_RUN_FILE: &str = "run";
/// ColorMC 的实例界面设置文件名
///
/// 注意与 [`names::GUI_SETTING_FILE`]（M²L 自己的 `gui_setting.json`）**不是同一个文件**：
/// 两边的 `Groups` 结构不同，同名会让 M²L 整份读不了。这里只用它判断"这个实例还带着
/// ColorMC 的界面设置"，好在报告里说明那部分数据没跟过来。
const COLORMC_GUI_SETTING_FILE: &str = "guisetting.json";

/// 兼容性对照表：ColorMC 工作目录的**顶层条目** → （级别，说明 i18n key）
///
/// 级别：`ok` 能直接用 / `partial` 能用但有偏差或会丢东西 / `extra` M²L 读不懂（留着不碍事）。
/// 结论来自逐项对照 ColorMC 源码（`E:\code\ColorMC`）与本仓库代码：
/// - `minecraft/` **同构**：实例、资源、版本库、依赖库都在同样的位置，能直接用；
/// - `collect.json` 结构一致（M²L 的 `CollectObj` 少了 ColorMC 的 4 个开关，未知字段被忽略）；
/// - `config.json` 能读（`#[serde(default)]`），但 ColorMC 独有的 `SafeLog4j` 会丢、
///   代理用 3 个 bool 表达而 M²L 用"策略 + 类型"两组枚举 —— 语义不同，会回落成默认值；
/// - `count.dat` 是游戏统计（读不了就从零开始）；`logs.log` 会盖掉 M²L 自己那份日志；
/// - `java/` 是 ColorMC 的运行时目录，合过去 M²L 不会自动认那些 JRE；
/// - 其余目录 / 文件 M²L 根本不读。
const COMPAT_TABLE: &[(&str, &str, &str)] = &[
    ("minecraft", "ok", "colormc.compat.minecraft"),
    ("collect.json", "ok", "colormc.compat.collect"),
    ("config.json", "partial", "colormc.compat.config"),
    ("count.dat", "partial", "colormc.compat.count"),
    ("logs.log", "partial", "colormc.compat.logs"),
    ("java", "partial", "colormc.compat.java"),
    ("download", "extra", "colormc.compat.download"),
    ("block", "extra", "colormc.compat.block"),
    ("image", "extra", "colormc.compat.extra"),
    ("tools", "extra", "colormc.compat.extra"),
    ("frpc", "extra", "colormc.compat.extra"),
    ("inputs", "extra", "colormc.compat.extra"),
    ("gui.json", "extra", "colormc.compat.gui"),
    ("window.json", "extra", "colormc.compat.gui"),
    ("cloud.json", "extra", "colormc.compat.extra"),
    ("frp.json", "extra", "colormc.compat.extra"),
    ("lock", "extra", "colormc.compat.lock"),
];

/// 迁移是否正在进行（防重入：迁移期间再点一次不该并发跑第二遍）
static MIGRATING: AtomicBool = AtomicBool::new(false);

/// 迁移标记（运行目录下的 `colormc_migrate.json`）
#[derive(Debug, Serialize, Deserialize)]
struct MigrateMarker {
    /// 用户的选择：`copy` / `move` / `skip`
    #[serde(rename = "Choice")]
    choice: String,
    /// 记录时间（仅用于排查）
    #[serde(rename = "Time")]
    time: String,
}

/// 标记文件路径
fn marker_file() -> PathBuf {
    get_base_dir().join(names::COLORMC_MIGRATE_FILE)
}

/// 是否已经问过（标记文件存在即算问过，任何选择都不再问）
fn asked() -> bool {
    marker_file().is_file()
}

/// 记下"已问过"以及当时的选择
fn mark(choice: &str) -> Result<(), String> {
    let obj = MigrateMarker {
        choice: choice.to_string(),
        time: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
    };
    serialize_tools::json_to_file(&obj, marker_file()).map_err(|err| err.to_string())
}

/// 这个目录**像不像** ColorMC 的工作目录
///
/// 不能只看"目录存在"：只要 ColorMC 跑过一次，`%LOCALAPPDATA%\ColorMC\` 就会存在
/// （里面是账户 `auth.json` 与方块解锁状态 `block.json`，与工作目录在哪无关），
/// 拿它当命中会误报。这里按 ColorMC 真正会写的东西判定：`minecraft/instances`、
/// `minecraft/`、`config.json`、`collect.json` 命中其一即可。
fn looks_like_colormc(dir: &Path) -> bool {
    let mc = dir.join(names::MINECRAFT_DIR);
    mc.join(names::INSTANCE_DIR).is_dir()
        || mc.is_dir()
        || dir.join(names::CONFIG_FILE).is_file()
        || dir.join(names::COLLECT_FILE).is_file()
}

/// 按 ColorMC 自己的顺序给出候选工作目录
///
/// 照 `Program.cs` 的 `Main()`：`%LOCALAPPDATA%\ColorMC\run` 记的路径 → 平台默认
/// （Windows `<exe 目录>\colormc\`、Linux `~/.ColorMC/`、macOS `/Users/shared/ColorMC/`）
/// → `%APPDATA%\ColorMC\`（ColorMC 在默认位置不可写时会退到这里）。
///
/// 与 ColorMC 的差别：**不创建任何目录、也不试写**（ColorMC 会建目录再写 `test` 文件来
/// 探可写性）。这里只是探测，不动用户的东西；代价是"默认位置不存在但 %APPDATA% 有数据"
/// 这种退化情况靠**三个候选挨个查**来覆盖，而不是靠试写。
fn candidates() -> Vec<(PathBuf, &'static str)> {
    let mut list: Vec<(PathBuf, &'static str)> = Vec::new();

    // 1. run 文件里记的路径
    if let Some(local) = dirs::data_local_dir() {
        let run = local.join(COLORMC_DIR).join(COLORMC_RUN_FILE);
        if run.is_file()
            && let Ok(text) = path_helper::read_text(&run)
        {
            let text = text.trim();
            if !text.is_empty() {
                list.push((PathBuf::from(text), "run"));
            }
        }
    }

    // 2. 平台默认位置
    #[cfg(target_os = "windows")]
    let default = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(COLORMC_INNER_DIR)));
    #[cfg(target_os = "linux")]
    let default = dirs::home_dir().map(|home| home.join(".ColorMC"));
    #[cfg(target_os = "macos")]
    let default = Some(PathBuf::from("/Users/shared/ColorMC"));
    if let Some(dir) = default {
        list.push((dir, "default"));
    }

    // 3. 兜底位置
    if let Some(data) = dirs::data_dir() {
        list.push((data.join(COLORMC_DIR), "fallback"));
    }

    list
}

/// 探测本机的 ColorMC 工作目录（第一个"存在且像 ColorMC 目录"的候选）
fn detect() -> Option<(PathBuf, &'static str)> {
    candidates()
        .into_iter()
        .find(|(dir, _)| dir.is_dir() && looks_like_colormc(dir))
}

/// ColorMC 的账户文件位置（**在工作目录之外**，所以永远不会被迁移）
pub fn auth_file() -> Option<PathBuf> {
    let file = dirs::data_local_dir()?.join(COLORMC_DIR).join(names::AUTH_FILE);
    file.is_file().then_some(file)
}

/// 实例根目录（`<工作目录>/minecraft/instances`，两边布局一致）
fn instance_root(root: &Path) -> PathBuf {
    root.join(names::MINECRAFT_DIR).join(names::INSTANCE_DIR)
}

/// 数实例（只数目录，便宜）
fn count_instances(root: &Path) -> u32 {
    let Ok(reader) = fs::read_dir(instance_root(root)) else {
        return 0;
    };
    reader
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .count() as u32
}

/// 实例目录名（有序）
fn list_instances(root: &Path) -> Vec<String> {
    let Ok(reader) = fs::read_dir(instance_root(root)) else {
        return Vec::new();
    };
    let mut list: Vec<String> = reader
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    list.sort();
    list
}

/// 工作目录根下的顶层条目名（有序；弹窗用它列出"将搬入什么"）
fn top_entries(root: &Path) -> Vec<String> {
    let Ok(reader) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut list: Vec<String> = reader
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect();
    list.sort();
    list
}

/// 组装探测结果
fn info(path: PathBuf, from: &'static str) -> ColorMcInfoDto {
    ColorMcInfoDto {
        instances: count_instances(&path),
        entries: top_entries(&path),
        auth_path: auth_file()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        from: from.to_string(),
        path: path.display().to_string(),
    }
}

/// 迁移进度事件（`colormc-progress`）
#[gui_macros::emit]
pub fn emit_colormc_progress(app: &AppHandle, data: ColorMcProgressDto) {
    let _ = app.emit(listens::COLORMC_PROGRESS, data);
}

/// 推一次进度
///
/// `app` 为 `None` 时不推（测试里没有界面句柄）—— 进度是"有界面就报"的东西，
/// 不该为了测试往引擎里塞一个假的 AppHandle，所以它一路是 `Option`。
fn progress(app: Option<&AppHandle>, stage: &str, done: u32, total: u32, text: &str) {
    let Some(app) = app else {
        return;
    };
    emit_colormc_progress(
        app,
        ColorMcProgressDto {
            stage: stage.to_string(),
            done,
            total,
            text: text.to_string(),
        },
    );
}

/// 检查本机有没有可迁移的 ColorMC 数据（**问过了就返回 `None`**）
///
/// 主窗口启动后调一次；返回 `Some` 时前端弹"要不要把数据搬过来"的三选一弹窗。
#[tauri::command]
pub fn window_check_colormc() -> Option<ColorMcInfoDto> {
    if asked() {
        return None;
    }
    let (path, from) = detect()?;
    // 探测结果就是运行目录（不该发生）：没什么可搬的，别弹窗
    if path == get_base_dir() {
        return None;
    }
    mml_log::info(format!(
        "colormc detected: {} (from={from})",
        path.display()
    ));
    Some(info(path, from))
}

/// 记下"不迁移"：以后不再问
#[tauri::command]
pub fn window_skip_colormc() -> Result<(), String> {
    mark("skip")?;
    mml_log::info(String::from("colormc migrate skipped by user"));
    Ok(())
}

/// 执行迁移并返回兼容性报告
///
/// - `source`: ColorMC 工作目录（前端把 [`window_check_colormc`] 给的那份传回来）
/// - `mode`: `"copy"` 复制（源目录保留）/ `"move"` 移动（源目录搬空后删除）
///
/// 跑在阻塞线程池上（可能搬几十 GB），期间用 `colormc-progress` 事件报进度。
#[tauri::command]
pub async fn window_migrate_colormc(
    app: AppHandle,
    source: String,
    mode: String,
) -> Result<ColorMcReportDto, String> {
    let moved = match mode.as_str() {
        "copy" => false,
        "move" => true,
        _ => return Err(String::from("err.colormcMode")),
    };

    if MIGRATING.swap(true, Ordering::AcqRel) {
        return Err(String::from("err.colormcBusy"));
    }

    let handle = app.clone();
    let dst = get_base_dir();
    let result = tauri::async_runtime::spawn_blocking(move || {
        let src = PathBuf::from(source.trim());
        migrate(Some(&handle), &src, &dst, moved)
    })
    .await;

    MIGRATING.store(false, Ordering::Release);

    match result {
        Ok(Ok(report)) => {
            // 只在**迁移成功后**记"问过了"（写不上只记日志：数据已经搬完，
            // 不该因为一个标记文件把整次迁移报成失败，顶多下次启动再问一遍）
            if let Err(err) = mark(if moved { "move" } else { "copy" }) {
                mml_log::error(format!("colormc marker write failed: {err}"));
            }
            Ok(report)
        }
        Ok(Err(err)) => Err(err),
        Err(err) => Err(format!("err.colormcTask: {err}")),
    }
}

/// 迁移主流程（同步、在阻塞线程里跑）
///
/// 目标目录是**参数**（命令传运行目录，测试传自己的临时目录）—— 引擎里不去摸全局单例，
/// 既能测，也避免了"引擎偷偷决定搬到哪"。
fn migrate(
    app: Option<&AppHandle>,
    src: &Path,
    dst: &Path,
    moved: bool,
) -> Result<ColorMcReportDto, String> {
    // 源目录必须是"真的 ColorMC 工作目录"：调用方传来的路径不能当数
    if !src.is_dir() || !looks_like_colormc(src) {
        return Err(String::from("err.colormcSource"));
    }
    if src == dst {
        return Err(String::from("err.colormcSameDir"));
    }
    // 运行目录在源目录里面：复制会递归地把自己再拷一遍，直接拒绝
    if dst.starts_with(src) {
        return Err(String::from("err.colormcNested"));
    }

    // 实例名要在搬运前收集：移动模式下源目录随后就没了
    let src_instances = list_instances(src);
    let src_entries = top_entries(src);
    if src_entries.is_empty() {
        return Err(String::from("err.colormcEmpty"));
    }

    mml_log::info(format!(
        "colormc migrate start: {} -> {} (mode={})",
        src.display(),
        dst.display(),
        if moved { "move" } else { "copy" }
    ));

    // ---- 1. 搬运 ----
    // `failed` 里可能有**嵌套**的失败项（某个文件被占用），所以成功条目数单独数，
    // 不能用"总数 - failed.len()"倒推
    let mut failed: Vec<String> = Vec::new();
    let mut ok_entries: u32 = 0;

    if moved {
        // 移动：按顶层条目重命名（跨盘回退复制 + 删源）
        let total = src_entries.len() as u32;
        progress(app, "move", 0, total, "");
        let mut done: u32 = 0;
        for name in &src_entries {
            let from = src.join(name);
            let to = dst.join(name);
            match move_entry(app, &from, &to, name, &mut done, total) {
                Ok(()) => ok_entries += 1,
                Err(err) => {
                    mml_log::error(format!("colormc move failed: {name} ({err})"));
                    failed.push(name.clone());
                }
            }
        }
        // 全部条目都搬完（源目录空了）才删掉它；有失败就留着
        let empty = fs::read_dir(src)
            .map(|mut reader| reader.next().is_none())
            .unwrap_or(false);
        if empty {
            if let Err(err) = fs::remove_dir_all(src) {
                mml_log::error(format!("colormc remove source failed: {err}"));
            }
        } else {
            mml_log::info(String::from("colormc move: source kept (not empty)"));
        }
    } else {
        // 复制：先数一遍文件数（进度用），再逐文件复制
        progress(app, "scan", 0, 0, "");
        let total = count_files(src);
        progress(app, "copy", 0, total, "");
        let mut done: u32 = 0;
        for name in &src_entries {
            let from = src.join(name);
            let to = dst.join(name);
            match copy_entry(app, "copy", &from, &to, name, &mut done, total, &mut failed) {
                Ok(()) => ok_entries += 1,
                Err(err) => {
                    mml_log::error(format!("colormc copy failed: {name} ({err})"));
                    failed.push(name.clone());
                }
            }
        }
    }

    // ---- 2. 兼容性检测 ----
    progress(app, "check", 0, src_instances.len() as u32, "");
    let checked = check(app, &src_entries, &src_instances, dst);

    mml_log::info(format!(
        "colormc migrate done: entries={ok_entries} failed={} instances={}/{}",
        failed.len(),
        checked.instances_ok.len(),
        checked.instances_ok.len() + checked.instances_bad.len()
    ));

    Ok(ColorMcReportDto {
        moved,
        source: src.display().to_string(),
        entries: ok_entries,
        failed,
        instances_ok: checked.instances_ok,
        instances_bad: checked.instances_bad,
        instances_legacy_gui: checked.instances_legacy_gui,
        compat: checked.compat,
        auth_outside: auth_file().is_some(),
    })
}

/// 递归数文件（复制模式的进度总量）
fn count_files(dir: &Path) -> u32 {
    let Ok(reader) = fs::read_dir(dir) else {
        return 0;
    };
    let mut total = 0;
    for entry in reader.flatten() {
        let path = entry.path();
        if path.is_dir() {
            total += count_files(&path);
        } else {
            total += 1;
        }
    }
    total
}

/// 删掉一个条目（文件或目录）—— 移动前清掉目标位置的同名条目（Windows 的 rename 不覆盖）
fn remove_entry(path: &Path) -> io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else if path.is_file() {
        fs::remove_file(path)
    } else {
        Ok(())
    }
}

/// 复制一个条目（目录递归、文件直接复制；同名一律覆盖）
///
/// 单个文件失败只记进 `failed` 并继续：占用 / 无权限不该毁掉整次迁移。
/// `stage` 只是进度事件里报的阶段名（移动的跨盘回退也走这里，那时报 `move`）。
#[allow(clippy::too_many_arguments)]
fn copy_entry(
    app: Option<&AppHandle>,
    stage: &str,
    from: &Path,
    to: &Path,
    rel: &str,
    done: &mut u32,
    total: u32,
    failed: &mut Vec<String>,
) -> Result<(), String> {
    if from.is_dir() {
        path_helper::create_dir_all(to).map_err(|err| err.to_string())?;
        let reader = fs::read_dir(from).map_err(|err| err.to_string())?;
        for entry in reader.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let child_rel = format!("{rel}/{name}");
            if let Err(err) = copy_entry(
                app,
                stage,
                &entry.path(),
                &to.join(&name),
                &child_rel,
                done,
                total,
                failed,
            ) {
                mml_log::error(format!("colormc copy skip: {child_rel} ({err})"));
                failed.push(child_rel.clone());
            }
        }
        return Ok(());
    }

    let result = path_helper::copy_file(from, to);
    *done += 1;
    progress(app, stage, *done, total, rel);
    result.map_err(|err| err.to_string())
}

/// 移动一个顶层条目
///
/// 先 `rename`（同盘瞬时）；跨盘时回退成"复制 + 删源"（[`path_helper::move_file`] 也是这个
/// 策略，但这里要按文件报进度，所以自己走一遍 [`copy_entry`]）。
fn move_entry(
    app: Option<&AppHandle>,
    from: &Path,
    to: &Path,
    rel: &str,
    done: &mut u32,
    total: u32,
) -> Result<(), String> {
    // "一律覆盖"：目标已存在就先删掉它
    if to.exists() {
        remove_entry(to).map_err(|err| err.to_string())?;
    }

    match fs::rename(from, to) {
        Ok(()) => {
            *done += 1;
            progress(app, "move", *done, total, rel);
            Ok(())
        }
        Err(err) if err.kind() == io::ErrorKind::CrossesDevices => {
            let mut failed: Vec<String> = Vec::new();
            let mut files: u32 = 0;
            // 跨盘只能真的复制：进度总量未知（0 = 未知），阶段名仍报 move
            copy_entry(app, "move", from, to, rel, &mut files, 0, &mut failed)?;
            remove_entry(from).map_err(|err| err.to_string())?;
            *done += 1;
            progress(app, "move", *done, total, rel);
            Ok(())
        }
        Err(err) => Err(err.to_string()),
    }
}

/// 兼容性检测的结果
struct Checked {
    instances_ok: Vec<String>,
    instances_bad: Vec<String>,
    instances_legacy_gui: Vec<String>,
    compat: Vec<ColorMcCompatDto>,
}

/// 兼容性检测：把"搬过来的东西 M²L 到底能不能用"逐条判定
///
/// - 顶层条目查 [`COMPAT_TABLE`]（没列进去的按 `extra` 处理）；
/// - 实例逐个用 M²L 的解析器读 `game.json` —— 这是真正的判据：M²L 只认"目录里有能解析的
///   `game.json`"的实例，所以解析失败就等于"搬过来了也看不见"；
/// - 实例里还留着 ColorMC 的 `guisetting.json` 时单独记一笔：M²L 读的是自己的
///   `gui_setting.json`（见 `crate::gui_setting`），所以 ColorMC 那份里的模组分组 / 备注 /
///   日志设置**不会跟着过来**（文件本身留着，将来要转换还有原数据）。
fn check(
    app: Option<&AppHandle>,
    src_entries: &[String],
    src_instances: &[String],
    dst: &Path,
) -> Checked {
    let mut checked = Checked {
        instances_ok: Vec::new(),
        instances_bad: Vec::new(),
        instances_legacy_gui: Vec::new(),
        compat: Vec::new(),
    };

    // 逐条给结论
    for name in src_entries {
        let (level, note) = COMPAT_TABLE
            .iter()
            .find(|(entry, _, _)| entry == name)
            .map(|(_, level, note)| (*level, *note))
            .unwrap_or(("extra", "colormc.compat.extra"));
        checked.compat.push(ColorMcCompatDto {
            name: name.clone(),
            level: level.to_string(),
            note: note.to_string(),
        });
    }

    // 实例逐个验：能不能被 M²L 读出来
    let root = instance_root(dst);
    let total = src_instances.len() as u32;
    for (index, name) in src_instances.iter().enumerate() {
        let dir = root.join(name);
        let game = dir.join(names::GAME_FILE);
        if serialize_tools::json_from_file::<InstanceSettingObj>(&game).is_ok() {
            if dir.join(COLORMC_GUI_SETTING_FILE).is_file() {
                checked.instances_legacy_gui.push(name.clone());
            }
            checked.instances_ok.push(name.clone());
        } else {
            checked.instances_bad.push(name.clone());
        }
        progress(app, "check", index as u32 + 1, total, name);
    }

    checked
}

#[cfg(test)]
mod tests {
    //! 迁移引擎的用例
    //!
    //! 数据两种：
    //! - **真实样本** `tests/colormc/`（真 ColorMC 工作目录裁出来的一份，见那边的 README）——
    //!   验"真实的 ColorMC 目录能被识别、里面的实例 M²L 读得出来、结论表覆盖齐全"；
    //! - **临时小目录**：边界情况（覆盖同名文件、拒绝套娃路径、只有 auth.json 的目录）现场造。
    //!
    //! 全部在 `target/temp` 下操作（AGENTS.md §6），而且**只读**真实样本 —— 移动用例先把样本
    //! 复制一份再搬，反复跑也不会把样本搬没。

    use super::*;
    use crate::test_support::{cleanup, ensure_boot, temp_dir};

    /// 真实样本目录（`mml-gui/src-tauri/tests/colormc`）
    fn fixture() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("colormc")
    }

    /// 兼容性结论的查表助手
    fn level_of(checked: &Checked, name: &str) -> String {
        checked
            .compat
            .iter()
            .find(|item| item.name == name)
            .map(|item| item.level.clone())
            .unwrap_or_default()
    }

    /// 真实样本：能识别成 ColorMC 工作目录，三个实例 M²L 都读得出来
    #[test]
    fn real_fixture_is_recognized_and_readable() {
        ensure_boot();
        let src = fixture();
        assert!(src.is_dir(), "缺少真实样本目录：{}", src.display());

        // 识别靠 `minecraft/instances` 命中，而不是"目录存在"或里面有几个 json
        assert!(looks_like_colormc(&src));

        // 三个真实实例都在
        let instances = list_instances(&src);
        assert_eq!(instances.len(), 3, "实际：{instances:?}");
        assert!(
            instances.contains(&String::from("1.21.11-NeoForge-21.11.38-beta")),
            "实际：{instances:?}"
        );

        // 以样本目录自身为"目标"只读地跑兼容性检测：真实实例不该读不出来
        let entries = top_entries(&src);
        let checked = check(None, &entries, &instances, &src);
        assert!(
            checked.instances_bad.is_empty(),
            "真实实例读不出来：{:?}",
            checked.instances_bad
        );
        assert_eq!(checked.instances_ok.len(), 3);
        // 那个 NeoForge 实例带着 ColorMC 的 guisetting.json：M²L 不读它，报告里要提一句
        assert_eq!(
            checked.instances_legacy_gui,
            vec![String::from("1.21.11-NeoForge-21.11.38-beta")]
        );

        // 顶层结论：minecraft 可用、config.json 有偏差、用不到的按 extra
        assert_eq!(level_of(&checked, names::MINECRAFT_DIR), "ok");
        assert_eq!(level_of(&checked, names::CONFIG_FILE), "partial");
        assert_eq!(level_of(&checked, names::COLLECT_FILE), "ok");
        assert_eq!(level_of(&checked, "lock"), "extra");
    }

    /// 兼容性结论要覆盖真实工作目录的**全部**顶层条目
    ///
    /// 样本里只搬了"有内容且小"的那些；`download` / `block` / `java` 之类太大没入仓，
    /// 这里按真实目录的名字建同名空目录来验结论表。
    #[test]
    fn compat_covers_all_real_top_level_entries() {
        ensure_boot();
        let src = temp_dir("compat");
        for name in [
            "minecraft",
            "download",
            "block",
            "image",
            "java",
            "tools",
            "frpc",
            "inputs",
        ] {
            fs::create_dir_all(src.join(name)).unwrap();
        }
        for name in [
            "cloud.json",
            "collect.json",
            "config.json",
            "count.dat",
            "frp.json",
            "gui.json",
            "lock",
            "logs.log",
            "window.json",
        ] {
            fs::write(src.join(name), "").unwrap();
        }

        let entries = top_entries(&src);
        let checked = check(None, &entries, &[], &src);

        // 能直接用
        assert_eq!(level_of(&checked, "minecraft"), "ok");
        assert_eq!(level_of(&checked, "collect.json"), "ok");
        // 能用但有偏差 / 会丢东西
        assert_eq!(level_of(&checked, "config.json"), "partial");
        assert_eq!(level_of(&checked, "count.dat"), "partial");
        assert_eq!(level_of(&checked, "logs.log"), "partial");
        assert_eq!(level_of(&checked, "java"), "partial");
        // M²L 读不懂（留着不碍事）——包括表里没列到的
        for name in [
            "download",
            "block",
            "image",
            "tools",
            "frpc",
            "inputs",
            "gui.json",
            "window.json",
            "lock",
            "cloud.json",
            "frp.json",
        ] {
            assert_eq!(level_of(&checked, name), "extra", "{name} 该归 extra");
        }

        cleanup(&[&src]);
    }

    /// 复制真实样本：源目录原样保留，目标拿到实例且仍然可用
    #[test]
    fn copy_real_fixture_keeps_source_and_instances_usable() {
        ensure_boot();
        let src = fixture();
        let dst = temp_dir("copy");

        let report = migrate(None, &src, &dst, false).unwrap();

        assert!(!report.moved);
        assert!(src.join(names::CONFIG_FILE).is_file(), "复制不该动源目录");
        assert!(dst.join(names::CONFIG_FILE).is_file());
        assert!(report.failed.is_empty(), "不该有失败项：{:?}", report.failed);
        assert_eq!(
            report.instances_ok.len(),
            3,
            "搬过去的实例应全部可用，认不出的：{:?}",
            report.instances_bad
        );

        // 实例里的文件真的搬过去了（不是只建了目录），而且还能被 M²L 解析
        let game = dst
            .join(names::MINECRAFT_DIR)
            .join(names::INSTANCE_DIR)
            .join("1.21.11-NeoForge-21.11.38-beta")
            .join(names::GAME_FILE);
        assert!(game.is_file());
        assert!(serialize_tools::json_from_file::<InstanceSettingObj>(&game).is_ok());

        cleanup(&[&dst]);
    }

    /// 移动：条目被重命名过去，搬空后源目录被删
    ///
    /// **先复制再移动** —— 真实样本不能被搬走（用例要能反复跑）。
    #[test]
    fn move_renames_entries_and_removes_source() {
        ensure_boot();
        let staging = temp_dir("move-src");
        let mut done: u32 = 0;
        let mut failed: Vec<String> = Vec::new();
        copy_entry(
            None,
            "copy",
            &fixture(),
            &staging,
            "",
            &mut done,
            0,
            &mut failed,
        )
        .unwrap();
        assert!(failed.is_empty(), "准备阶段就不该失败：{failed:?}");

        let dst = temp_dir("move-dst");
        let report = migrate(None, &staging, &dst, true).unwrap();

        assert!(report.moved);
        assert!(!staging.exists(), "搬空后源目录该被删掉");
        assert!(dst.join(names::CONFIG_FILE).is_file());
        assert_eq!(report.instances_ok.len(), 3);

        cleanup(&[&dst]);
    }

    /// 复制时目标里的同名文件被覆盖（用户选的是"一律覆盖"）
    #[test]
    fn copy_overwrites_same_named_file() {
        ensure_boot();
        let src = temp_dir("over-src");
        fs::create_dir_all(src.join(names::MINECRAFT_DIR).join(names::INSTANCE_DIR)).unwrap();
        fs::write(src.join(names::CONFIG_FILE), "{\"Version\":\"from-colormc\"}").unwrap();

        let dst = temp_dir("over-dst");
        fs::write(dst.join(names::CONFIG_FILE), "{\"Version\":\"from-mml\"}").unwrap();

        migrate(None, &src, &dst, false).unwrap();

        let text = fs::read_to_string(dst.join(names::CONFIG_FILE)).unwrap();
        assert!(text.contains("from-colormc"), "同名文件该被覆盖，实际：{text}");

        cleanup(&[&src, &dst]);
    }

    /// 拒绝：目标 = 源 / 目标在源目录里面 / 目录不像 ColorMC 工作目录
    #[test]
    fn rejects_bad_source_and_nested_target() {
        ensure_boot();
        let src = temp_dir("bad-src");
        fs::create_dir_all(src.join(names::MINECRAFT_DIR).join(names::INSTANCE_DIR)).unwrap();
        let dst = temp_dir("bad-dst");

        assert!(
            migrate(None, &src, &src, false).is_err(),
            "目标就是源目录，该拒绝"
        );
        assert!(
            migrate(None, &src, &src.join("inner"), false).is_err(),
            "目标在源目录里面（会递归复制自己），该拒绝"
        );

        // 不像 ColorMC 的空目录
        let empty = temp_dir("bad-empty");
        assert!(migrate(None, &empty, &dst, false).is_err());

        cleanup(&[&src, &dst, &empty]);
    }

    /// 探测判据：只有 `auth.json` 的目录（`%LOCALAPPDATA%\ColorMC` 那种）**不算**工作目录
    ///
    /// ColorMC 只要跑过一次就会建出那个目录（里面是账户与方块解锁状态，与工作目录在哪无关），
    /// 拿它当命中会误报 —— 这条用例把这个坑钉住。
    #[test]
    fn only_auth_json_is_not_a_working_dir() {
        ensure_boot();
        let dir = temp_dir("looks");
        fs::write(dir.join(names::AUTH_FILE), "[]").unwrap();
        assert!(!looks_like_colormc(&dir), "只有 auth.json 不该算工作目录");

        fs::create_dir_all(dir.join(names::MINECRAFT_DIR).join(names::INSTANCE_DIR)).unwrap();
        assert!(looks_like_colormc(&dir), "有 minecraft/instances 才算");

        cleanup(&[&dir]);
    }

    /// 选「不迁移」会写下标记：之后就是"问过了"（重启也不弹）
    ///
    /// 标记落在**运行目录**（测试里是 `test_support` boot 时那个临时目录）。
    /// 这条同时把"标记文件是唯一判据"钉住：删掉它就能重新触发询问。
    #[test]
    fn skip_writes_marker_and_stops_asking() {
        ensure_boot();
        let file = get_base_dir().join(names::COLORMC_MIGRATE_FILE);
        let _ = fs::remove_file(&file);
        assert!(!asked(), "标记文件在用例开始时该不存在");

        window_skip_colormc().unwrap();

        assert!(file.is_file(), "「不迁移」该写下标记：{}", file.display());
        assert!(asked(), "写过标记之后就算问过了");
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.contains("skip"), "标记里该记下这次的选择，实际：{text}");

        let _ = fs::remove_file(&file);
        assert!(!asked(), "删掉标记就该重新问");
    }
}
