@echo off
rem 正式发布构建：fat LTO + 单代码生成单元（体积最小、运行最快，但出包慢——
rem 末尾的跨 crate 优化是单线程的，会看到只有一个核在跑）
rem 产出安装包（tauri 打包链）
rem 预发布/日常打包用 build-prerelease.bat
cd /d "%~dp0mcml-gui"
echo 正在构建 release（fat LTO，含安装包）...
call npm run tauri build
pause
