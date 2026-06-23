pub const DOWNLOAD_DIR: &str = "download";
pub const OVERRIDE_DIR: &str = "overrides";
pub const LIBRARIES_DIR: &str = "libraries";
pub const INSTANCE_DIR: &str = "instances";
pub const REMOVE_DIR: &str = "remove";
pub const BACKUP_DIR: &str = "backup";
pub const TEMP_DIR: &str = "temp";
pub const CACHE_DIR: &str = "cache";
pub const VERSION_DIR: &str = "versions";
pub const GAME_DIR: &str = ".minecraft";
pub const GAME_LOGS_DIR: &str = "logs";
pub const GAME_CRASH_DIR: &str = "crash-reports";
pub const GAME_DATAPACK_DIR: &str = "datapacks";
pub const GAME_MODS_DIR: &str = "mods";
pub const GAME_ASSETS_DIR: &str = "assets";
pub const GAME_INDEX_DIR: &str = "indexes";
pub const GAME_OBJECT_DIR: &str = "objects";
pub const GAME_SKIN_DIR: &str = "skins";
pub const GAME_SCREENSHOTS_DIR: &str = "screenshots";
pub const GAME_RESOURCEPACKS_DIR: &str = "resourcepacks";
pub const GAME_SHADERPACKS_DIR: &str = "shaderpacks";
pub const GAME_SAVES_DIR: &str = "saves";
pub const GAME_CONFIG_DIR: &str = "config";
pub const GAME_SCHEMATICS_DIR: &str = "schematics";
pub const JAVA_DIR: &str = "java";
pub const JSON_DIR: &str = "patches";
pub const DEFAULT_DIR: &str = "default";
pub const OPEN_LOADER_DIR: &str = "openloader";
pub const DATA_DIR: &str = "data";
pub const NATIVE_DIR: &str = "native";

pub const LOG_FILE: &str = "logs.log";
pub const LANG_FILE: &str = "lang.txt";
pub const MOD_INFO_FILE: &str = "modfileinfo.json";
pub const GAME_FILE: &str = "game.json";
pub const MOD_PACK_FILE: &str = "Modpack.json";
pub const CONFIG_FILE: &str = "config.json";
pub const SHA_FILE: &str = "sha1";
pub const COLOR_MC_INFO_FILE: &str = "colormc.info.json";
pub const MMCJSON_FILE: &str = "mmc-pack.json";
pub const MMCCFG_FILE: &str = "instance.cfg";
pub const HMCLFILE: &str = "mcbbs.packmeta";
pub const MANIFEST_FILE: &str = "manifest.json";
pub const MODRINTH_FILE: &str = "modrinth.index.json";
pub const ICON_FILE: &str = "icon.png";
pub const SERVER_FILE: &str = "server.json";
pub const SERVER_OLD_FILE: &str = "server.old.json";
pub const LAUNCH_COUNT_FILE: &str = "launch.json";
pub const LOG4J_FILE: &str = "log4j-rce-patch.xml";
pub const LOADER_FILE: &str = "loader.jar";
pub const LEVEL_FILE: &str = "level.dat";
pub const PACK_META_FILE: &str = "pack.mcmeta";
pub const PACK_ICON_FILE: &str = "pack.png";
pub const OPTION_FILE: &str = "options.txt";
pub const GAME_SERVER_FILE: &str = "servers.dat";
pub const VERSION_FILE: &str = "version.json";
pub const AUTH_FILE: &str = "auth.json";
pub const MAVEN_FILE: &str = "maven.json";
pub const JAVA_FILE: &str = "java";
pub const JAVAW_FILE: &str = "javaw.exe";
pub const OPTIFINE_FILE: &str = "optifine.json";
pub const LITELOADER_FILE: &str = "liteloader.json";
pub const MOD_LIST_FILE: &str = "modlist.html";
pub const LATEST_LOG_FILE: &str = "latest.log";
pub const DEBUG_LOG_FILE: &str = "debug.log";
pub const SERVER_MANIFEST_FILE: &str = "server-manifest.json";

