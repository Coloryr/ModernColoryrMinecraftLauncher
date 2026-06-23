use std::{collections::HashMap, path::PathBuf, sync::Arc};

use mcml_auth::{
    AuthType, LoginObj,
    legacy::{authlib_injector, little_skin},
};
use mcml_base::{Os, hash_helper};
use mcml_config::config_obj::GCType;
use mcml_names::{i18_items::error_type::CoreResult, names};
use mcml_net::urls;

use crate::{
    game_launch::GameLaunchObj,
    game_saves::SaveObj,
    launcher::game_setting_obj::{GameSettingObj, ServerObj},
    launcher_path::{self, assets_path, libraries_path},
    loader::LoaderType,
    mojang::{
        self,
        game_arg_obj::{ArgValue, Argument, GameArgObj},
    },
};

/// 启动后自动操作
pub enum AutoJoinType {
    None,
    /// 进入存档
    Save(Arc<SaveObj>),
    /// 进入服务器
    Server(Arc<ServerObj>),
}

/// 游戏启动所使用的参数
pub struct GameLaunchArg {
    /// 登录账户
    pub auth: Arc<LoginObj>,
    /// 自动进入的操作
    pub auto: AutoJoinType,
    /// 是否以管理员方式启动
    pub admin: bool,
    // pub gui: ILaunchGui,
    /// ASM端口
    pub mixin: Option<u16>,
}

