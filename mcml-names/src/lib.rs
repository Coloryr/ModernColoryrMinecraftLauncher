use const_format::formatcp;

/// 启动器主版本号
pub const VERSION_NUM: i32 = 1;
/// 启动器日期
pub const DATE: &str = "20260503";
/// 启动器版本号
pub const VERSION: &str = formatcp!("1.{}.{DATE}", VERSION_NUM);

pub const NAME_DOWNLOAD_DIR: &str = "download";
pub const NAME_OVERRIDE_DIR: &str = "overrides";
pub const NAME_LIB_DIR: &str = "libraries";
pub const NAME_INSTANCE_DIR: &str = "instances";
pub const NAME_REMOVE_DIR: &str = "remove";
pub const NAME_BACKUP_DIR: &str = "backup";
pub const NAME_TEMP_DIR: &str = "temp";
pub const NAME_CACHE_DIR: &str = "cache";
pub const NAME_VERSION_DIR: &str = "versions";
pub const NAME_GAME_DIR: &str = ".minecraft";
pub const NAME_GAME_LOG_DIR: &str = "logs";
pub const NAME_GAME_CRASH_LOG_DIR: &str = "crash-reports";
pub const NAME_GAME_DATAPACK_DIR: &str = "datapacks";
pub const NAME_GAME_MOD_DIR: &str = "mods";
pub const NAME_GAME_ASSETS_DIR: &str = "assets";
pub const NAME_GAME_INDEX_DIR: &str = "indexes";
pub const NAME_GAME_OBJECT_DIR: &str = "objects";
pub const NAME_GAME_SKIN_DIR: &str = "skins";
pub const NAME_GAME_SCREEN_SHOT_DIR: &str = "screenshots";
pub const NAME_GAME_RESOURCEPACK_DIR: &str = "resourcepacks";
pub const NAME_GAME_SHADERPACK_DIR: &str = "shaderpacks";
pub const NAME_GAME_SAVES_DIR: &str = "saves";
pub const NAME_GAME_CONFIG_DIR: &str = "config";
pub const NAME_GAME_SCHEMATICS_DIR: &str = "schematics";
pub const NAME_JAVA_DIR: &str = "java";
pub const NAME_JSON_DIR: &str = "patches";
pub const NAME_DEFAULT_DIR: &str = "default";
pub const NAME_OPEN_LOADER_DIR: &str = "openloader";
pub const NAME_DATA_DIR: &str = "data";

pub const NAME_MOD_INFO_FILE: &str = "modfileinfo.json";
pub const NAME_GAME_FILE: &str = "game.json";
pub const NAME_MOD_PACK_FILE: &str = "Modpack.json";
pub const NAME_CONFIG_FILE: &str = "config.json";
pub const NAME_SHA_FILE: &str = "sha1";
pub const NAME_COLOR_MC_INFO_FILE: &str = "colormc.info.json";
pub const NAME_MMCJSON_FILE: &str = "mmc-pack.json";
pub const NAME_MMCCFG_FILE: &str = "instance.cfg";
pub const NAME_HMCLFILE: &str = "mcbbs.packmeta";
pub const NAME_MANIFEST_FILE: &str = "manifest.json";
pub const NAME_MODRINTH_FILE: &str = "modrinth.index.json";
pub const NAME_ICON_FILE: &str = "icon.png";
pub const NAME_SERVER_FILE: &str = "server.json";
pub const NAME_SERVER_OLD_FILE: &str = "server.old.json";
pub const NAME_LAUNCH_COUNT_FILE: &str = "launch.json";
pub const NAME_LOG4J_FILE: &str = "log4j-rce-patch.xml";
pub const NAME_LOADER_FILE: &str = "loader.jar";
pub const NAME_LEVEL_FILE: &str = "level.dat";
pub const NAME_PACK_META_FILE: &str = "pack.mcmeta";
pub const NAME_PACK_ICON_FILE: &str = "pack.png";
pub const NAME_OPTION_FILE: &str = "options.txt";
pub const NAME_GAME_SERVER_FILE: &str = "servers.dat";
pub const NAME_VERSION_FILE: &str = "version.json";
pub const NAME_AUTH_FILE: &str = "auth.json";
pub const NAME_MAVEN_FILE: &str = "maven.json";
pub const NAME_JAVA_FILE: &str = "java";
pub const NAME_JAVAW_FILE: &str = "javaw.exe";
pub const NAME_OPTIFINE_FILE: &str = "optifine.json";
pub const NAME_MOD_LIST_FILE: &str = "modlist.html";
pub const NAME_LATEST_LOG_FILE: &str = "latest.log";
pub const NAME_DEBUG_LOG_FILE: &str = "debug.log";
pub const NAME_SERVER_MANIFEST_FILE: &str = "server-manifest.json";

