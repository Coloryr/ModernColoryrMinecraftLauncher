@echo off
rem 预发布构建：thin LTO + 16 个代码生成单元（能并行，出包快很多）
rem 只产出可执行文件，不打安装包；产物在 根目录\target\prerelease\
rem 用的是根 Cargo.toml 里的 [profile.prerelease]，与 release 各有独立的 target 子目录，
rem 两个模式来回切不会互相刷缓存
rem 正式发布用 build-release.bat
cd /d "%~dp0mcml-gui\src-tauri"
echo 正在构建 prerelease（thin LTO）...
cargo build --profile prerelease
echo.
echo 产物: %~dp0target\prerelease\mcml-gui.exe
pause