/// V1启动Jvm参数
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
                    if !mojang::check_allow(&obj.rules) {
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
                    libraries_path::get_lib_dir().display()
                ));
                let file = if obj.loader == LoaderType::NeoForge {
                    obj.build_neoforge_installer()
                } else {
                    obj.build_forge_installer()
                };
                list.push(format!("-Dforgewrapper.installer={}", file.file.display()));
                list.push(format!(
                    "-Dforgewrapper.minecraft={}",
                    libraries_path::get_game_file(&obj.version).display()
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
                    libraries_path::get_lib_dir().to_string_lossy()
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

        let config = mcml_config::read_config().window.clone();

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
                args.push(width.to_string());
            }
            if height > 0 {
                args.push(String::from("--height"));
                args.push(height.to_string());
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
                        args.push(port.to_string());
                    }
                }
            }
        }

        if let Some(proxy) = &self.proxy_host {
            if let Some(ip) = &proxy.ip {
                args.push(String::from("--proxyHost"));
                args.push(ip.clone());
            }
            if let Some(port) = &proxy.port {
                args.push(String::from("--proxyPort"));
                args.push(port.to_string());
            }
            if let Some(user) = &proxy.user {
                args.push(String::from("--proxyUser"));
                args.push(user.clone());
            }
            if let Some(password) = &proxy.password {
                args.push(String::from("--proxyPass"));
                args.push(password.clone());
            }
        }

        if let Some(arg) = &self.jvm_arg
            && let Some(arg) = &arg.game_args
        {
            let arg: Vec<&str> = arg.split('\n').collect();
            args.extend(arg.iter().map(|item| String::from(*item)));
        }

        args
    }

    /// 创建Jvm参数
    /// - `login`: 登陆使用的账户
    /// - `java`: 使用的JAVA主版本号
    /// - `mixin`: 外部注入使用的端口号
    async fn make_jvm_arg(
        &self,
        auth: &LoginObj,
        java: u8,
        mixin: Option<u16>,
    ) -> CoreResult<Vec<String>> {
        let mut jvm_args = String::new();
        let mut gc = GCType::Auto;
        let mut max = 0u32;
        let mut min = 0u32;
        let mut colorasm = false;

        match &self.jvm_arg {
            Some(arg) => {
                if let Some(data) = &arg.jvm_args {
                    jvm_args = data.clone();
                }

                if let Some(data) = arg.gc_mode {
                    gc = data;
                }

                if let Some(data) = arg.max_memory {
                    max = data;
                }

                if let Some(data) = arg.min_memory {
                    min = data;
                }

                if let Some(data) = arg.colorasm {
                    colorasm = data;
                }
            }
            None => {
                let config = &mcml_config::read_config().jvm_arg;

                if let Some(data) = &config.jvm_args {
                    jvm_args = data.clone();
                }

                if let Some(data) = config.gc_mode {
                    gc = data;
                }

                if let Some(data) = config.max_memory {
                    max = data;
                }

                if let Some(data) = config.min_memory {
                    min = data;
                }

                if let Some(data) = config.colorasm {
                    colorasm = data;
                }
            }
        }

        let mut args = Vec::new();

        if colorasm && let Some(port) = mixin {
            args.push(format!("-Dcolormc.mixin.port={}", port));
            args.push(format!("-Dcolormc.mixin.uuid={}", self.uuid));
            args.push(format!(
                "-javaagent:{}",
                launcher_path::ready_colorasm()?.to_string_lossy()
            ));
        }

        if gc == GCType::Auto {
            if java >= 21 {
                gc = GCType::ZGC;
            } else {
                gc = GCType::G1GC;
            }
        }

        if gc == GCType::ZGC {
            args.extend(names::GCZGC.map(|item| String::from(item)));
        } else if gc == GCType::G1GC {
            args.extend(names::G1GC.map(|item| String::from(item)));
        }

        if min > 0 {
            args.push(format!("-Xms{min}m"));
        }

        if max > 0 {
            args.push(format!("-Xms{max}m"));
        }

        if !jvm_args.is_empty() {
            args.extend(jvm_args.split('\n').map(|item| String::from(item.trim())));
        }

        match &auth.auth_type {
            AuthType::Nide8 => {
                args.push(format!(
                    "-javaagent:{}={}",
                    libraries_path::get_nide8_file().unwrap().to_string_lossy(),
                    auth.text1.as_ref().unwrap()
                ));
            }
            AuthType::AuthlibInjector => {
                let key = authlib_injector::get_key(&auth).await?;
                args.push(format!(
                    "-javaagent:{}={}",
                    libraries_path::get_authlib_file()
                        .unwrap()
                        .to_string_lossy(),
                    auth.text1.as_ref().unwrap(),
                ));
                args.push(format!(
                    "-Dauthlibinjector.yggdrasil.prefetched={}",
                    hash_helper::gen_base64(&key)
                ));
                args.push(String::from("-Dauthlibinjector.side=client"));
            }
            AuthType::LittleSkin => {
                let key = little_skin::get_key(&auth).await?;
                args.push(format!(
                    "-javaagent:{}={}api/yggdrasil",
                    libraries_path::get_authlib_file()
                        .unwrap()
                        .to_string_lossy(),
                    urls::LITTLE_SKIN_URL,
                ));
                args.push(format!(
                    "-Dauthlibinjector.yggdrasil.prefetched={}",
                    hash_helper::gen_base64(&key)
                ));
                args.push(String::from("-Dauthlibinjector.side=client"));
            }
            AuthType::SelfLittleSkin => {
                let key = little_skin::get_key(&auth).await?;
                args.push(format!(
                    "-javaagent:{}={}api/yggdrasil",
                    libraries_path::get_authlib_file()
                        .unwrap()
                        .to_string_lossy(),
                    auth.text1.as_ref().unwrap(),
                ));
                args.push(format!(
                    "-Dauthlibinjector.yggdrasil.prefetched={}",
                    hash_helper::gen_base64(&key)
                ));
                args.push(String::from("-Dauthlibinjector.side=client"));
            }
            _ => {}
        }

        args.push(format!(
            "-Dcolormc.dir={}",
            mcml_base::get_base_dir().to_string_lossy()
        ));
        args.push(format!("-Dcolormc.game.uuid={}", self.uuid.to_string()));
        args.push(format!("-Dcolormc.game.name={}", self.name));
        args.push(format!("-Dcolormc.game.version={}", self.version));
        args.push(format!(
            "-Dcolormc.game.dir={}",
            self.get_game_path().to_string_lossy()
        ));

        Ok(args)
    }

    fn replace_all_value(
        &self,
        auth: &LoginObj,
        args: &mut Vec<String>,
        classpath: String,
        native: PathBuf,
        lang: Option<String>,
        assets: Option<String>,
    ) {
        let assets_path = assets_path::get_assets_dir();
        let game_dir = self.get_game_path();
        let assets_index = match assets {
            Some(assets) => assets,
            None => String::from("legacy"),
        };

        let version_name = match self.loader {
            LoaderType::Normal => self.version.clone(),
            LoaderType::Forge => format!(
                "forge-{}-{}",
                self.version,
                self.loader_version.as_ref().unwrap()
            ),
            LoaderType::Fabric => format!(
                "fabric-{}-{}",
                self.version,
                self.loader_version.as_ref().unwrap()
            ),
            LoaderType::Quilt => format!(
                "quilt-{}-{}",
                self.version,
                self.loader_version.as_ref().unwrap()
            ),
            LoaderType::NeoForge => format!(
                "neoforge-{}-{}",
                self.version,
                self.loader_version.as_ref().unwrap()
            ),
            LoaderType::OptiFine => format!(
                "optifine-{}-{}",
                self.version,
                self.loader_version.as_ref().unwrap()
            ),
            LoaderType::Custom => String::from("custom"),
        };

        let sep = if mcml_base::get_system_info().os == Os::Windows {
            ';'
        } else {
            ':'
        };

        let token = if auth.access_token.is_empty() {
            String::from("0")
        } else {
            auth.access_token.clone()
        };

        let user_properties = match lang {
            Some(lang) => format!("{{\"language\":\"{lang}\"}}"),
            None => String::from("{}"),
        };

        let user_type = if auth.auth_type == AuthType::OAuth {
            String::from("msa")
        } else {
            String::from("mojang")
        };

        let mut map = HashMap::new();
        map.insert("{auth_player_name}", auth.user_name.clone());
        map.insert("{version_name}", version_name);
        map.insert("{game_directory}", game_dir.to_string_lossy().to_string());
        map.insert("{assets_root}", assets_path.to_string_lossy().to_string());
        map.insert("{assets_index_name}", assets_index);
        map.insert("{auth_uuid}", auth.uuid.clone());
        map.insert(
            "{auth_access_token}",
            assets_path.to_string_lossy().to_string(),
        );
        map.insert("{user_properties}", user_properties);
        map.insert("{user_type}", user_type);
        map.insert("{version_type}", self.get_version_type());
        map.insert("{natives_directory}", native.to_string_lossy().to_string());
        map.insert(
            "{library_directory}",
            libraries_path::get_lib_dir().to_string_lossy().to_string(),
        );
        map.insert("{classpath_separator}", String::from(sep));
        map.insert("{launcher_name}", String::from(names::MCML));
        map.insert("{launcher_version}", mcml_names::VERSION.clone());
        map.insert("{classpath}", classpath);

        for index in 0..args.len() {
            for (key, value) in map.iter() {
                args[index] = args[index].replace(key, value);
            }
        }
    }

    pub fn make_run_arg(
        &self,
        arg: GameLaunchArg,
        obj: &GameLaunchObj,
        check: bool,
    ) -> CoreResult<Vec<String>> {
        let classpath = String::new();
        let sep = if mcml_base::get_system_info().os == Os::Windows {
            ';'
        }
        else {
            ':'
        };

        

        Ok()
    }
}
