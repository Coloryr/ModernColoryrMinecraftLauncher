use mcml_base::checker::check_is_not_number;

use crate::launcher::SourceType;

/// 检测下载源
/// - `pid`: 项目号
/// - `fid`: 文件号
pub fn get_source_type(pid: &str, fid: &str) -> SourceType {
    if check_is_not_number(pid) || check_is_not_number(fid) {
        SourceType::Modrinth
    } else {
        SourceType::CurseForge
    }
}
