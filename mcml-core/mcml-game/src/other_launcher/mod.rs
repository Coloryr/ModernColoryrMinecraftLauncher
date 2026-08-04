use std::path::Path;

use mcml_base::tools;
use mcml_config::config_obj::{RunArgObj, WindowSettingObj};
use mcml_names::names;

use crate::{
    game_options::InstanceCfg,
    launcher::instance_setting_obj::{CustomLoaderObj, InstanceSettingObj, ServerObj},
    launcher_path::version_path,
    loader::LoaderType,
    other_launcher::{mmc_obj::MMCObj, official_obj::OfficialObj},
};

pub mod mmc_obj;
pub mod official_obj;

/// 检测是否为MMC实例
pub fn is_mmc_version<P: AsRef<Path>>(dir: P) -> bool {
    let file = dir.as_ref().join(names::MMCJSON_FILE);
    let file1 = dir.as_ref().join(names::MMCCFG_FILE);

    file.exists() && file.is_file() && file1.exists() && file1.is_file()
}

impl MMCObj {
    /// 转换为实例信息
    pub fn to_instance(self, cfg: InstanceCfg) -> InstanceSettingObj {
        let mut instance = InstanceSettingObj::default();

        for item in self.components {
            if item.uid.eq_ignore_ascii_case("net.minecraft") {
                instance.version = if item.version.is_empty() {
                    item.cached_version
                } else {
                    item.version
                };
            } else if item.uid.eq_ignore_ascii_case("net.minecraftforge") {
                instance.loader = LoaderType::Forge;
                instance.loader_version = Some(item.version);
            } else if item.uid.eq_ignore_ascii_case("net.neoforged") {
                instance.loader = LoaderType::NeoForge;
                instance.loader_version = Some(item.version);
            } else if item.uid.eq_ignore_ascii_case("net.fabricmc.fabric-loader") {
                instance.loader = LoaderType::Fabric;
                instance.loader_version = Some(item.version);
            } else if item.uid.eq_ignore_ascii_case("org.quiltmc.quilt-loader") {
                instance.loader = LoaderType::Quilt;
                instance.loader_version = Some(item.version);
            } else {
                let custom = CustomLoaderObj {
                    custom_json: true,
                    ..Default::default()
                };
                instance.custom_loader = Some(custom);
            }
        }

        let mut jvm = RunArgObj::default();
        let mut window = WindowSettingObj::default();
        let mut server = ServerObj::default();

        for (key, value) in cfg {
            if key.eq_ignore_ascii_case("name") {
                instance.name = value
            } else if key == "JvmArgs" {
                jvm.jvm_args = Some(value.replace(" ", "\n").replace("\"", ""));
            } else if key == "MaxMemAlloc" {
                jvm.max_memory = value
                    .parse::<u32>()
                    .map(|value| Some(value))
                    .unwrap_or_default();
            } else if key == "MinMemAlloc" {
                jvm.min_memory = value
                    .parse::<u32>()
                    .map(|value| Some(value))
                    .unwrap_or_default();
            } else if key == "MinecraftWinHeight" {
                window.height = value
                    .parse::<u16>()
                    .map(|value| Some(value))
                    .unwrap_or_default();
            } else if key == "MinecraftWinWidth" {
                window.width = value
                    .parse::<u16>()
                    .map(|value| Some(value))
                    .unwrap_or_default();
            } else if key == "LaunchMaximized" {
                window.full_screen = Some(value == "true");
            } else if key == "JoinServerOnLaunch" {
                server.enable = value == "true";
            } else if key == "JoinServerOnLaunchAddress" {
                let mut args = value.split(':');
                if let Some(data) = args.next() {
                    server.ip = Some(data.to_string());
                }
                if let Some(data) = args.next() {
                    server.port = data
                        .parse::<u16>()
                        .map(|value| Some(value))
                        .unwrap_or_default();
                }
            } else if key == "PreLaunchCommand" {
                let args = tools::arg_parse(&value);
                jvm.launch_pre_run = Some(true);
                let mut data = String::new();

                for item in args {
                    let mut temp = item;
                    if temp.starts_with('"') {
                        temp = temp[1..].to_string();
                    }
                    if temp.ends_with('"') {
                        temp = temp[..(temp.len() - 1)].to_string();
                    }

                    if temp == "$INST_JAVA" {
                        data.push_str(names::ARG_JAVA_LOCAL);
                        data.push('\n');
                    } else if temp == "$INST_NAME" {
                        data.push_str(names::ARG_GAME_NAME);
                        data.push('\n');
                    } else if temp == "$INST_DIR" {
                        data.push_str(names::ARG_GAME_BASE_DIR);
                        data.push('\n');
                    } else if temp == "$INST_MC_DIR" {
                        data.push_str(names::ARG_GAME_DIR);
                        data.push('\n');
                    } else if temp == "$INST_ID" {
                        data.push_str(names::ARG_GAME_UUID);
                        data.push('\n');
                    } else if temp == "$INST_JAVA_ARGS" {
                        data.push_str(names::ARG_JAVA_ARG);
                        data.push('\n');
                    } else {
                        data.push_str(&temp);
                        data.push('\n');
                    }
                }

                jvm.pre_run_arg = Some(data);
            } else if key == "iconKey" {
                instance.icon = Some(format!("{value}{}", names::PNG_DOT_EXT));
            }
        }

        instance.jvm_arg = Some(jvm);
        instance.window = Some(window);
        instance.start_server = Some(server);

        instance
    }
}

