use std::{collections::HashMap, io::BufRead, path::PathBuf, sync::Arc, time::Instant};

use async_trait::async_trait;
use mcml_auth::LoginObj;
use mcml_base::process_utils;
use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType, FileNotExistsData};
use tokio_util::sync::CancellationToken;

use crate::{
    game_arg::GameLaunchObj,
    game_log::GameLog,
    game_saves::SaveObj,
    launcher::game_setting_obj::{GameSettingObj, ServerObj},
    launcher_path::libraries_path,
    loader::LoaderType,
};

/// 界面回调
#[async_trait]
pub trait ILaunchGui {
    /// 启动状态修改
    fn update_state(&self, setting: &GameSettingObj, state: LaunchState);
    /// 登陆失败
    async fn login_fail(&self, auth: &LoginObj) -> bool;
    /// 请求是否要下载文件
    async fn requesst_download_file(&self) -> bool;
    /// 没有合适的java
    fn no_java(&self, java: i32);
}

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
    /// ASM端口
    pub mixin: Option<u16>,
    /// 启动时设置语言
    pub lang: Option<String>,
    /// 界面显示回调
    pub gui: Option<Box<dyn ILaunchGui>>,
}

/// 游戏实例处理完成后的参数
pub struct GameRunObj {
    /// 游戏实例
    pub obj: Arc<GameSettingObj>,
    /// 登陆的账户
    pub auth: Arc<LoginObj>,
    /// 运行路径
    pub path: PathBuf,
    /// Java路径
    pub java: PathBuf,
    /// 启动参数
    pub args: Vec<String>,
    /// 运行环境
    pub env: HashMap<String, String>,
    /// 是否管理员方式启动
    pub admin: bool,
}

/// 实例启动状态
pub enum LaunchState {
    /// 登陆账户
    Login,
    /// 检查文件
    Check,
    /// 读取信息
    ReadInfo,
    /// 下载文件
    Download,
    /// 准备启动参数
    Jvm,
    /// 启动前运行
    Pre,
    /// 启动后运行
    Post,
    /// 结束
    End,
    LoadServerPack,
    CheckServerPack,
    DownloadServerPack,
    DownloadServerPackEnd,
}

/// 创建启动命令参数
pub struct LaunchCmd {
    pub java: PathBuf,
    pub dir: PathBuf,
    pub arg: Vec<String>,
    pub env: HashMap<String, String>,
}

pub struct GameHandle {
    
}

impl GameSettingObj {
    /// 尝试刷新账户，若失败则询问是否离线模式
    /// - `arg`: 启动参数
    async fn auth_login(
        &self,
        arg: &mut GameLaunchArg,
        cancel: &CancellationToken,
    ) -> CoreResult<()> {
        let start = Instant::now();
        let res = Arc::make_mut(&mut arg.auth).refresh(cancel).await;
        let time = start.elapsed();
        crate::add_game_log_item(&self.uuid, GameLog::LoginTime(time));

        if cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }

