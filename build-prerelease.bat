@echo off
rem 预发布构建：thin LTO + 16 个代码生成单元（能并行，出包快很多）
rem 只产出可执行文件，不打安装包；产物在 根目录\target\prerelease\
rem 用的是根 Cargo.toml 里的 [profile.prerelease]，与 release 各有独立的 target 子目录，
rem 两个模式来回切不会互相刷缓存
rem 正式发布用 build-release.bat

rem ---- 构建期临时目录 ----
rem 主修复在 .cargo\config.toml 的 [env]（把 TMP/TEMP 指到仓库的 target\，覆盖一切走 cargo
rem 的构建：npm run tauri build / tauri dev / cargo build / cargo test）。
rem 这里再设一次是为了**非 cargo 的子进程**（打包链等不经过 cargo 的部分）。
rem 起因：系统 %TEMP% 会让 MSVC 的 lib.exe / link.exe 报
rem   LINK : fatal error LNK1104: 无法打开文件 "...\lnk{GUID}.tmp"（退出码 1104）。
set "MML_BUILD_TMP=%~dp0target"
if not exist "%MML_BUILD_TMP%" mkdir "%MML_BUILD_TMP%"
set "TMP=%MML_BUILD_TMP%"
set "TEMP=%MML_BUILD_TMP%"

rem vendor 插件要先还原（cargo 解析依赖图时就要读它的 Cargo.toml，早于构建脚本）；
rem 脚本幂等，已还原过会直接跳过
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0mml-gui\vendor\prepare.ps1"
if errorlevel 1 (
    echo vendor 插件还原失败，已中止构建。
    pause
    exit /b 1
)

cd /d "%~dp0mml-gui\src-tauri"
echo 正在构建 prerelease（thin LTO）...
cargo build --profile prerelease
echo.
echo 产物: %~dp0target\prerelease\mml-gui.exe
pause
