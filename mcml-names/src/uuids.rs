use std::sync::LazyLock;

use uuid::{Uuid, uuid};

pub const CONFIG_UUID: Uuid = uuid!("1c857169-72ac-48c6-b954-4030b4dc6f94");
pub const AUTH_UUID: Uuid = uuid!("341758b5-321c-4c72-9975-bebc8fef44fe");
pub const OPTIFINE_UUID: Uuid = uuid!("f9bd5b73-4bc5-4355-88ca-81f8dc3d5a16");
pub const LITELOADER_UUID: Uuid = uuid!("501863ab-af68-4134-a279-f96f002384e1");

pub const BACKUP_UUID: Uuid = uuid!("af877a38-c6d8-4c9c-a8b5-201567ceb2b5");
pub const ONLINE_FILE_UUID: Uuid = uuid!("a2132056-e180-47e7-ba0d-4331160c10e7");
pub const LAUNCH_COUNT_DATA_FILE_UUID: Uuid = uuid!("29f03f82-157c-4fb5-b0a9-ca78d86ceb84");

static UUIDS: LazyLock<Vec<Uuid>> = LazyLock::new(|| {
    vec![
        CONFIG_UUID,
        AUTH_UUID,
        OPTIFINE_UUID,
        LITELOADER_UUID,
        BACKUP_UUID,
        ONLINE_FILE_UUID,
        LAUNCH_COUNT_DATA_FILE_UUID,
    ]
});

pub fn check_uuid(uuid: Uuid) -> bool {
    for item in UUIDS.iter() {
        if uuid.eq(item) {
            return true;
        }
    }

    false
}

pub fn mix_uuid(uuid1: Uuid, uuid2: Uuid) -> Uuid {
    let data1 = uuid1.as_bytes();
    let data2 = uuid2.as_bytes();

    let mut data = [0u8; 16];

    for i in 0..16 {
        data[i] = data1[i] + data2[i];
    }

    Uuid::from_bytes(data)
}
