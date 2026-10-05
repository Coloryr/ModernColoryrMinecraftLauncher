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

- **改任何代码之前，必须先关闭程序——没有例外。** 流程固定为：
  1. 先停掉正在运行的 `tauri dev` 及其带起的应用进程；
  2. 再改代码；
  3. 改完跑验证（`cargo check -p <crate>` / `npm run build`，见下一条）。
  **改完不需要（也不要）把启动器重新启动**，启动由用户自己来。
  运行中的 dev 监听会在文件保存中途触发重建 / 热更新，容易出现半成品编译错误、窗口反复重启或
  行为异常；用户正在用界面排查问题时尤其不能这么干，会打乱他手上的复现步骤。
  **不要因为「只是小改」「想省一次重启」就跳过第 1 步**，也不要指望靠热更新看效果。
- **前端一视同仁：`mml-vue/` 下的样式 / 文案 / 组件微调也要先关程序再改。**
  "只是改几个字""vite 会热更新"都不是理由——热更新恰恰是在文件改到一半时把半成品送进
  正在运行的界面，而用户很可能正拿着那个界面复现问题。
- **关不掉就先提醒用户，别硬改。** 出现下面任一情况，就把改动先放着，
  告诉用户"请先关闭启动器（`tauri dev` / `mml-gui.exe` / 占用 1420 的 vite），关好我再动手"，
  等用户确认已关闭再继续：
  - 结束进程失败（无权限、`taskkill` 报错、进程结束不掉）；
  - 不确定某个进程是不是这个项目的（例如有别的 Electron / Node 程序在跑）；
  - 用户说自己正开着界面（哪怕我这边查不到进程）。
- **`tauri dev` 通常是用户手动起的，关它由你负责，别让用户自己关。**（用户已明确授权。）
  动手前先查进程；发现还在跑就先结束掉，再开始改；**结束失败或进程归属不明时走上一条，
  提醒用户关闭**。参考命令（Windows / bash）：

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

- 中间产物（下载、解包、日志、试验脚本）统一放根目录 `target/temp`（`target\` 已被 git
  忽略），不要写进版本控制；也不要放系统盘 `%TEMP%`（会被清理）。
- **测试的临时目录同样放 `target/temp`**：不要用 `std::env::temp_dir()`（系统 `%TEMP%`）——
  它会被清理，测试留下的样本下次就找不到了，散在系统临时目录里也根本没法排查。
  统一走 `mml_testutil::temp_dir()` / `temp_path(..)`（`mml-core/mml-testutil`，
  按 `[dev-dependencies]` 引入），不要在测试里自己拼 `CARGO_MANIFEST_DIR/../..`。
  - **只改测试代码**：生产代码里的临时目录（如 `base_archive` 解包时的中转目录）本来就该用
    系统临时目录，不要换成 `mml_testutil` —— 那是 dev-dependency，正式产物里不存在。
  - 每个用例**各用各的子目录**（名字带用例特征 + `Uuid` / 进程 id）：落盘是异步的，
    两个用例共用一个目录时，先跑完的那个可能在另一个 `remove_dir_all` 之后才写文件，
    于是后者读到前者的数据（表现为随机失败、单独跑就过）。
  - 内核那几个系统（`mml_base` / `mml_names` / `mml_log` / `mml_config`）是**进程级单例**，
    测试里不要各自写一份启动代码：`mml_log::STREAM` 是 `OnceLock`，第二次 `start()` 会
    **panic**。同一 crate 内共用一个 boot（见 `mml-game/src/test_support.rs`）。

## 6.1 TASK.md（临时任务文件）

- **`TASK.md` 是临时的任务清单，不是仓库文档**：一轮任务做完就**删掉**，不要把它当交接文档
  长期维护，也不要往里写"已完成"记录。
- **不要主动改 `TASK.md`**：既不改写、也不追加。只有用户明确说"更新 TASK.md / 写进 TASK.md"
  时才动它。
- 开工时它可以作为待办来源读一次；读完就按它干活，干完把文件删掉（`git rm` / 直接删）。
- 需要长期保留的约定，写进 `AGENTS.md`（本文件）；**不要**写进 `TASK.md`。

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

## 11. 缓存目录

- 渲染测试的运行缓存（下载的客户端 jar 等）在 `mml-core/mml-tex-draw/tests/out/run`
  （`render_all.rs` 的 `run_dir`，跨进程复用）。
- Claude 的工作缓存（反编译 jar、提取脚本等）放根目录 `target/temp`（与 §6 同源，
  不再放 H 盘）。

## 12. 前后端分工（默认只做前端，必要就一起改完）

- **默认只做前端**：界面与接线任务只改 `mml-vue/`（UI、状态、调用 `commands.xxx` 的接线）。
  "默认"是省事的起点，不是禁令——**功能确实需要后端时，就把后端一起改完，不要只留骨架**。
- **判据是"前端拿不到的能力"**：重启进程、直接读写磁盘文件、访问注册表、拉起外部程序、
  拿本机信息之类，不写后端就根本跑不通。这种情况把 `#[tauri::command]` 连同**函数体**
  一起实现，改完跑 `cd mml-gui && npm run gen`（或任意 `cargo check`）重新生成
  `bindings.ts`。
  **不要留 `todo!()` / 空实现**：那会让用户点下去的那一刻 panic、或者毫无反应，
  比不改还糟（`window_restart_app` 就是这种"必须实现"的例子）。
