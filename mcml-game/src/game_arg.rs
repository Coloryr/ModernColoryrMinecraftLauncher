use mcml_config::config_obj::WindowSettingObj;

use crate::{
    game_saves::SaveObj,
    launcher::{
        LoaderType,
        game_setting_obj::{GameSettingObj, ServerObj},
    },
    launcher_path::libraies_path,
    mojang::{
        check_allow,
        game_arg_obj::{ArgValue, Argument, GameArgObj},
        version_checker::is_game_version_120,
    },
};

const V1_JVM_ARG: [&str; 3] = [
    "-Djava.library.path=${natives_directory}",
    "-cp",
    "${classpath}",
];

/// 创建V1游戏启动参数
/// - `game`: 游戏启动参数
fn make_v1_game_arg(game: &GameArgObj) -> Vec<String> {
    match &game.minecraft_arguments {
        None => Vec::new(),
        Some(arg) => arg.split(' ').map(|s| s.to_string()).collect(),
    }
}

/// 创建V2游戏启动参数
/// - `game`: 游戏启动参数
fn make_v2_game_arg(game: &GameArgObj) -> Vec<String> {
    match &game.arguments {
        None => Vec::new(),
        Some(args) => {
            let mut vec: Vec<String> = Vec::new();

            for item in args.game.iter() {
                if let Argument::Plain(item) = item {
                    vec.push(item.to_string());
                }
            }

            vec
        }
    }
}

/// 创建V1加载器启动参数
/// - `obj`: 游戏启动参数
/// - `game`: 游戏启动参数
fn make_loader_v1_game_arg(obj: &GameSettingObj, game: &GameArgObj) -> Vec<String> {
    match obj.loader {
        LoaderType::Forge | LoaderType::NeoForge => {
            let loader = if obj.loader == LoaderType::Forge {
                obj.get_forge()
            } else {
                obj.get_neoforge()
            };

            if let Some(data) = loader
                && let Some(data1) = &data.minecraft_arguments
            {
                let args: Vec<&str> = data1.split(' ').collect();
                args.iter().map(|item| String::from(*item)).collect()
            } else {
                Default::default()
            }
        }
        LoaderType::Normal => Default::default(),
        LoaderType::Fabric => {
            let loader = obj.get_fabric();
            if let Some(data) = loader {
                let mut args = make_v1_game_arg(game);
                for item in data.arguments.game.iter() {
                    args.push(item.clone());
                }

                args
            } else {
                Default::default()
            }
        }
        LoaderType::Quilt => {
            let loader = obj.get_quilt();
            if let Some(data) = loader {
                let mut args = make_v1_game_arg(game);
                for item in data.arguments.game.iter() {
                    args.push(item.clone());
                }

                args
            } else {
                Default::default()
            }
        }
        LoaderType::OptiFine => {
            let mut args = make_v1_game_arg(game);
            args.push(String::from("--tweakClass"));
            args.push(String::from("optifine.OptiFineTweaker"));
            args
        }
        LoaderType::Custom => obj.get_custom_loader_game_args(),
    }
}

/// 创建V2加载器启动参数
/// - `obj`: 游戏启动参数
/// - `game`: 游戏启动参数
fn make_loader_v2_game_arg(obj: &GameSettingObj) -> Vec<String> {
    match obj.loader {
        LoaderType::Forge | LoaderType::NeoForge => {
            let loader = if obj.loader == LoaderType::Forge {
                obj.get_forge()
            } else {
                obj.get_neoforge()
            };

            if let Some(data) = loader
                && let Some(data1) = &data.arguments
            {
                data1.game.clone()
            } else {
                Default::default()
            }
        }
        LoaderType::Normal => Default::default(),
        LoaderType::Fabric => {
            let loader = obj.get_fabric();
            if let Some(data) = loader {
                data.arguments.game.clone()
            } else {
                Default::default()
            }
        }
        LoaderType::Quilt => {
            let loader = obj.get_quilt();
            if let Some(data) = loader {
                data.arguments.game.clone()
            } else {
                Default::default()
            }
        }
        LoaderType::OptiFine => {
            vec![
                String::from("--tweakClass"),
                String::from("optifine.OptiFineTweaker"),
            ]
        }
        LoaderType::Custom => obj.get_custom_loader_game_args(),
    }
}

/// 创建V1游戏Jvm参数
fn make_v2_jvm_arg(game: &GameArgObj) -> Vec<String> {
    if let Some(data) = &game.arguments {
        let mut args = Vec::<String>::new();

        for item in data.jvm.iter() {
            match item {
                Argument::Plain(str) => args.push(str.clone()),
                Argument::Conditional(obj) => {
                    if !check_allow(&obj.rules) {
                        continue;
                    }

                    match &obj.value {
                        ArgValue::Single(str) => args.push(str.clone()),
                        ArgValue::Multi(items) => {
                            for item1 in items {
                                args.push(item1.clone())
                            }
                        }
                    }
                }
            }
        }

        args
    } else {
        Default::default()
    }
}

