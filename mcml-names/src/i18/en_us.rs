use crate::{error_type::ErrorType, i18::I18Lang, info_type::InfoType};

pub struct EnUs;

impl I18Lang for EnUs {
    fn get_info(&self, info: InfoType) -> String {
        match info {
            InfoType::CoreStart => todo!(),
        }
    }

    fn get_error(&self, error: ErrorType) -> String {
        match error {
            ErrorType::AdoptiumGetError(_) => todo!(),
        }
    }
}
