/// 光影包相关
use std::{
    io::Read,
    path::{Path, PathBuf},
    sync::Mutex,
};

use mcml_base::path_helper;
use mcml_names::{
    i18_items::error_type::{CoreResult, ErrorType, FileSystemErrorData},
    names,
};
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use zip::ZipArchive;

use crate::{game_options, launcher::instance_setting_obj::InstanceSettingObj};

/// 光影包
pub struct ShaderpackObj {
    /// 名字
    pub name: String,
    /// 说明
    pub comment: String,
    /// 文件位置
    pub file: PathBuf,
}

impl Default for ShaderpackObj {
    fn default() -> Self {
        Self {
            name: Default::default(),
            comment: Default::default(),
            file: Default::default(),
        }
    }
}

impl ShaderpackObj {
    /// 删除
    pub fn delete(&self) -> CoreResult<()> {
        path_helper::move_to_trash(&self.file)
    }
}

fn read_data<R: Read>(stream: R, obj: &mut ShaderpackObj) {
    // 从语言文件中读取名字，可能会有更多
    let options = game_options::read_options(stream, Some('='));
    if let Ok(data) = options {
        if let Some(about) = data.get("option.ABOUT") {
            obj.name = about.clone();
        }
        if let Some(about) = data.get("screen.ABOUT") {
            obj.name = about.clone();
        }
        if let Some(about) = data.get("option.ACERCADE") {
            obj.name = about.clone();
        }

        if let Some(about) = data.get("option.ABOUT.comment") {
            obj.comment = about.clone();
        }
        if let Some(about) = data.get("option.ACERCADE.comment") {
            obj.comment = about.clone();
        }
    }
}

pub fn read_shaderpacks<P: AsRef<Path>>(path: P) -> CoreResult<ShaderpackObj> {
    if let Some(ext) = path.as_ref().extension() {
        if ext.eq_ignore_ascii_case(names::ZIP_EXT) {
            let mut obj = ShaderpackObj {
                file: path.as_ref().to_path_buf(),
                ..Default::default()
            };

            let stream = path_helper::open_read(path.as_ref())?;
            let mut zip = ZipArchive::new(&stream).map_err(|err| {
                ErrorType::ArchiveOpenError(FileSystemErrorData {
                    path: path.as_ref().to_path_buf(),
                    error: err.to_string(),
                })
            })?;

            for index in 0..zip.len() {
                let temp = zip.by_index(index);
                if let Ok(file) = temp {
                    let lang = mcml_names::get_lang(mcml_names::get_lang_type());
                    if file.is_file() && file.name().ends_with(&format!("lang/{lang}.lang")) {
                        read_data(file, &mut obj);
                    } else if file.is_file() && file.name().ends_with("lang/en_US.lang") {
                        read_data(file, &mut obj);
                    }
                }
            }

            if obj.name.is_empty() {
                obj.name = path
                    .as_ref()
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
            }

            return Ok(obj);
        }
    }

    Err(ErrorType::InfoNotFound("Zip".to_string()))
}

impl InstanceSettingObj {
    /// 获取光影包列表
    pub async fn get_shaderpacks(&self) -> Vec<ShaderpackObj> {
        let path = self.get_shaderpacks_path();
        let files = path_helper::get_files(path);

        tokio::task::spawn_blocking(move || {
            let list = Mutex::new(Vec::new());

            files
                .par_iter()
                .for_each(|item| match read_shaderpacks(item) {
                    Ok(obj) => {
                        list.lock().unwrap().push(obj);
                    }
                    Err(err) => {
                        mcml_log::error_type(err);
                    }
                });

            list.into_inner().unwrap()
        })
        .await
        .unwrap_or_default()
    }
}
