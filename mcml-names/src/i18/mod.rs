use std::sync::{OnceLock, RwLock};

use crate::{error_type::ErrorType, info_type::InfoType};

pub mod zh_cn;
pub mod en_us;

pub trait I18Lang {
    fn get_info(&self, info: InfoType) -> String;
    fn get_error(&self, error: ErrorType) -> String;
}

struct EmptyLang;

impl I18Lang for EmptyLang {
    fn get_info(&self, _info: InfoType) -> String {
        String::new()
    }

    fn get_error(&self, _error: ErrorType) -> String {
        String::new()
    }
}

static I18: OnceLock<RwLock<Box<dyn I18Lang + Send + Sync>>> = OnceLock::new();

pub fn set(lang_impl: Box<dyn I18Lang + Send + Sync>) {
    let data = I18.get_or_init(|| RwLock::new(Box::new(EmptyLang)));

    *data.write().unwrap() = lang_impl;
}

pub fn get_info(info: InfoType) -> String {
    let i18 = I18.get().expect("I18 not initialized");
    let lang = i18.read().unwrap();
    lang.get_info(info)
}

pub fn get_error(error: ErrorType) -> String {
    let i18 = I18.get().expect("I18 not initialized");
    let lang = i18.read().unwrap();
    lang.get_error(error)
}
