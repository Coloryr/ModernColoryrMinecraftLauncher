/// 游戏实例屏幕截图相关

use std::path::PathBuf;

use mcml_base::path_helper;
use mcml_names::{i18_items::error_type::CoreResult, names};

use crate::launcher::instance_setting_obj::InstanceSettingObj;

/// 屏幕截图文件
pub struct ScreenshotObj {
    /// 文件名
    pub name: String,
    /// 路径
    pub file: PathBuf,
}

impl ScreenshotObj {
    /// 删除截图
    pub fn delete(&self) -> CoreResult<()> {
        path_helper::move_to_trash(&self.file)
    }
}

impl InstanceSettingObj {
    /// 获取实例的所有截图
    pub fn get_screenshots(&self) -> Vec<ScreenshotObj> {
        let dir = self.get_screenshots_path();
        let mut list = Vec::new();
        for item in path_helper::get_files(dir).iter() {
            if let Some(ext) = item.extension()
                && let Some(name) = item.file_name()
            {
                if ext.eq_ignore_ascii_case(names::PNG_EXT) {
                    list.push(ScreenshotObj {
                        name: name.to_string_lossy().to_string(),
                        file: item.clone(),
                    });
                }
            }
        }

        list
    }

    /// 删除所有截图
    pub fn clear_screenshots(&self) -> CoreResult<()> {
        let dir = self.get_screenshots_path();
        for item in path_helper::get_files(dir).iter() {
            path_helper::move_to_trash(item)?;
        }

        Ok(())
    }
}
