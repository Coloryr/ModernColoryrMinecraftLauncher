use std::sync::{OnceLock, RwLock};

use crate::i18_items::{
    error_type::ErrorType, gui_type::GuiType, info_type::InfoType, panic_type::PanicType,
    thread_type::ThreadType,
};

pub mod en_us;
pub mod zh_cn;

pub trait I18Lang {
    fn get_info(&self, info: &InfoType) -> String;
    fn get_error(&self, error: &ErrorType) -> String;
    fn get_panic(&self, panic: &PanicType) -> String;
    fn get_thread(&self, thread: &ThreadType) -> String;
    fn get_gui(&self, gui: &GuiType) -> String;
}

static I18: OnceLock<RwLock<Box<dyn I18Lang + Send + Sync>>> = OnceLock::new();

pub fn set(lang_impl: Box<dyn I18Lang + Send + Sync>) {
    let data = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));

    *data.write().unwrap() = lang_impl;
}

pub fn get_info(info: InfoType) -> String {
    with_info(&info)
}

pub fn get_error(error: ErrorType) -> String {
    with_error(&error)
}

pub fn get_panic(panic: PanicType) -> String {
    with_panic(&panic)
}

pub fn get_thread(thread: ThreadType) -> String {
    with_thread(&thread)
}

pub fn get_gui(gui: GuiType) -> String {
    with_gui(&gui)
}

pub fn with_info(info: &InfoType) -> String {
    let i18 = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));
    let lang = i18.read().unwrap();
    lang.get_info(info)
}

pub fn with_error(error: &ErrorType) -> String {
    let i18 = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));
    let lang = i18.read().unwrap();
    lang.get_error(error)
}

pub fn with_panic(panic: &PanicType) -> String {
    let i18 = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));
    let lang = i18.read().unwrap();
    lang.get_panic(panic)
}

pub fn with_thread(thread: &ThreadType) -> String {
    let i18 = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));
    let lang = i18.read().unwrap();
    lang.get_thread(thread)
}

pub fn with_gui(gui: &GuiType) -> String {
    let i18 = I18.get_or_init(|| RwLock::new(Box::new(zh_cn::ZhCn)));
    let lang = i18.read().unwrap();
    lang.get_gui(gui)
}
