@echo off
rem 只启动前端（mcml-vue 的 vite 开发服务器，端口 1420）
rem 不启动 Tauri 桌面壳；浏览器访问 http://localhost:1420 即可预览界面
cd /d "%~dp0mcml-vue"
echo 正在启动前端开发服务器: http://localhost:1420 ...
call npm run dev
pause
