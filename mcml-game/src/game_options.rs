use std::{collections::HashMap, io::Read};

use crate::launcher::instance_setting_obj::InstanceSettingObj;

pub fn read_options<R: Read>(buffer: R, sp: Option<char>) -> HashMap<String, String> {

}

impl InstanceSettingObj {
    pub fn get_options(&self) -> HashMap<String, String> {
        let file = self.get_optifine_file();
        if file.exists() {

        } else {
            Default::default()
        }
    }
}