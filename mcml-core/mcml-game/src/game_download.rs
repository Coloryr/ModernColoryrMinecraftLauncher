use mcml_base::tools::check_is_not_number;

use crate::launcher::ModPackType;

/// 检测下载源
/// - `pid`: 项目号
/// - `fid`: 文件号
pub fn get_source_type(pid: &str, fid: &str) -> ModPackType {
    if check_is_not_number(pid) || check_is_not_number(fid) {
        ModPackType::Modrinth
    } else {
        ModPackType::CurseForge
    }
}