pub const MINECRAFT_KEY: &str = "minecraft";
pub const LANG_KEY1: &str = "minecraft/lang/";
pub const LANG_KEY2: &str = "lang";
pub const FML_KEY: &str = "fmlloader";
pub const FORGE_KEY: &str = "forge";
pub const MINECRAFT_FORGE_KEY: &str = "minecraftforge";
pub const NEOFORGE_KEY: &str = "neoforge";
pub const NEOFORGED_KEY: &str = "neoforged";
pub const FABRIC_KEY: &str = "fabric";
pub const FABRIC_MC_KEY: &str = "fabricmc";
pub const FABRIC_LOADER_KEY: &str = "fabric-loader";
pub const QUILT_KEY: &str = "quilt";
pub const QUILT_MC_KEY: &str = "quiltmc";
pub const QUILT_LOADER_KEY: &str = "quilt-loader";

pub const FILE_INSTALL: &str = "install";
pub const FILE_INSTALLER: &str = "installer";
pub const FILE_UNIVERSAL: &str = "universal";
pub const FILE_CLIENT: &str = "client";
pub const FILE_LAUNCHER: &str = "launcher";
pub const FILE_INSTALL_PROFILE: &str = "install_profile.json";

pub const LOG_EXT: &str = ".log";
pub const TXT_EXT: &str = ".txt";
pub const LOG_GZ_EXT: &str = ".log.gz";
pub const ZIP_EXT: &str = ".zip";
pub const JAR_EXT: &str = ".jar";
pub const JSON_EXT: &str = ".json";
pub const DISABLE_EXT: &str = ".disable";
pub const DISABLED_EXT: &str = ".disabled";
pub const LITEMATIC_EXT: &str = ".litematic";
pub const SCHEMATIC_EXT: &str = ".schematic";
pub const SCHEM_EXT: &str = ".schem";
pub const SHA1_EXT: &str = ".sha1";
pub const TAR_GZ_EXT: &str = ".tar.gz";
pub const TAR_XZ_EXT: &str = ".tar.xz";
pub const TGZ_EXT: &str = ".tgz";
pub const TXZ_EXT: &str = ".txz";
pub const MRPACK_EXT: &str = ".mrpack";
pub const DAT_EXT: &str = ".dat";
pub const DAT_OLD_EXT: &str = ".dat_old";
pub const RIO_EXT: &str = ".rio";
pub const MCA_EXT: &str = ".mca";
pub const PNG_EXT: &str = ".png";
pub const NBT_EXT: &str = ".nbt";

pub const DEFAULT_GROUP: &str = " ";

pub const ARG_JAVA_LOCAL: &str = "%JAVA_LOCAL%";
pub const ARG_JAVA_ARG: &str = "%JAVA_ARG%";
pub const ARG_LAUNCHER_DIR: &str = "%LAUNCH_DIR%";
pub const ARG_GAME_NAME: &str = "%GAME_NAME%";
pub const ARG_GAME_UUID: &str = "%GAME_UUID%";
pub const ARG_GAME_DIR: &str = "%GAME_DIR%";
pub const ARG_GAME_BASE_DIR: &str = "%GAME_BASE_DIR%";

pub const MC_MOD_INFO_FILE: &str = "mcmod.info";
pub const MC_MOD_TOML_FILE: &str = "META-INF/mods.toml";
pub const NEO_TOML_FILE: &str = "META-INF/neoforge.mods.toml";
pub const NEO_TOML1_FILE: &str = "neoforge.mods.toml";
pub const MOD_JAR_JAR_DIR: &str = "META-INF/jarjar/";

pub const MCML: &str = "Mcml";
pub const MINECRAFT: &str = "Minecraft";

pub const G1GC: [&str; 8] = [
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+UseG1GC",
    "-XX:MaxGCPauseMillis=200",
    "-XX:G1NewSizePercent=30",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:InitiatingHeapOccupancyPercent=35",
    "-XX:ConcGCThreads=4",
    "-XX:ParallelGCThreads=8",
];

pub const GCZGC: [&str; 2] = ["-XX:+UseZGC", "-XX:+ZGenerational"];

pub const LANG_ZH_CN: &str = "zh_CN";
pub const LANG_EN_US: &str = "en_US";
