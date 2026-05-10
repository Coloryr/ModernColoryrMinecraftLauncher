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
            ErrorType::ConfigSaveError(data, data1) => {
                format!("配置文件保存失败：{} 位置：{}", data, data1)
            }
            ErrorType::ConfigReadError(data, data1) => {
                format!("配置文件读取失败：{} 位置：{}", data, data1)
            }
            ErrorType::ColoryrApiGetError(data) => format!("ColoryrApi请求错误：{}", data),
            ErrorType::ColoryrApiServerError(data) => format!("ColoryrApi返回错误：{}", data),
            ErrorType::HttpReqError(data) => format!("网络发送请求错误：{}", data),
            ErrorType::JsonDecError(data) => format!("Json解析失败：{}", data),
            ErrorType::FileNotExists(data) => format!("文件不存在：{}", data),
            ErrorType::HttpReadError(data) => format!("网络请求错误：{}", data),
            ErrorType::AuthLoginFail(data) => format!("账户登录失败：{}", data),
            ErrorType::AuthLoginNoProfile => String::from("账户登录错误，没有找到账户"),
            ErrorType::AuthRefreshFail(data) => format!("账户刷新失败：{}", data),
            ErrorType::AuthRefreshNoProfile => String::from("账户刷新错误，没有找到账户"),
            _ => String::new()
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