- **什么时候才只写骨架**：用户明确说了"只加接口 / 函数体我自己写"的时候——补
  `#[tauri::command]` 声明与 DTO 定义（含注册进 `generate_handler!`），函数体留空，
  跑生成让前端接线类型对得上。
- **用户添加 DTO 后要同步前端**：用户在 Rust 侧添加 / 修改 DTO 模型时，读取对应源码，
  跑 `cd mml-gui && npm run gen` 重新生成 `bindings.ts`，并在前端接上新结构。
- 不论哪种情况，**命令与 DTO 的唯一来源仍是 Rust 源码**（见 §4），前端不另写一套类型。

## 13. 复杂问题先写日志调试（硬性）

- **凡是「原因不明显」的问题，先加日志把链路打断点，再动手改代码**；不要靠读代码猜、
  更不要一次改多处"可能的原因"。判据很简单——**如果你需要用"可能是…"来解释现象，
  那就属于本条**（并发 / 时序 / 缓存单飞 / 中断与取消 / 跨窗口状态 / 只有用户环境才复现
  的问题，都是典型）。
- 流程固定为：

  1. **先定位**：在关键分叉处加日志（进入 / 退出 / 耗时 / 状态值），让日志把范围钉死到
     "哪一层、哪一行"；
  2. **再修复**：只改被日志指认的那一处，不做顺手重构；
  3. **后清理**：问题解决后**立即删除全部临时日志**，只保留有长期价值的运行日志。

- 日志写法约定：

  - 临时诊断日志统一带 **`[TEMP]` 前缀**，便于 `Select-String "TEMP"` 一次性找全并清除
    （与 §2「临时验证代码必须标 `TEMP` 并在验证后立即还原」同源）；
  - 用 `mml_log::info(...)`（**参数是 `String`，不是 `&str`**，字面量要 `.to_string()` 或
    用 `format!`），前端侧走 `console`；不要用 `println!`（release 无控制台，看不到）；
  - 日志要能**独立回答一个是非问题**（"走到了没" "值是多少" "耗时多久"），
    而不是打印一堆无结构的信息；
  - 涉及并发时，把**世代号 / 序号 / 任务 id** 一起打出来，否则分不清是哪一次调用。

- **长期保留的运行日志**是另一回事：像「HTTP 客户端重启」这种能解释"用户可见行为"的日志，
  属于正式日志（不带 `[TEMP]`），应当留下，方便用户贴日志排查。
- 提给用户的配合方式：让用户**复现一次并把日志贴回来**，比来回猜快得多；
  给用户判定表（"如果看到 A 说明是 X，看到 B 说明是 Y"），让一次复现就能定位。


