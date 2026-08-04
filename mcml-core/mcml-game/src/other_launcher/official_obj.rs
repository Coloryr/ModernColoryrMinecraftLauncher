use std::path::Path;

use mcml_base::{path_helper, serialize_tools::MiniJsonObj};
use mcml_names::i18_items::error_type::{ArgEmptyData, CoreResult, ErrorType};

/// 官方实例信息
pub struct OfficialObj {
    pub id: String,
    pub inherits_from: String,
    pub patches: Vec<PatchObj>,
    pub libraries: Vec<LibrarieObj>,
    pub arguments: ArgumentsObj,
}

impl Default for OfficialObj {
    fn default() -> Self {
        Self {
            id: Default::default(),
            inherits_from: Default::default(),
            patches: Default::default(),
            libraries: Default::default(),
            arguments: Default::default(),
        }
    }
}

pub struct PatchObj {
    pub id: String,
    pub version: String,
}

pub struct LibrarieObj {
    pub name: String,
}

pub struct ArgumentsObj {
    pub game: Vec<MiniJsonObj>,
}

impl Default for ArgumentsObj {
    fn default() -> Self {
        Self {
            game: Default::default(),
        }
    }
}

impl OfficialObj {
    /// 从文件读取信息
    ///
    /// - `file`: 文件位置
    pub fn read_from_file<P: AsRef<Path>>(file: P) -> CoreResult<Self> {
        let stream = path_helper::open_read(file)?;
        let json = MiniJsonObj::from_stream(stream)?;

        if let Some(data) = json.as_object() {
            let mut obj = OfficialObj {
                id: data.get_string("id"),
                inherits_from: data.get_string("inheritsFrom"),
                ..Default::default()
            };

            if let Some(list) = data.get_list("patches") {
                for item in list {
                    if let Some(list) = item.as_object() {
                        obj.patches.push(PatchObj {
                            id: list.get_string("id"),
                            version: list.get_string("version"),
                        });
                    }
                }
            }

            if let Some(list) = data.get_list("libraries") {
                for item in list {
                    if let Some(list) = item.as_object() {
                        obj.libraries.push(LibrarieObj {
                            name: list.get_string("name"),
                        });
                    }
                }
            }

            if let Some(list) = data.get_list("arguments") {
                obj.arguments.game.extend(list);
            }

            Ok(obj)
        } else {
            Err(ErrorType::ArgEmpty(ArgEmptyData {
                arg: "id".to_string(),
            }))
        }
    }
}
