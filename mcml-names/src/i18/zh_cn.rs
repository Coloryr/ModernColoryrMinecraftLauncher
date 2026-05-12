use crate::VERSION;

use crate::{
    i18::I18Lang,
    i18_items::{
        error_type::ErrorType, info_type::InfoType, panic_type::PanicType, thread_type::ThreadType,
    },
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
            ErrorType::ConfigSaveError(data) => {
                format!("配置文件 {} 保存失败：{}", data.file, data.error)
            }
            ErrorType::ConfigReadError(data) => {
                format!("配置文件 {} 读取失败：{}", data.file, data.error)
            }
            ErrorType::HttpReqError(data) => {
                format!("发送网络请求 {} 错误：{}", data.url, data.error)
            }
            ErrorType::HttpReadError(data) => {
                format!("发送网络请求 {} 读取失败：{}", data.url, data.error)
            }

            ErrorType::JsonError(data) => format!("Json解析失败：{}", data.error),
            ErrorType::FileNotExists(data) => format!("文件不存在：{}", data.file),

            ErrorType::AuthLoginFail(data) => format!("账户登录失败：{}", data),
            ErrorType::AuthLoginNoProfile => String::from("账户登录错误，没有找到账户"),
            ErrorType::AuthRefreshFail(data) => format!("账户刷新失败：{}", data),
            ErrorType::AuthRefreshNoProfile => String::from("账户刷新错误，没有找到账户"),
            _ => String::new(),
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
