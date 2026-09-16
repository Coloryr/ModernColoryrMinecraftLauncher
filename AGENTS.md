# MCML 项目约定

> 适用于本仓库内的所有协作方（人类与 AI 助手）。

## 1. 屏幕与桌面操作（硬性）

- **禁止截图 / 录屏**：不得对屏幕或任何窗口截图、捕获图像、录屏，包括"为了看界面效果"。
  只有**用户在当轮对话中明确允许**时才可进行。
- **禁止操作用户桌面**：不得移动或点击鼠标、发送键盘输入、切换窗口前台、枚举 / 操纵窗口
  （如 `SetForegroundWindow`、`EnumWindows`、窗口矩形抓取等），除非用户明确允许。
- 需要确认界面效果时：不要自己去"看"，而是
  1. 说明改了哪些文件、期望表现如何，由用户自行启动查看；或
  2. 给出用户可自行打开的入口（命令、URL、界面按钮位置）。

## 2. 临时验证代码

- 为验证效果临时加入的代码（假数据、自动开窗、调试输出等）必须：
  - 在代码中标注 `TEMP` 注释；
  - 验证结束后**立即还原**，不得留在工作区。

## 3. 构建与运行

- **改动代码前的固定流程：先停掉正在运行的 `tauri dev`（连同应用进程），改完并通过验证
  （`cargo check` / `cargo build` + `npm run build`）后再重启。** 运行中的 dev 监听会在文件保存
  中途触发重建 / 热更新，容易出现半成品编译错误、窗口反复重启或行为异常。
- 桌面壳：`cd mcml-gui && npm run tauri dev`（会先起 mcml-vue 的 vite，**端口 1420 必须空闲**）。
- 只跑前端：根目录 `dev-frontend.bat`（vite，1420）；
  浏览器可访问 `http://localhost:1420/?window=<kind>` 预览某个窗口（无 IPC 数据）。
- Rust workspace 在 `mcml-core/`（首次编译较慢）。

## 4. IPC 约定

- 命令名 = 函数名 = **`窗口名_方法名`**，例如 `main_get_instances`、`account_add_account`、
  `window_open_window`、`add_list_dir`。
- 事件名由 `emit_xxx_yyy` 推导：去掉 `emit_` 前缀、`_` 换成 `-`，例如
  `emit_account_change` → `account-change`。
- 命令/事件常量由 `mcml-gui/src-tauri/build.rs` 扫描源码自动生成到
  `mcml-vue/src/lib/{invokes,listens}.ts`（**勿手改**）。新增命令加 `#[tauri::command]`，
  新增事件加 `#[gui_macros::emit]`；单独重新生成可跑 `cd mcml-gui && npm run gen`。
- 磁盘配置用 Rust 命名（snake_case），跨 IPC 传输一律经 `src-tauri/src/dtos/` 转成
  camelCase DTO。

## 5. 目录分工

| 目录 | 职责 |
| --- | --- |
| `mcml-core/` | 启动器内核（cargo workspace，多 crate） |
| `mcml-gui/` | Tauri 桌面壳；`src-tauri/src/windows/<窗口>.rs` 放该窗口的规格 / 模型 / IPC |
| `mcml-vue/` | 前端（Vue3 + Vite）；窗口在 `src/windows/<kind>/`，通用组件在 `src/components/` |

## 6. 临时文件

- 中间产物（下载、解包、日志、试验脚本）统一放 `H:\Temp`，不要写进本仓库。

## 7. 写应用会读取的文件（重要）

- 应用读取的 JSON（`gui_config.json`、`window_save.json` 等）**必须不带 BOM**。
- 本机 `pwsh` 是 **Windows PowerShell 5.1**：`Set-Content -Encoding UTF8` / `Out-File -Encoding UTF8`
  都会写入 UTF-8 BOM，而 Rust 侧 `serde_json` 解析带 BOM 的文件会**失败**，
  表现为配置/窗口几何**静默重置**（启动后按默认值打开，关窗时只写回自己知道的那几条）。
- 写入这类文件请用无 BOM 写法：

  ```powershell
  $utf8 = [System.Text.UTF8Encoding]::new($false)
  [System.IO.File]::WriteAllText($path, $json, $utf8)
  ```

## 8. 网页获取

- 获取网页内容一律使用 `python`（`urllib` / `requests`）或 `curl`，
  不使用其他网页抓取工具（含内置的 WebFetch 等）。