pub const NAME_MINECRAFT_KEY: &str = "minecraft";
pub const NAME_LANG_KEY1: &str = "minecraft/lang/";
pub const NAME_LANG_KEY2: &str = "lang";
pub const NAME_FML_KEY: &str = "fmlloader";
pub const NAME_FORGE_KEY: &str = "forge";
pub const NAME_MINECRAFT_FORGE_KEY: &str = "minecraftforge";
pub const NAME_NEO_FORGE_KEY: &str = "neoforge";
pub const NAME_NEO_FORGED_KEY: &str = "neoforged";
pub const NAME_FABRIC_KEY: &str = "fabric";
pub const NAME_FABRIC_MC_KEY: &str = "fabricmc";
pub const NAME_FABRIC_LOADER_KEY: &str = "fabric-loader";
pub const NAME_QUILT_KEY: &str = "quilt";
pub const NAME_QUILT_MC_KEY: &str = "quiltmc";
pub const NAME_QUILT_LOADER_KEY: &str = "quilt-loader";

pub const NAME_FORGE_FILE1: &str = "installer";
pub const NAME_FORGE_FILE2: &str = "universal";
pub const NAME_FORGE_FILE3: &str = "client";
pub const NAME_FORGE_FILE4: &str = "launcher";
pub const NAME_FORGE_INSTALL_FILE: &str = "install_profile.json";

pub const NAME_LOG_EXT: &str = ".log";
pub const NAME_TXT_EXT: &str = ".txt";
pub const NAME_LOG_GZ_EXT: &str = ".log.gz";
pub const NAME_ZIP_EXT: &str = ".zip";
pub const NAME_JAR_EXT: &str = ".jar";
pub const NAME_JSON_EXT: &str = ".json";
pub const NAME_DISABLE_EXT: &str = ".disable";
pub const NAME_DISABLED_EXT: &str = ".disabled";
pub const NAME_LITEMATIC_EXT: &str = ".litematic";
pub const NAME_SCHEMATIC_EXT: &str = ".schematic";
pub const NAME_SCHEM_EXT: &str = ".schem";
pub const NAME_SHA1_EXT: &str = ".sha1";
pub const NAME_TAR_GZ_EXT: &str = ".tar.gz";
pub const NAME_MRPACK_EXT: &str = ".mrpack";
pub const NAME_DAT_EXT: &str = ".dat";
pub const NAME_DAT_OLD_EXT: &str = ".dat_old";
pub const NAME_RIO_EXT: &str = ".rio";
pub const NAME_MCA_EXT: &str = ".mca";
pub const NAME_PNG_EXT: &str = ".png";
pub const NAME_NBT_EXT: &str = ".nbt";

pub const NAME_DEFAULT_GROUP: &str = " ";

pub const NAME_ARG_JAVA_LOCAL: &str = "%JAVA_LOCAL%";
pub const NAME_ARG_JAVA_ARG: &str = "%JAVA_ARG%";
pub const NAME_ARG_LAUNCHER_DIR: &str = "%LAUNCH_DIR%";
pub const NAME_ARG_GAME_NAME: &str = "%GAME_NAME%";
pub const NAME_ARG_GAME_UUID: &str = "%GAME_UUID%";
pub const NAME_ARG_GAME_DIR: &str = "%GAME_DIR%";
pub const NAME_ARG_GAME_BASE_DIR: &str = "%GAME_BASE_DIR%";

pub const NAME_MC_MOD_INFO_FILE: &str = "mcmod.info";
pub const NAME_MC_MOD_TOML_FILE: &str = "META-INF/mods.toml";
pub const NAME_NEO_TOML_FILE: &str = "META-INF/neoforge.mods.toml";
pub const NAME_NEO_TOML1_FILE: &str = "neoforge.mods.toml";
pub const NAME_MOD_JAR_JAR_DIR: &str = "META-INF/jarjar/";

pub const NAME_GCARG_G1_GC: [&str; 8] = [
    "-XX:+UnlockExperimentalVMOptions",
    "-XX:+UseG1GC",
    "-XX:MaxGCPauseMillis=200",
    "-XX:G1NewSizePercent=30",
    "-XX:G1MaxNewSizePercent=40",
    "-XX:InitiatingHeapOccupancyPercent=35",
    "-XX:ConcGCThreads=4",
    "-XX:ParallelGCThreads=8",
];

pub const NAME_GCZGC: [&str; 2] = ["-XX:+UseZGC", "-XX:+ZGenerational"];
