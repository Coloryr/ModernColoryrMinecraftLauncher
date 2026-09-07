use std::sync::LazyLock;

use uuid::{Uuid, uuid};

pub const CONFIG_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000001");
pub const AUTH_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000002");
pub const OPTIFINE_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000003");
pub const LITELOADER_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000004");
pub const BACKUP_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000005");
pub const ONLINE_FILE_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000006");
pub const LAUNCH_COUNT_DATA_FILE_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000007");
pub const SERVERPACK_FILE_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000008");
pub const WINDOW_FILE_UUID: Uuid = uuid!("00000000-0000-0000-0000-000000000009");
pub const GUI_CONFIG_FILE_UUID: Uuid = uuid!("00000000-0000-0000-0000-00000000000a");
pub const AUTH_SELECT_UUID: Uuid = uuid!("00000000-0000-0000-0000-00000000000b");

static UUIDS: LazyLock<Vec<Uuid>> = LazyLock::new(|| {
    vec![
        CONFIG_UUID,
        AUTH_UUID,
        OPTIFINE_UUID,
        LITELOADER_UUID,
        BACKUP_UUID,
        ONLINE_FILE_UUID,
        LAUNCH_COUNT_DATA_FILE_UUID,
        SERVERPACK_FILE_UUID,
        WINDOW_FILE_UUID,
        GUI_CONFIG_FILE_UUID,
        AUTH_SELECT_UUID
    ]
});

/// 检查是否是配置文件的UUID
/// 
/// - `uuid`: 需要检查的uuid
pub fn check_uuid(uuid: Uuid) -> bool {
    for item in UUIDS.iter() {
        if uuid.eq(item) {
            return true;
        }
    }

    false
}

/// 混合UUID
pub fn mix_uuid(uuid1: Uuid, uuid2: Uuid) -> Uuid {
    Uuid::new_v5(&uuid1, uuid2.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 所有内置配置 UUID 都应被 check_uuid 识别
    #[test]
    fn check_builtin_uuids() {
        for uuid in UUIDS.iter() {
            assert!(check_uuid(*uuid), "内置 UUID {uuid} 应被识别");
        }
    }

    /// 随机 UUID 不应被识别为配置 UUID
    #[test]
    fn check_random_uuid() {
        assert!(!check_uuid(Uuid::new_v4()));
        assert!(!check_uuid(Uuid::nil()));
    }

    /// 内置 UUID 互不重复
    #[test]
    fn builtin_uuids_unique() {
        let mut list = UUIDS.clone();
        list.sort();
        let before = list.len();
        list.dedup();
        assert_eq!(list.len(), before, "内置 UUID 不应重复");
        assert_eq!(before, 11);
    }

    /// mix_uuid 基于 UUIDv5，同参数结果确定，不同参数结果不同
    #[test]
    fn mix_uuid_deterministic() {
        let a = mix_uuid(CONFIG_UUID, AUTH_UUID);
        let b = mix_uuid(CONFIG_UUID, AUTH_UUID);
        assert_eq!(a, b, "同参数混合结果应一致");
        assert_ne!(a, mix_uuid(AUTH_UUID, CONFIG_UUID), "参数顺序影响结果");
        assert_ne!(a, CONFIG_UUID);
        assert_ne!(a, AUTH_UUID);
        assert_eq!(a.get_version_num(), 5, "混合结果应是 UUIDv5");
    }
}
