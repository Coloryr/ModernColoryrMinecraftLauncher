pub mod i18;
pub mod i18_items;
pub mod names;
pub mod uuids;

use std::{
    env,
    fs::File,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{LazyLock, OnceLock, RwLock},
};

use serde::{Deserialize, Serialize};

use crate::{
    i18::{I18Lang, en_us::EnUs, zh_cn::ZhCn},
    i18_items::error_type::{CoreResult, ErrorType::FileSystemError, FileSystemErrorData},
    names::{LANG_EN_US, LANG_ZH_CN},
};

/// 启动器主版本号
pub const VERSION_NUM: i32 = 1;
/// 启动器日期
pub const DATE: &str = "20260831";
/// 启动器版本号
pub const VERSION: LazyLock<String> = LazyLock::new(|| format!("1.{}.{DATE}", VERSION_NUM));

#[allow(non_camel_case_types)]
#[derive(Clone, Copy, Serialize, Deserialize, Debug, PartialEq, Eq, Hash)]
pub enum Lang {
    zh_cn,
    en_us,
}

impl Default for Lang {
    fn default() -> Self {
        Lang::zh_cn
    }
}

/// 加载的语言
static LANG: LazyLock<RwLock<Lang>> = LazyLock::new(|| {
    let local = get_current_locale();
    let lang = check_lang(&local);
    load_lang(lang);
    RwLock::new(lang)
});
/// 语言配置
static FILE: OnceLock<PathBuf> = OnceLock::new();

static LINE_ENDING: LazyLock<String> =
    LazyLock::new(|| String::from(if cfg!(windows) { "\r\n" } else { "\n" }));

/// 获取换行符
pub fn get_line_ending() -> String {
    LINE_ENDING.clone()
}

/// 获取本地语言
pub fn get_current_locale() -> String {
    if let Ok(lang) = env::var("LANG") {
        lang
    } else if let Ok(lang) = env::var("LC_ALL") {
        lang
    } else if let Ok(lang) = env::var("LANGUAGE") {
        lang
    } else {
        String::from(LANG_ZH_CN)
    }
}

/// 获取语言
pub fn get_lang(lang: Lang) -> &'static str {
    match lang {
        Lang::zh_cn => LANG_ZH_CN,
        Lang::en_us => LANG_EN_US,
    }
}

/// 从字符串判断语言类型
fn check_lang(data: &String) -> Lang {
    if data.eq(LANG_ZH_CN) {
        return Lang::zh_cn;
    } else if data.eq(LANG_EN_US) {
        return Lang::en_us;
    }

    return Lang::zh_cn;
}

/// 加载语言
fn load_lang(lang: Lang) {
    let i18: Box<dyn I18Lang + Send + Sync> = match lang {
        Lang::zh_cn => Box::new(ZhCn),
        Lang::en_us => Box::new(EnUs),
    };

    i18::set(i18);
}

