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
