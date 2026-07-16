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

use crate::{
    i18::{I18Lang, en_us::EnUs, zh_cn::ZhCn},
    names::{LANG_EN_US, LANG_ZH_CN},
};

/// 启动器主版本号
pub const VERSION_NUM: i32 = 1;
/// 启动器日期
pub const DATE: &str = "20260714";
/// 启动器版本号
pub const VERSION: LazyLock<String> = LazyLock::new(|| format!("1.{}.{DATE}", VERSION_NUM));

#[allow(non_camel_case_types)]
#[derive(Clone, Copy)]
pub enum Lang {
    zh_cn,
    en_us,
}

/// 加载的语言
static LANG: OnceLock<RwLock<Lang>> = OnceLock::new();
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
fn load_lang() {
    let i18: Box<dyn I18Lang + Send + Sync> = match *LANG.get().unwrap().read().unwrap() {
        Lang::zh_cn => Box::new(ZhCn),
        Lang::en_us => Box::new(EnUs),
    };

    i18::set(i18);
}

/// 从文件加载语言类型
fn load(file: &PathBuf) {
    let mut file = File::open(file).unwrap();
    let mut str = String::new();
    file.read_to_string(&mut str).unwrap();
    LANG.get_or_init(|| RwLock::new(check_lang(&str)));
}

/// 保存语言类型
fn save() {
    let data = LANG.get().unwrap().read().unwrap();
    let str = get_lang(*data);

    let mut file = File::create(FILE.get().unwrap()).unwrap();
    file.write_all(str.as_bytes()).unwrap();
}

/// 获取语言类型
pub fn get_lang_type() -> Lang {
    LANG.get().unwrap().read().unwrap().clone()
}

/// 设置语言类型
pub fn set_lang(lang: Lang) {
    *LANG.get().unwrap().write().unwrap() = lang;
    save();

    load_lang();
}

/// 初始化语言
pub fn init<P: AsRef<Path>>(path: P) {
    let file = path.as_ref().with_file_name(names::LANG_FILE);
    let file = FILE.get_or_init(|| file);

    if file.exists() {
        load(file);
    } else {
        let local = get_current_locale();

        LANG.get_or_init(|| RwLock::new(check_lang(&local)));

        save();
    }

    load_lang();
}