/// 创建加载器Jvm参数
/// - `v2`: 是否为V2版本
/// - `obj`: 游戏实例
pub fn make_loader_jvm_arg(v2: bool, obj: &GameSettingObj) -> Vec<String> {
    match obj.loader {
        LoaderType::Normal => Default::default(),
        LoaderType::Forge | LoaderType::NeoForge => {
            if v2 {
                let mut list = Vec::<String>::new();
                list.push(format!(
                    "-Dforgewrapper.librariesDir={}",
                    libraies_path::get_base_dir().display()
                ));
                let file = if obj.loader == LoaderType::NeoForge {
                    obj.build_neoforge_installer()
                } else {
                    obj.build_forge_installer()
                };
                list.push(format!("-Dforgewrapper.installer={}", file.file.display()));
                list.push(format!(
                    "-Dforgewrapper.minecraft={}",
                    libraies_path::get_game_file(&obj.version).display()
                ));

                let obj = if obj.loader == LoaderType::NeoForge {
                    obj.get_neoforge().unwrap()
                } else {
                    obj.get_forge().unwrap()
                };

                if let Some(args) = &obj.arguments {
                    list.extend(args.jvm.clone());
                }

                list
            } else {
                Default::default()
            }
        }
        LoaderType::Fabric => {
            let fabric = obj.get_fabric().unwrap();

            fabric.arguments.jvm.clone()
        }
        LoaderType::Quilt => Default::default(),
        LoaderType::OptiFine => {
            let mut list = vec![
                format!(
                    "-Dlibdir={}",
                    libraies_path::get_base_dir().to_string_lossy()
                ),
                format!("-Dgamecore={}", obj.get_game_file().to_string_lossy()),
                format!("-Doptifine={}", obj.get_optifine_file().to_string_lossy()),
            ];

            if v2 {
                list.push(String::from("--add-opens"));
                list.push(String::from("java.base/java.lang=ALL-UNNAMED"));
                list.push(String::from("--add-opens"));
                list.push(String::from("java.base/java.util=ALL-UNNAMED"));
                list.push(String::from("--add-opens"));
                list.push(String::from("java.base/java.net=ALL-UNNAMED"));
                list.push(String::from("--add-opens"));
                list.push(String::from("java.base/jdk.internal.loader=ALL-UNNAMED"));
            }

            list
        }
        LoaderType::Custom => obj.get_custom_loader_game_args(),
    }
}

impl GameSettingObj {
    pub fn make_game_arg(
        &self,
        world: Option<&SaveObj>,
        server: Option<&ServerObj>,
    ) -> Vec<String> {
        let mut args = Vec::new();

        let config = mcml_config::CONFIG
            .get()
            .unwrap()
            .read()
            .unwrap()
            .window
            .clone();

        let mut full_screen = false;
        let mut width = 0;
        let mut height = 0;

        match &self.window {
            Some(window) => {
                if let Some(data) = window.full_screen {
                    full_screen = data;
                }
                if let Some(data) = window.width {
                    width = data;
                }
                if let Some(data) = window.height {
                    height = data;
                }
            }
            None => {
                if let Some(data) = config.full_screen {
                    full_screen = data;
                }
                if let Some(data) = config.width {
                    width = data;
                }
                if let Some(data) = config.height {
                    height = data;
                }
            }
        }

        if full_screen {
            args.push(String::from("--fullscreen"));
        } else {
            if width > 0 {
                args.push(String::from("--width"));
                args.push(format!("{width}"));
            }
            if height > 0 {
                args.push(String::from("--height"));
                args.push(format!("{height}"));
            }
        }

        match world {
            Some(world) => {
                args.push(String::from("--quickPlaySingleplayer"));
                args.push(world.level_name.clone());
            }
            None => {
                let server = match &self.start_server {
                    Some(server) => Some(server),
                    None => server,
                };

                if let Some(server) = server
                    && let Some(ip) = &server.ip
                    && !ip.is_empty()
                {
                    let port = match server.port {
                        Some(port) => port,
                        None => 25565,
                    };
                    if self.is_game_version_120() {
                        args.push(String::from("--quickPlayMultiplayer"));
                        args.push(format!("{ip}:{port}"));
                    } else {
                        args.push(String::from("--server"));
                        args.push(ip.clone());
                        args.push(String::from("--port"));
                        args.push(format!("{port}"));
                    }
                }
            }
        }

        if let Some(proxy) = &self.proxy_host {
            if let Some(ip) = &proxy.ip {
                args.push(String::from("--proxyHost"));
                args.push(ip.clone());
            }
        }

        args
    }
}
