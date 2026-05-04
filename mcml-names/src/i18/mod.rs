use std::sync::{OnceLock, RwLock};

use crate::{error_type::ErrorType, info_type::InfoType, panic_type::PanicType, thread_type::ThreadType};

pub mod zh_cn;
pub mod en_us;

pub trait I18Lang {
    fn get_info(&self, info: InfoType) -> String;
    fn get_error(&self, error: ErrorType) -> String;
    fn get_panic(&self, panic: PanicType) -> String;
    fn get_thread(&self, thread: ThreadType) -> String;
}

static I18: OnceLock<RwLock<Box<dyn I18Lang + Send + Sync>>> = OnceLock::new();

pub fn set(lang_impl: Box<dyn I18Lang + Send + Sync>) {
    let data = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));

    *data.write().unwrap() = lang_impl;
}

pub fn get_info(info: InfoType) -> String {
    let i18 = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));
    let lang = i18.read().unwrap();
    lang.get_info(info)
}

pub fn get_error(error: ErrorType) -> String {
    let i18 = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));
    let lang = i18.read().unwrap();
    lang.get_error(error)
}

pub fn get_panic(panic: PanicType) -> String {
    let i18 = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));
    let lang = i18.read().unwrap();
    lang.get_panic(panic)
}

pub fn get_thread(thread: ThreadType) -> String {
    let i18 = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));
    let lang = i18.read().unwrap();
    lang.get_thread(thread)
}
