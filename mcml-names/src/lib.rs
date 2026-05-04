pub mod error_type;
pub mod i18;
pub mod info_type;
pub mod names;
pub mod os;
pub mod panic_type;
pub mod thread_type;

use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    sync::{OnceLock, RwLock},
};

use const_format::formatcp;

use crate::i18::{I18Lang, en_us::EnUs, zh_cn::ZhCn};

/// 启动器主版本号
pub const VERSION_NUM: i32 = 1;
/// 启动器日期
pub const DATE: &str = "20260503";
/// 启动器版本号
pub const VERSION: &str = formatcp!("1.{}.{DATE}", VERSION_NUM);

#[derive(Clone, Copy)]
pub enum Lang {
    ZhCn,
    EnUs,
}

static LANG: OnceLock<RwLock<Lang>> = OnceLock::new();
static FILE: OnceLock<PathBuf> = OnceLock::new();

fn get_lang(lang: Lang) -> &'static str {
    match lang {
        Lang::ZhCn => "zh_CN",
        Lang::EnUs => "en_US",
    }
}

fn check_lang(data: &String) -> Lang {
    if data.eq("zh_CN") {
        return Lang::ZhCn;
    } else if data.eq("en_US") {
        return Lang::EnUs;
    }

    return Lang::EnUs;
}

fn save() {
    let data = LANG.get().unwrap().read().unwrap();
    let str = get_lang(*data);

    let mut file = File::create(FILE.get().unwrap()).unwrap();
    file.write_all(str.as_bytes()).unwrap();
}

pub fn init(local: &PathBuf) {
    let file = local.with_file_name(names::NAME_LANG_FILE);

    FILE.get_or_init(|| file);
    let file = FILE.get().unwrap();

    if file.exists() {
        let mut file = File::open(file).unwrap();
        let mut str = String::new();
        file.read_to_string(&mut str).unwrap();

        LANG.get_or_init(|| RwLock::new(check_lang(&str)));
    } else {
        LANG.get_or_init(|| RwLock::new(Lang::ZhCn));

        save();
    }

    let i18: Box<dyn I18Lang + Send + Sync> = match *LANG.get().unwrap().read().unwrap() {
        Lang::ZhCn => Box::new(ZhCn),
        Lang::EnUs => Box::new(EnUs),
    };

    i18::set(i18);
}
