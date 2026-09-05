use std::{
    collections::HashMap,
    path::Path,
    str::FromStr,
    sync::{LazyLock, RwLock},
};

use mcml_auth::{AuthType, UserKeyObj, auths};
use mcml_game::launcher_path::assets_path;
use mcml_net::mojang_api;
use skia_safe::{Data, EncodedImageFormat, Image};
use tauri::{
    UriSchemeResponder,
    http::{Request, Response, StatusCode},
};
use uuid::Uuid;

static INSTANCE_IMAGE: LazyLock<RwLock<HashMap<Uuid, Vec<u8>>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));
static SKIN_IMAGE: LazyLock<HashMap<UserKeyObj, Vec<u8>>> = LazyLock::new(|| HashMap::new());
static HEAD_IMAGE: LazyLock<HashMap<UserKeyObj, Vec<u8>>> = LazyLock::new(|| HashMap::new());

fn send_png(res: UriSchemeResponder, data: Vec<u8>) {
    res.respond(
        Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "image/png")
            .body(data)
            .unwrap(),
    );
}

// fn send_jpeg(res: UriSchemeResponder, data: Vec<u8>) {
//     res.respond(
//         Response::builder()
//             .status(StatusCode::OK)
//             .header("Content-Type", "image/jpeg")
//             .body(data)
//             .unwrap(),
//     );
// }

fn send_bad(res: UriSchemeResponder) {
    res.respond(
        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(&[0u8; 0])
            .unwrap(),
    );
}

fn read_as_png<P: AsRef<Path>>(file: P) -> Option<Vec<u8>> {
    let data = Data::from_filename(file)?;
    let image = Image::from_encoded(data)?;
    let data = image.encode(None, EncodedImageFormat::PNG, 100)?;

    Some(data.to_vec())
}

fn load_instance_image(uri: Vec<&str>, res: UriSchemeResponder) {
    if uri.len() != 2 {
        send_bad(res);
        return;
    }
    let uuid = Uuid::from_str(uri[1]);
    if uuid.is_err() {
        send_bad(res);
        return;
    }
    let uuid = uuid.unwrap();
    let image = INSTANCE_IMAGE.read().unwrap().get(&uuid).cloned();
    if let Some(data) = image {
        send_png(res, data.clone());
        return;
    }

    let instance = mcml_game::get_instance(&uuid);
    if instance.is_none() {
        send_bad(res);
        return;
    }

    let instance = instance.unwrap();

    let icon = instance.read().unwrap().get_icon_file();
    if !icon.exists() || !icon.is_file() {
        send_bad(res);
        return;
    }
    let image = read_as_png(icon);
    if image.is_none() {
        send_bad(res);
        return;
    }

    let image = image.unwrap();
    INSTANCE_IMAGE
        .write()
        .unwrap()
        .insert(uuid.clone(), image.clone());
    send_png(res, image);
}

async fn load_skin_image(uri: Vec<&str>, res: UriSchemeResponder) {
    if uri.len() != 3 {
        send_bad(res);
        return;
    }
    let user_type = AuthType::from_str(uri[1]);
    let uuid = uri[2];

    let user = auths::get(uuid, user_type);
    if user.is_none() {
        send_bad(res);
        return;
    }

    let user = user.unwrap();
    if user.auth_type == AuthType::OAuth {
        let data = mojang_api::get_user_profile(&user.uuid, None).await;
        if let Err(err) = data {
            mcml_log::error_type(err);
            send_bad(res);
            return;
        }

        let data =data.unwrap();
        for item in data.properties {
            if item.value
        }
    }
}

pub async fn url_image(req: Request<Vec<u8>>, res: UriSchemeResponder) {
    let uri: Vec<&str> = req.uri().path().split('/').collect();
    let image_type = uri[0];

    if image_type == "instance" {
        load_instance_image(uri, res);
    } else if image_type == "skin" {
        load_skin_image(uri, res).await;
    }
}
