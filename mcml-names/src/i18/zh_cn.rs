use crate::{
    VERSION, error_type::ErrorType, i18::I18Lang, info_type::InfoType, panic_type::PanicType,
    thread_type::ThreadType,
};

pub struct ZhCn;

impl I18Lang for ZhCn {
    fn get_info(&self, info: InfoType) -> String {
        match info {
            InfoType::CoreStart => format!("MCML启动，版本：{}", VERSION),
            InfoType::CoreStop => format!("MCML停止"),
        }
    }

    fn get_error(&self, error: ErrorType) -> String {
        match error {
            ErrorType::AdoptiumGetError(_) => todo!(),
        }
    }

    fn get_panic(&self, panic: PanicType) -> String {
        match panic {
            PanicType::CoreArgLocalEmpty => String::from("运行路径为空"),
            PanicType::CoreArgLocalError(data) => format!("运行路径无法创建：{}", data),
            PanicType::LogOpenFail(data, data1) => {
                format!("日志系统初始化失败：{} 路径：{}", data1, data)
            }
        }
    }

    fn get_thread(&self, thread: ThreadType) -> String {
        match thread {
            ThreadType::LogThread => String::from("日志线程"),
            ThreadType::ConfigSaveThread => String::from("配置保存线程"),
        }
    }
}