/// 从文件加载语言类型
fn load<P: AsRef<Path>>(file: P) -> CoreResult<()> {
    let mut stream = File::open(file.as_ref()).map_err(|err| {
        FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;
    let mut str = String::new();
    stream.read_to_string(&mut str).map_err(|err| {
        FileSystemError(FileSystemErrorData {
            path: file.as_ref().to_path_buf(),
            error: err.to_string(),
        })
    })?;
    let lang = check_lang(&str);
    *LANG.write().unwrap() = lang;
    load_lang(lang);

    Ok(())
}

/// 保存语言类型
fn save() -> CoreResult<()> {
    let data = LANG.read().unwrap();
    let str = get_lang(*data);

    let file = FILE.get().unwrap();
    let mut stream = File::create(file).map_err(|err| {
        FileSystemError(FileSystemErrorData {
            path: file.to_path_buf(),
            error: err.to_string(),
        })
    })?;
    stream.write_all(str.as_bytes()).map_err(|err| {
        FileSystemError(FileSystemErrorData {
            path: file.to_path_buf(),
            error: err.to_string(),
        })
    })?;

    Ok(())
}

/// 获取语言类型
pub fn get_lang_type() -> Lang {
    LANG.read().unwrap().clone()
}

/// 设置语言类型
pub fn set_lang(lang: Lang) -> CoreResult<()> {
    *LANG.write().unwrap() = lang;
    save()?;

    load_lang(lang);

    Ok(())
}

/// 初始化语言
pub fn init<P: AsRef<Path>>(path: P) -> CoreResult<()> {
    // 读取文件语言
    let file = path.as_ref().with_file_name(names::LANG_FILE);
    let file = FILE.get_or_init(|| file);

    if file.exists() {
        load(file)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        i18_items::{
            error_type::ErrorType, info_type::InfoType, panic_type::PanicType,
            thread_type::ThreadType,
        },
        names,
    };
    use std::fs;

    /// i18 文案与语言加载生命周期测试
    ///
    /// `LANG` 与 `i18::I18` 都是进程级全局状态，且 `EnUs` 的文案实现
    /// 大量使用 `todo!()`（调用即 panic），因此所有涉及全局状态的操作
    /// 统一放在这一个测试里顺序执行，避免与其他测试并发时互相干扰。
    #[test]
    fn lang_and_i18_lifecycle() {
        // ---------- i18 文案（强制使用中文实现，不依赖系统语言） ----------
        i18::set(Box::new(i18::zh_cn::ZhCn));

        // 线程名
        assert_eq!(
            i18::get_thread(ThreadType::LogThread),
            String::from("日志线程")
        );
        assert_eq!(
            i18::get_thread(ThreadType::ConfigSaveThread),
            String::from("配置保存线程")
        );

        // 信息文案（CoreStart 中包含版本号）
        assert_eq!(
            i18::get_info(InfoType::CoreStart),
            format!("MCML启动，版本：{}", *VERSION)
        );
        assert_eq!(i18::get_info(InfoType::TempFile), String::from("临时文件"));

        // 错误文案（带参数的格式化）
        let err = ErrorType::ConfigSaveError(FileSystemErrorData {
            path: PathBuf::from("a.json"),
            error: String::from("boom"),
        });
        assert_eq!(
            i18::get_error(err.clone()),
            String::from("配置文件 a.json 保存失败：boom")
        );

        // Display 实现（走同一条 i18 路径）
        assert_eq!(err.to_string(), "配置文件 a.json 保存失败：boom");
        assert_eq!(ErrorType::TaskCancel.to_string(), "任务已取消");
        assert_eq!(ErrorType::TaskTimeout.to_string(), "任务执行超时");

        // 严重错误文案
        assert_eq!(
            i18::get_panic(PanicType::CoreArgLocalEmpty),
            String::from("运行路径为空")
        );
        assert_eq!(
            i18::get_panic(PanicType::LogOpenFail(
                String::from("C:/logs"),
                String::from("denied")
            )),
            String::from("日志系统初始化失败：denied 路径：C:/logs")
        );

        // ---------- 语言文件加载 ----------
        // 使用临时目录，测完清理
        let dir = std::env::temp_dir().join(format!("mcml_names_test_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let lang_file = dir.join(names::LANG_FILE);
        // 清掉上次运行可能残留的语言文件
        let _ = fs::remove_file(&lang_file);

        // init 传入的是一个文件路径，语言文件取其同目录下的 lang.txt
        let init_path = dir.join("notexist.json");
        init(&init_path).unwrap();
        // 语言文件不存在时不应被创建
        assert!(!lang_file.exists());

        // 写入 en_US 后重新 init，应切换到英文
        fs::write(&lang_file, names::LANG_EN_US).unwrap();
        init(&init_path).unwrap();
        assert!(matches!(get_lang_type(), Lang::en_us));

        // set_lang 切回中文，同时应把语言写回文件
        set_lang(Lang::zh_cn).unwrap();
        assert!(matches!(get_lang_type(), Lang::zh_cn));
        assert_eq!(fs::read_to_string(&lang_file).unwrap(), names::LANG_ZH_CN);

        // 未知语言字符串应回退为中文
        fs::write(&lang_file, "fr_FR").unwrap();
        init(&init_path).unwrap();
        assert!(matches!(get_lang_type(), Lang::zh_cn));

        // 清理临时目录
        let _ = fs::remove_dir_all(&dir);
    }

    /// 语言字符串判断（纯函数）
    #[test]
    fn check_lang_strings() {
        assert!(matches!(check_lang(&String::from("zh_CN")), Lang::zh_cn));
        assert!(matches!(check_lang(&String::from("en_US")), Lang::en_us));
        // 不认识的语言回退为中文
        assert!(matches!(check_lang(&String::from("fr_FR")), Lang::zh_cn));
    }

    /// get_lang 与语言常量一一对应
    #[test]
    fn get_lang_map() {
        assert_eq!(get_lang(Lang::zh_cn), names::LANG_ZH_CN);
        assert_eq!(get_lang(Lang::en_us), names::LANG_EN_US);
    }

    /// Lang 的默认值是中文
    #[test]
    fn lang_default() {
        assert!(matches!(Lang::default(), Lang::zh_cn));
    }

    /// 获取本地语言（只读环境变量，不做修改，仅检查返回值非空）
    #[test]
    fn current_locale_non_empty() {
        assert!(!get_current_locale().is_empty());
    }

    /// 换行符应与编译目标平台一致
    #[test]
    fn line_ending_matches_platform() {
        let expected = if cfg!(windows) { "\r\n" } else { "\n" };
        assert_eq!(get_line_ending(), expected);
    }

    /// 版本号拼接格式
    #[test]
    fn version_format() {
        assert_eq!(*VERSION, format!("1.{}.{}", VERSION_NUM, DATE));
        assert!(!DATE.is_empty());
    }
}
