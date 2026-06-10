use crate::{
    launcher::{LoaderType, game_setting_obj::GameSettingObj},
    mojang::game_arg_obj::{Argument, GameArgObj},
};

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

            if let Some(data) = loader {
                let args: Vec<&str> = data.minecraft_arguments.split(' ').collect();
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

            if let Some(data) = loader {
                data.arguments.game.clone()
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
    match &game.arguments {
        None => Default::default(),
        Some(data) => {
            let mut args = Vec::<String>::new();

            for item in data.jvm.iter() {
                match item {
                    Argument::Plain(str) => args.push(str.clone()),
                    Argument::Conditional(obj) => {
                        
                    },
                }
            }

            args
        }
    }
}
