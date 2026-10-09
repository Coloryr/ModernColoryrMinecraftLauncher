@echo off
rem 正式发布构建：fat LTO + 单代码生成单元（体积最小、运行最快，但出包慢——
rem 末尾的跨 crate 优化是单线程的，会看到只有一个核在跑）
rem 产出安装包（tauri 打包链）
rem 预发布/日常打包用 build-prerelease.bat

rem ---- 构建期临时目录 ----
rem 主修复在 .cargo\config.toml 的 [env]（把 TMP/TEMP 指到仓库的 target\，覆盖一切走 cargo
rem 的构建：npm run tauri build / tauri dev / cargo build / cargo test）。
rem 这里再设一次是为了**非 cargo 的子进程**：tauri 打包链（NSIS / WiX）不经过 cargo，
rem 拿不到 [env] 注入的值。
rem 起因：系统 %TEMP% 会让 MSVC 的 lib.exe / link.exe 报
rem   LINK : fatal error LNK1104: 无法打开文件 "...\lnk{GUID}.tmp"（退出码 1104）。
set "MML_BUILD_TMP=%~dp0target"
if not exist "%MML_BUILD_TMP%" mkdir "%MML_BUILD_TMP%"
set "TMP=%MML_BUILD_TMP%"
set "TEMP=%MML_BUILD_TMP%"

rem vendor 插件（tauri-plugin-decoration）要先还原：它由 patch 打到上游源码上，
rem 而 cargo 解析依赖图时就要读它的 Cargo.toml，早于任何构建脚本，必须在这里先跑。
rem 脚本幂等，已还原过会直接跳过。
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0mml-gui\vendor\prepare.ps1"
if errorlevel 1 (
    echo vendor 插件还原失败，已中止构建。
    pause
    exit /b 1
)

cd /d "%~dp0mml-gui"
echo 正在构建 release（fat LTO，含安装包）...
call npm run tauri build
pause
