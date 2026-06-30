use core::fmt;

use crate::{
    i18,
    i18_items::{
        error_type::ErrorType, info_type::InfoType, panic_type::PanicType, thread_type::ThreadType,
    },
};

pub mod error_type;
pub mod info_type;
pub mod panic_type;
pub mod thread_type;

impl fmt::Display for InfoType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", i18::with_info(self))
    }
}

impl fmt::Display for ErrorType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", i18::with_error(self))
    }
}

impl fmt::Display for PanicType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", i18::with_panic(self))
    }
}

impl fmt::Display for ThreadType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", i18::with_thread(self))
    }
}
