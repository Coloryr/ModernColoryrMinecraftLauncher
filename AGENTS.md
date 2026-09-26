# M²L 项目约定

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

- **改任何代码之前，必须先关闭程序。** 这条没有例外——改一行 CSS 也算。流程固定为：
  1. 先停掉正在运行的 `tauri dev` 及其带起的应用进程；
  2. 再改代码；
  3. 改完跑验证（`cargo check -p <crate>` / `npm run build`，见下一条）。
  **改完不需要（也不要）把启动器重新启动**，启动由用户自己来。
  运行中的 dev 监听会在文件保存中途触发重建 / 热更新，容易出现半成品编译错误、窗口反复重启或
  行为异常；用户正在用界面排查问题时尤其不能这么干，会打乱他手上的复现步骤。
  **不要因为「只是小改」「想省一次重启」就跳过第 1 步**，也不要指望靠热更新看效果。
- **`tauri dev` 通常是用户手动起的，关它由你负责，别让用户自己关。**（用户已明确授权。）
  动手前先查进程；发现还在跑就先结束掉，再开始改。参考命令（Windows / bash）：

  ```bash
  tasklist //FI "IMAGENAME eq mml-gui.exe"        # 应用进程
  netstat -ano | grep ":1420"                      # vite 监听（最后一列是 PID）
  taskkill //IM mml-gui.exe //F                   # 结束应用
  taskkill //PID <pid> //F                         # 结束 vite
  ```

  结束应用后 `tauri dev` 的父进程（cargo-tauri / npm）一般会自行退出；顺手 `tasklist` 确认一下
  `cargo.exe` / `node.exe` 也没了。
- **禁止过度检查 / 过度编译测试**：只跑与本次改动直接相关的最小验证——改动所在 crate 的
  `cargo check -p <crate>`、相关的那几个测试（`cargo test -p <crate> --lib`、
  `... --test <name>`，必要时加 `单个用例名` 过滤）。不要动辄
  `cargo check --workspace --all-targets` / `cargo test --workspace` / 整仓全量测试；
  同一结论不要重复跑第二遍（改了代码再验一次除外）。
- 桌面壳：`cd mml-gui && npm run tauri dev`（会先起 mml-vue 的 vite，**端口 1420 必须空闲**）。
- 只跑前端：根目录 `dev-frontend.bat`（vite，1420）；
  浏览器可访问 `http://localhost:1420/?window=<kind>` 预览某个窗口（无 IPC 数据）。
- **出包两个 profile**（各自独立 target 目录，来回切不会互相刷缓存）：
  - 正式发布：根目录 `build-release.bat`（= `cd mml-gui && npm run tauri build`）。
    `[profile.release]` 是 fat LTO + `codegen-units = 1`：体积最小、运行最快，但末尾的
    跨 crate 优化是单线程的，**构建时会看到只有一个核在跑**，含安装包。
  - 预发布：根目录 `build-prerelease.bat`（= `cd mml-gui/src-tauri && cargo build --profile prerelease`）。
    `[profile.prerelease]` 是 thin LTO + `codegen-units = 16`：快很多，体积略大，只出 exe。
  - 预发布也要**安装包**时：`tauri` CLI 不认 `--profile`（`--` 透传给 cargo 会让它去
    `target/release` 找错文件），只能覆盖 release 的设置：
    `CARGO_PROFILE_RELEASE_LTO=thin CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 npm run tauri build`。
    代价是它与正式发布共用 target 目录，来回切会重编末尾那几个单元。
- Rust workspace 在**仓库根目录**（根 `Cargo.toml` 收编 `mml-core/` 全部子 crate 与
  `mml-gui/src-tauri` + `ipc-gen` / `macros`；`workspace.dependencies` 与 `[profile.*]`
  也统一在根清单维护）。target 目录在根目录 `target\`（首次编译较慢）。

## 4. IPC 约定

- 命令名 = 函数名 = **`窗口名_方法名`**，例如 `main_get_instances`、`account_add_account`、
  `window_open_window`、`add_list_dir`。
- **命令与 DTO 的唯一来源是 Rust 源码**，全部由生成器产出。前端缺命令 / 缺字段时，
  先在 Rust 侧补 `#[tauri::command]` 或改 DTO 结构，再跑生成；**不要**在 `bindings.ts`
  或前端另写一套类型（下次生成会被覆盖，且与 Rust 侧脱节）。
  `src/lib/api.ts` 只是手写的便捷包装层，其中的类型一律 `import` 自 `./bindings`。
- 事件名由 `emit_xxx_yyy` 推导：去掉 `emit_` 前缀、`_` 换成 `-`，例如
  `emit_account_change` → `account-change`。
- 前端接口由 `mml-gui/src-tauri/build.rs`（逻辑在 `ipc-gen` 包）扫描 Rust 源码自动生成（**勿手改**）：
  - `mml-vue/src/lib/bindings.ts`：命令的类型化包装（`commands.xxx(...)`）+ 跨 IPC 类型的
    TS 定义。命令签名、DTO 结构、`#[serde(rename_all)]` 都是从 Rust 源码解析出来的
  - `mml-vue/src/lib/listens.ts`：事件名常量
  新增命令加 `#[tauri::command]`，新增事件加 `#[gui_macros::emit]`；单独重新生成可跑
  `cd mml-gui && npm run gen`（或任意 `cargo check`）。
- 扫描与生成逻辑在 `mml-gui/ipc-gen/`（包名 `gui-ipc-gen`，有单测）：
  `src-tauri/build.rs` 只是调用方（算路径、传 `GenConfig`、写文件、打 cargo 指令）；
  改生成规则改那个包，跑 `cd mml-gui/ipc-gen && cargo test` 验证。
- 命令在 `bindings.ts` 里按**来源 .rs 模块**分组（组键取模块最后一段，如
  `windows::account` → `commands.account`），组内剥掉命令名的公共前段
  （`add_modpack_search` → `commands.addModpack.search`）。同一个名字的命令**不能定义两次**，
  否则 `generate_handler!` 会报重复定义。
- 磁盘配置用 Rust 命名（snake_case），跨 IPC 传输一律经 `src-tauri/src/dtos/` 转成
  camelCase DTO。

## 5. 目录分工

| 目录 | 职责 |
| --- | --- |
| `mml-core/` | 启动器内核（多 crate，隶属根目录 workspace） |
| `mml-gui/` | Tauri 桌面壳；`src-tauri/src/windows/<窗口>.rs` 放该窗口的规格 / 模型 / IPC |
| `mml-vue/` | 前端（Vue3 + Vite）；窗口在 `src/windows/<kind>/`，通用组件在 `src/components/` |

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

## 9. 读取文件的范围

- 只读本次任务直接相关的文件：用户在指令里点名的文件，以及为完成改动所必需的那些。
- 非必要不要扩大范围去翻别的 `.rs` 文件。确有必要时，**先说明要读哪个文件、为什么**，
  得到用户同意再读；不要静默地顺藤摸瓜。

## 10. 子代理使用

- **查找代码等探索类工作可以用子代理**（Explore 等，并行摸底、汇总结论）。
- **写计划不能使用代理**：方案设计、计划文件必须由主会话自己完成，不派 Plan 代理。


