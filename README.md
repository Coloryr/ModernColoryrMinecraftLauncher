# ModernMinecraftLauncher (M²L)

[ColorMC](https://github.com/Coloryr/ColorMC) 的迭代软件，一个 Minecraft 启动器。

**未完成，请勿使用任何东西~~也不要催~~**

## 技术栈

- **内核**：Rust workspace（`mml-core/` 下 16 个子 crate）
- **界面**：Tauri 2 桌面壳（`mml-gui/`）+ Vue3 / Vite 前端（`mml-vue/`）
- **渲染**：wgpu（皮肤 3D 渲染、方块/物品贴图绘制）

## 目录结构

| 目录 | 职责 |
| --- | --- |
| `mml-core/` | 启动器内核，多 crate：游戏启动、版本管理、Mod 加载器、整合包、下载、认证、Java 管理、网络 API（Mojang / Modrinth / CurseForge / Fabric 等）、皮肤与贴图渲染、NBT 解析等 |
| `mml-gui/` | Tauri 桌面壳；窗口规格与 IPC 命令在 `src-tauri/src/windows/`，IPC 类型由 `ipc-gen` 扫描 Rust 源码自动生成 |
| `mml-vue/` | 前端（Vue3 + Vite）；窗口在 `src/windows/`，通用组件在 `src/components/` |

## 开发与构建

依赖：Rust（含 cargo）、Node.js。

| 命令 | 说明 |
| --- | --- |
| `cd mml-gui && npm run tauri dev` | 桌面开发模式（先起 vite，端口 1420 必须空闲） |
| `dev-frontend.bat` | 只跑前端（vite，浏览器访问 `http://localhost:1420/?window=<kind>` 预览窗口，无 IPC 数据） |
| `build-release.bat` | 正式发布构建（fat LTO，含安装包） |
| `build-prerelease.bat` | 预发布构建（thin LTO，只出 exe，更快） |

## TODO

- 添加 LiteLoader 支持

## 许可证

Apache 2.0

```
Copyright 2026 coloryr

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.
```