impl OfficialObj {
    /// 转换为实例信息
    pub fn to_instance(self) -> InstanceSettingObj {
        let mut instance = InstanceSettingObj {
            name: self.id.clone(),
            ..Default::default()
        };

        if self.patches.is_empty() {
            if !self.inherits_from.is_empty() {
                if version_path::have_version(&self.inherits_from) {
                    instance.version = self.inherits_from
                }
            } else {
                if version_path::have_version(&self.id) {
                    instance.version = self.id
                }
            }

            for item in self.libraries {
                if item.name.contains(names::MINECRAFT_FORGE_KEY) {
                    let args: Vec<&str> = item.name.split(':').collect();
                    if args.len() >= 3 && (args[1] == names::FORGE_KEY || args[1] == names::FML_KEY)
                    {
                        let names: Vec<&str> = args[2].split('-').collect();
                        if names.len() >= 2 && version_path::have_version(names[0]) {
                            instance.loader = LoaderType::Forge;
                            instance.loader_version = Some(names[1].to_string());
                            break;
                        }
                    }
                } else if item.name.contains(names::NEOFORGED_KEY) {
                    for index in 0..self.arguments.game.len() {
                        let item = &self.arguments.game[index];
                        if let Some(data) = item.as_string()
                            && (data == "--fml.neoForgeVersion" || data == "--fml.forgeVersion")
                            && self.arguments.game.len() > index + 1
                        {
                            instance.loader = LoaderType::NeoForge;
                            instance.loader_version =
                                Some(self.arguments.game[index + 1].as_string().unwrap());
                            break;
                        }
                    }
                } else if item.name.contains(names::FABRIC_MC_KEY) {
                    let args: Vec<&str> = item.name.split(':').collect();
                    if args.len() >= 3 && args[1] == names::FABRIC_LOADER_KEY {
                        instance.loader = LoaderType::Fabric;
                        instance.loader_version = Some(args[2].to_string());
                        break;
                    }
                } else if item.name.contains(names::QUILT_MC_KEY) {
                    let args: Vec<&str> = item.name.split(':').collect();
                    if args.len() >= 3 && args[1] == names::QUILT_LOADER_KEY {
                        instance.loader = LoaderType::Fabric;
                        instance.loader_version = Some(args[2].to_string());
                        break;
                    }
                }
            }
        } else {
            for item in self.patches {
                if item.id == "game" {
                    instance.version = item.version;
                } else if item.id == "forge" {
                    instance.loader = LoaderType::Forge;
                    instance.loader_version = Some(item.version);
                } else if item.id == "fabric" {
                    instance.loader = LoaderType::Fabric;
                    instance.loader_version = Some(item.version);
                } else if item.id == "neoforge" {
                    instance.loader = LoaderType::NeoForge;
                    instance.loader_version = Some(item.version);
                } else if item.id == "quilt" {
                    instance.loader = LoaderType::Quilt;
                    instance.loader_version = Some(item.version);
                }
            }
        }

        instance
    }
}