        match res {
            Ok(_) => {
                arg.auth.save();
                Ok(())
            }
            Err(err) => {
                if let Some(gui) = &arg.gui
                    && gui.login_fail(&arg.auth).await
                {
                    let old = &arg.auth;
                    arg.auth = Arc::new(LoginObj {
                        user_name: old.user_name.clone(),
                        uuid: old.uuid.clone(),
                        access_token: Default::default(),
                        client_token: Default::default(),
                        auth_type: Default::default(),
                        text1: Default::default(),
                        text2: Default::default(),
                        last_login: Default::default(),
                    });
                    Ok(())
                } else {
                    Err(err)
                }
            }
        }
    }

    /// 检查缺失的文件，自动下载，完成后返回启动配置
    /// - `arg`: 启动参数
    async fn check_game_file(&self, arg: &mut GameLaunchArg) -> CoreResult<GameLaunchObj> {
        if let Some(gui) = &arg.gui {
            gui.update_state(self, LaunchState::Check);
        }

        let start = Instant::now();
        let authlib = libraries_path::check_authlib(&arg.auth.auth_type).await?;
        let obj = self.make_game_launch_obj(arg).await?;
        let mut check = self.get_lost_game_file(&obj).await?;
        let time = start.elapsed();
        crate::add_game_log_item(&self.uuid, GameLog::CheckGameFileTime(time));

        if let Some(data) = authlib {
            check.push(data);
        }

        if check.is_empty() {
            Ok(obj)
        } else {
            let download = if mcml_config::read_config().http.auto_download == false
                && let Some(gui) = &arg.gui
            {
                gui.requesst_download_file().await
            } else {
                true
            };

            if download {
                if let Some(gui) = &arg.gui {
                    gui.update_state(self, LaunchState::Download);
                }
                let start = Instant::now();
                let res = mcml_downloader::run_download_task(check).await;
                let time = start.elapsed();
                crate::add_game_log_item(&self.uuid, GameLog::DownloadFileTime(time));
                if !res {
                    Err(ErrorType::DownloadFileFail)
                } else {
                    Ok(obj)
                }
            } else {
                Ok(obj)
            }
        }
    }

    /// 创建环境变量
    fn make_env_arg(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();
        let envstr = self
            .jvm_arg
            .clone()
            .map(|item| item.jvm_env.clone())
            .unwrap_or(mcml_config::read_config().jvm_arg.jvm_env.clone())
            .unwrap_or(String::new());

        if !envstr.is_empty() {
            let list: Vec<&str> = envstr.split('\n').collect();
            for item in list {
                let temp = item.trim();
                let index: Vec<&str> = temp.splitn(2, '=').collect();
                if index.len() == 1 {
                    env.insert(index[0].to_string(), String::new());
                } else {
                    env.insert(index[0].to_string(), index[1].to_string());
                }
            }
        }

        env
    }

    // /// 检查服务器包更新
    // async fn server_pack_update(&self) -> CoreResult<()> {
    //     if !self.is_modpack || self.source_type != SourceType::ServerPack || None == self.server_url
    //     {
    //         Ok(())
    //     } else {
    //         Ok(())
    //     }
    // }

    /// 获取启动使用的java
    fn get_java(&self, arg: &GameLaunchArg, obj: &GameLaunchObj) -> Option<PathBuf> {
        if let Some(data) = &self.jvm_local {
            let path = PathBuf::from(data);

            if path.exists() {
                return Some(path);
            } else {
                crate::add_game_log_item(&self.uuid, GameLog::JavaLocalRedirect);
            }
        }

        if mcml_jvms::get_all_java().is_empty() {
            mcml_jvms::scan_java();
        }

        if let Some(name) = &self.jvm_name {
            if let Some(data) = mcml_jvms::get_java_info(&name) {
                if data.path.exists() {
                    return Some(data.path.clone());
                }
            }

            crate::add_game_log_item(&self.uuid, GameLog::JavaLocalRedirect);
        }

        let mut list: Vec<_> = obj.java_versions.clone().into_iter().collect();
        list.sort_unstable_by(|a, b| b.cmp(a));
        for item in list.iter() {
            let jvm = mcml_jvms::get_java(*item, true);
            if let Some(jvm) = jvm {
                if jvm.path.exists() {
                    return Some(jvm.path.clone());
                }
            }
        }

        if let Some(gui) = &arg.gui {
            gui.no_java(*list.first().unwrap());
        }

        None
    }

    /// 执行指令
    /// - `cmd`: 命令行（换行分隔的指令）
    /// - `env`: 环境变量
    /// - `wait_run`: 是否等待执行完成
    /// - `admin`: 是否以管理员方式启动
    fn cmd_run(
        &self,
        cmd: &str,
        env: &HashMap<String, String>,
        wait_run: bool,
        admin: bool,
    ) -> CoreResult<()> {
        let args: Vec<&str> = cmd.split('\n').collect();
        let name = args[0].trim();

        let mut path = PathBuf::from(name);
        if !path.exists() {
            path = self.get_base_path().join(name);
        }

        if !path.exists() {
            path = self.get_game_path().join(name);
        }

        if !path.exists() {
            return Err(ErrorType::FileNotExists(FileNotExistsData {
                file: PathBuf::from(name),
            }));
        }

        // 收集参数（跳过第0个，即程序名本身）
        let proc_args: Vec<String> = args
            .iter()
            .skip(1)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        match process_utils::launch(&path, &proc_args, env, &self.get_game_path(), admin) {
            Ok(result) => match result {
                process_utils::LaunchResult::Normal(mut child) => {
                    // 获取 stdout 和 stderr 用于异步读取
                    let stdout = child.stdout.take();
                    let stderr = child.stderr.take();

                    let uuid = self.uuid;

                    // 读取 stdout 线程
                    if let Some(stdout) = stdout {
                        let uuid1 = uuid;
                        std::thread::spawn(move || {
                            let reader = std::io::BufReader::new(stdout);
                            for line in reader.lines() {
                                if let Ok(line) = line {
                                    crate::add_game_log(uuid1, &line);
                                }
                            }
                        });
                    }

                    // 读取 stderr 线程
                    if let Some(stderr) = stderr {
                        let uuid2 = uuid;
                        std::thread::spawn(move || {
                            let reader = std::io::BufReader::new(stderr);
                            for line in reader.lines() {
                                if let Ok(line) = line {
                                    crate::add_game_log(uuid2, &line);
                                }
                            }
                        });
                    }

                    // 是否与游戏同时启动
                    if !wait_run {
                        return Ok(());
                    }

                    child.wait().map_err(|e| {
                        ErrorType::StreamError(ErrorData {
                            error: e.to_string(),
                        })
                    })?;

                    Ok(())
                }
                process_utils::LaunchResult::Elevated => {
                    // 提权启动，无进程句柄，直接返回
                    Ok(())
                }
            },
            Err(err) => Err(err),
        }
    }

    /// 生成游戏启动参数
    /// - `arg`: 启动参数
    pub async fn create_game_cmd(
        &self,
        arg: &mut GameLaunchArg,
        cancel: &CancellationToken,
    ) -> CoreResult<LaunchCmd> {
        if self.version.is_empty() {
            return Err(ErrorType::VersionInfoError);
        }

        if self.loader != LoaderType::Normal || self.loader != LoaderType::Custom {
            if let Some(data) = &self.loader_version
                && data.is_empty()
            {
                return Err(ErrorType::VersionInfoError);
            } else {
                return Err(ErrorType::VersionInfoError);
            }
        }

        if self.loader == LoaderType::Custom && !self.get_loader_file().exists() {
            return Err(ErrorType::VersionInfoError);
        }

        if let Some(gui) = &arg.gui {
            gui.update_state(&self, LaunchState::Login);
        }

        self.auth_login(arg, cancel).await?;

        if cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }

        if let Some(gui) = &arg.gui {
            gui.update_state(&self, LaunchState::ReadInfo);
        }

        let mut obj = self.make_game_launch_obj(arg).await?;

        if cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }

        let java = self.get_java(arg, &obj);

        if java.is_none() {
            return Err(ErrorType::JavaNotFound);
        }

        if cancel.is_cancelled() {
            return Err(ErrorType::TaskCancel);
        }

        if let Some(gui) = &arg.gui {
            gui.update_state(&self, LaunchState::Jvm);
        }

        let args = self.make_run_arg(&arg, &mut obj, false);
        let env = self.make_env_arg();

        Ok(LaunchCmd {
            java: java.unwrap(),
            dir: self.get_game_path(),
            arg: args,
            env,
        })
    }

    pub async fn start_game(&self, arg: &GameLaunchArg, cancel: &CancellationToken) -> CoreResult<>
}
