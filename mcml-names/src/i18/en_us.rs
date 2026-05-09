use crate::{i18::I18Lang, i18_items::{error_type::ErrorType, info_type::InfoType, panic_type::PanicType, thread_type::ThreadType}};
pub struct EnUs;

impl I18Lang for EnUs {
    fn get_info(&self, info: InfoType) -> String {
        match info {
            InfoType::CoreStart => todo!(),
            _ => todo!(),
        }
    }

    fn get_error(&self, error: ErrorType) -> String {
        match error {
            _ => todo!(),
        }
    }

    fn get_panic(&self, panic: PanicType) -> String {
        match panic {
            _ => todo!(),
        }
    }
    
    fn get_thread(&self, thread: ThreadType) -> String {
        todo!()
    }
}
