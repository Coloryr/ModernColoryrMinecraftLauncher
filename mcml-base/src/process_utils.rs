use std::{
    collections::HashMap,
    path::Path,
    process::{Child, Command, Stdio},
};

use mcml_names::i18_items::error_type::{CoreResult, ErrorData, ErrorType};

/// 进程启动结果
pub enum LaunchResult {
    /// 正常启动，返回进程句柄，可读取 stdout/stderr 并等待退出
    Normal(Child),
    /// 提权启动，无进程句柄（已通过平台机制独立启动）
    Elevated,
}

/// 检查当前进程是否以管理员/root 权限运行
pub fn is_run_as_admin() -> bool {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::Shell::IsUserAnAdmin;
        unsafe { IsUserAnAdmin() != 0 }
    }
    #[cfg(not(target_os = "windows"))]
    {
        // 通过 `id -u` 检查是否为 root（跨 macOS / Linux）
        std::process::Command::new("id")
            .arg("-u")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim() == "0")
            .unwrap_or(false)
    }
}

/// 启动进程
///
/// 对应 C# 的 `ProcessUtils.Launch(Process, bool)`。
///
/// - `path`: 可执行文件路径
/// - `args`: 程序参数
/// - `env`: 环境变量
/// - `working_dir`: 工作目录
/// - `admin`: 是否请求管理员权限
pub fn launch(
    path: &Path,
    args: &[String],
    env: &HashMap<String, String>,
    working_dir: &Path,
    admin: bool,
) -> CoreResult<LaunchResult> {
    if admin && !is_run_as_admin() {
        launch_with_elevation(path, args, env, working_dir)
    } else {
        let mut cmd = Command::new(path);
        cmd.current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in env {
            cmd.env(key, value);
        }

        for arg in args {
            cmd.arg(arg);
        }

        cmd.spawn()
            .map_err(|err| {
                ErrorType::ProcessError(ErrorData {
                    error: err.to_string(),
                })
            })
            .map(LaunchResult::Normal)
    }
}

/// 以管理员权限启动进程（平台特定实现）
///
/// 提权启动无法重定向 stdout/stderr，因此返回 `LaunchResult::Elevated`。
fn launch_with_elevation(
    path: &Path,
    args: &[String],
    env: &HashMap<String, String>,
    working_dir: &Path,
) -> CoreResult<LaunchResult> {
    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 PowerShell Start-Process -Verb RunAs 提权

        use mcml_names::i18_items::error_type::{ErrorData, ErrorType};
        let mut ps_cmd = format!(
            "Start-Process -FilePath '{}' -WorkingDirectory '{}' -Verb RunAs",
            path.display(),
            working_dir.display()
        );
        for arg in args {
            ps_cmd.push_str(&format!(" -ArgumentList '{}'", arg));
        }

        let mut cmd = Command::new("powershell");
        cmd.arg("-Command").arg(ps_cmd);

        for (key, value) in env {
            cmd.env(key, value);
        }

        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        cmd.spawn().map_err(|err| {
            ErrorType::ProcessError(ErrorData {
                error: err.to_string(),
            })
        })?;
        Ok(LaunchResult::Elevated)
    }

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "do shell script \"'{}' {}\" with administrator privileges",
            path.display(),
            args.iter()
                .map(|a| format!("\\\"{}\\\"", a))
                .collect::<Vec<_>>()
                .join(" ")
        );

        let mut cmd = Command::new("osascript");
        cmd.arg("-e").arg(script);
        cmd.current_dir(working_dir);

        for (key, value) in env {
            cmd.env(key, value);
        }

        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        cmd.spawn()?;
        Ok(LaunchResult::Elevated)
    }

    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("pkexec");
        cmd.arg(path);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.current_dir(working_dir);

        for (key, value) in env {
            cmd.env(key, value);
        }

        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        cmd.spawn()?;
        Ok(LaunchResult::Elevated)
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Admin elevation not supported on this platform",
        ))
    }
}

/// 以管理员权限重新启动 ColorMC 自身
///
/// 对应 C# 的 `ProcessUtils.LaunchAdmin(string[])`。
///
/// - `args`: 启动参数
pub fn launch_admin(args: &[String]) -> CoreResult<()> {
    let current_exe = std::env::current_exe().map_err(|err| {
        ErrorType::ProcessError(ErrorData {
            error: err.to_string(),
        })
    })?;

    #[cfg(target_os = "windows")]
    {
        // Windows: 使用 PowerShell Start-Process -Verb RunAs 提权重启自身
        let mut ps_cmd = format!(
            "Start-Process -FilePath '{}' -Verb RunAs",
            current_exe.display()
        );
        for arg in args {
            ps_cmd.push_str(&format!(" -ArgumentList '{}'", arg));
        }

        let mut child = Command::new("powershell")
            .arg("-Command")
            .arg(ps_cmd)
            .spawn()
            .map_err(|err| {
                ErrorType::ProcessError(ErrorData {
                    error: err.to_string(),
                })
            })?;
        child.wait().map_err(|err| {
            ErrorType::ProcessError(ErrorData {
                error: err.to_string(),
            })
        })?;
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "do shell script \"'{}' {}\" with administrator privileges",
            current_exe.display(),
            args.iter()
                .map(|a| format!("\\\"{}\\\"", a))
                .collect::<Vec<_>>()
                .join(" ")
        );

        let mut child = Command::new("osascript").arg("-e").arg(script).spawn()?;
        child.wait()?;
        Ok(())
    }

    #[cfg(target_os = "linux")]
    {
        let mut cmd = Command::new("pkexec");
        cmd.arg(&current_exe);
        for arg in args {
            cmd.arg(arg);
        }
        let mut child = cmd.spawn()?;
        child.wait()?;
        Ok(())
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Admin elevation not supported on this platform",
        ))
    }
}
