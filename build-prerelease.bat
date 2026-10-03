@echo off
rem 预发布构建：thin LTO + 16 个代码生成单元（能并行，出包快很多）
rem 只产出可执行文件，不打安装包；产物在 根目录\target\prerelease\
rem 用的是根 Cargo.toml 里的 [profile.prerelease]，与 release 各有独立的 target 子目录，
rem 两个模式来回切不会互相刷缓存
rem 正式发布用 build-release.bat

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
