/// 游戏实例配置相关

use std::{
    collections::HashMap,
    io::{BufRead, BufReader, Read, Write},
};

use mcml_base::path_helper;
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};

use crate::launcher::instance_setting_obj::InstanceSettingObj;

pub fn read_options<R: Read>(buffer: R, sp: Option<char>) -> CoreResult<HashMap<String, String>> {
    let mut reader = BufReader::new(buffer);
    let mut data = HashMap::new();
    let mut line = String::new();
    let sp = sp.unwrap_or(':');
    loop {
        let size = reader.read_line(&mut line).map_err(|err| {
            ErrorType::StreamError(ErrorData {
                error: err.to_string(),
            })
        })?;
        if size == 0 {
            break;
        }

        // 去除 # 注释
        let line_trimmed = line.trim();
        if line_trimmed.starts_with('#') || line_trimmed.is_empty() {
            line.clear();
            continue;
        }
        // 处理行内注释
        let line_content = if let Some(pos) = line_trimmed.find('#') {
            line_trimmed[..pos].trim()
        } else {
            line_trimmed
        };
        if line_content.is_empty() {
            line.clear();
            continue;
        }

        let lists: Vec<&str> = line_content.splitn(2, sp).collect();
        if lists.len() == 1 {
            data.insert(lists[0].to_string(), Default::default());
        } else if lists.len() == 2 {
            data.insert(lists[0].to_string(), lists[1].to_string());
        }
        line.clear();
    }

    Ok(data)
}

impl InstanceSettingObj {
    /// 读取配置文件
    pub fn get_options(&self) -> CoreResult<HashMap<String, String>> {
        let file = self.get_optifine_file();
        if file.exists() {
            let stream = path_helper::open_read(file)?;
            read_options(stream, None)
        } else {
            Ok(Default::default())
        }
    }

    /// 保存配置文件
    /// - `list`: 配置选项
    pub fn save_options(&self, list: &HashMap<String, String>, sp: Option<char>) -> CoreResult<()> {
        let file = self.get_optifine_file();
        let mut stream = path_helper::open_write(file)?;
        let sp = sp.unwrap_or(':');

        for (key, value) in list.iter() {
            stream
                .write_fmt(format_args!(
                    "{}{sp}{}{}",
                    key,
                    value,
                    mcml_names::get_line_ending()
                ))
                .map_err(|err| {
                    ErrorType::StreamError(ErrorData {
                        error: err.to_string(),
                    })
                })?;
        }

        Ok(())
    }
}
