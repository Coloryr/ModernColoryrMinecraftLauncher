# 待办：下载/安装链路的两个改动

> 本文档记录的是一批**已确认需求但尚未实现**的改动，供后续接手（人或 AI）直接开工。
> 每条都写了：目标、现状与入口、改动点、验收标准。

## 0. 开工前必读（本仓库约定）

- **改任何代码前必须先关掉程序**（`mml-gui.exe` / 占着 1420 的 vite / cargo），改完**不要**替用户启动。
  参考：
  ```powershell
  tasklist /FI "IMAGENAME eq mml-gui.exe"
  netstat -ano | Select-String ":1420.*LISTENING"   # 最后一列是 PID
  taskkill /PID <pid> /T /F
  ```
  其它来路不明的 `node.exe` **不要动**（可能是编辑器/其它工具）。
- **验证只跑最小集**：`npx vue-tsc --noEmit` + `npm run build`（在 `mml-vue/`）；
  动 Rust 时 `cargo check -p mml-gui --all-targets`（在仓库根）。
  **不要**用 `cargo check ... | Select-String "error" | Select-Object -First N` 这类**截断**输出的过滤 ——
  既有 warning 会挤掉真正的 error（本项目已经因此漏过一次编译错误）。
- 命令与 DTO 的唯一来源是 Rust 源码，改了要 `cd mml-gui && npm run gen`（或任意 `cargo check`）重新生成
  `mml-vue/src/lib/bindings.ts`，**不要手改**。
- 临时诊断日志统一带 `[TEMP]` 前缀，定位完立即删除。

---

## T1. 下载窗口：进度按"下载项目"计，而不是文件大小

### 目标

- 一个任务里含**多个**下载项目（例如整合包安装要下几十个文件）时，进度显示
  **已完成项目数 / 总项目数**。
- 只有当该任务**只含 1 个**下载项目时，才回退成现在的**文件大小**进度。

### 现状与入口

- 任务数据：`ResourceStatusDto.tasks` / `ModPackStatusDto.tasks`，
  **一个文件一条**，每条带 `pid`（项目）与 `fid`（文件）以及进度字段 `now`。
- 前端状态聚合在：
  - `mml-vue/src/lib/resourceTasks.ts`
  - `mml-vue/src/lib/modpackTasks.ts`
- 窗口 UI：`mml-vue/src/windows/download/DownloadWindow.vue`
  （标题栏上的两个指示器：`components/DownloadTitleIndicator.vue`、`components/ModpackTitleIndicator.vue`）
- 后端每条任务的进度来源（写着参考）：
  - `mml-gui/src-tauri/src/windows/add_resource.rs` 的 `SourceDownloadInfo { file, name, now, done, failed }`

### 建议改法（纯前端，不动后端）

1. 在任务聚合层按 **`pid`** 把文件分组：`Map<pid, { total, done }>`。
2. 渲染时：
   - 组内 `total > 1` → 主进度 = `done / total`（显示成百分比或 "3/12 个项目"）；
   - 组内 `total === 1` → 用该文件自己的 `now`（现有字节进度）。
3. 需要一个"任务 → 它的文件集合"的关联键。整合包安装的任务用事件里带的实例/任务 id，
   资源的用 `pid` + 目标实例；具体键名以现有 DTO 为准（**先读 DTO，别猜**）。

### 验收

- 下载一个整合包（多文件）：主进度按项目数推进，不再是"总字节的百分比"。
- 单独下载一个模组（单文件）：进度仍是字节/大小。
- 两个窗口的标题栏指示器与下载窗口内的列表口径一致。

---

## T2. 添加实例 → 导入压缩包 → 类型选「整合包」：走"下载整合包 → 安装整合包"路径

### 目标

导入一个**整合包类型**的压缩包时：

- **新建一个实例**（与"下载整合包"完全一致：装完出现新实例）；
- 过程表现为**一个安装任务**（下载/安装进度走整合包那一套，标题栏有整合包进度指示）；
- 不再是"当普通压缩包解开建实例、没有安装任务"。

### 现状与入口（已核对）

| 层 | 位置 | 说明 |
| --- | --- | --- |
| 前端 | `mml-vue/src/windows/add/modes/ArchiveMode.vue` | 类型下拉（`packType`）与文件树；通过事件把 `packType` 交给父组件 |
| 前端 | `mml-vue/src/windows/add/AddInstanceWindow.vue` | 提交入口（`addPackType` / 检测结果 `add_detect_archive`） |
| GUI 命令 | `mml-gui/src-tauri/src/windows/add.rs:400` | `add_import_archive(path, pack_type, …)`；`parse_pack_type` 在 `add.rs:96` |
| GUI 命令 | `mml-gui/src-tauri/src/windows/add.rs:417 / 446` | 分别调 `add_game::install_archive_from_file(...)` / `install_archive_from_url(...)` |
| 参照实现 | `mml-gui/src-tauri/src/windows/add_modpack.rs:216` | `add_modpack_install`：建任务 + 发 `ModPackStatusDto` 进度 |
| 内核 | `mml-core/mml-game/src/add_game.rs` | **整合包分支就在这里**（要改的重点） |

### 建议改法

1. **先只读、不改**：读清 `add_game.rs` 里 `install_archive_from_file` 的整合包分支现在做什么
   （解包建实例？装到哪？实例元信息怎么填？），以及整合包安装 worker 的对外接口与进度回调。
2. 让整合包分支改走 **modpack 安装 worker**（即 `add_modpack_install` 用的那套），
   从而产生 `ModPackTaskDto`、进度事件与"锁定实例/整合包来源"这些信息。
3. 前端：压缩包模式下当类型为整合包时，按钮文案与提示改成"安装整合包"，
   进度接到现有整合包进度的 UI 上。
4. 注意点（易漏）：
   - 实例命名与 `is_modpack` / `modpack_type` 字段要和"下载整合包"那条路径一致；
   - 安装完成后的通知/刷新要走同一套事件，否则主窗口列表不刷新；
   - 本地压缩包没有项目 id，别误用需要 pid 的接口。

### 验收

- 导入一个整合包 zip → 标题栏出现整合包安装进度 → 完成后主窗口出现新实例（信息与在线安装的一致）。
- 导入同为"整合包"但实际是普通包/损坏包 → 有明确失败提示，不留半个实例。
- 导入其它类型（存档/材质包等）行为不变。

---

## 3. 其它已知待办（零散）

- **`[TEMP]` 排序诊断日志待删**：`mml-vue/src/windows/add_modpack/ModpackMode.vue` 与
  `mml-vue/src/windows/add_resource/AddResourceWindow.vue` 里各一条
  `[TEMP] sort 被拒 …`。排序竞态已用"`loadSource` 代次号（`sourceSeq`）"修掉，
  **用户确认不再复现后删掉这两条日志**。
- **两处既有的 Vue 警告（监听泄漏）**：`components/ModpackTitleIndicator.vue:29`、
  `components/ui/WindowControls.vue:65` 报
  `onUnmounted is called when there is no active component instance`
  —— 在 `await` 之后注册卸载钩子，实际没注册上。修法与 `CollectWindow.vue` 里的一样：
  把退订函数存变量、在**同步**的 `onUnmounted` 里调用。
- **`cargo check` 的既有警告**：`mml-gui/src-tauri/src/windows/mod.rs:394` `unused variable: app`，
  顺手可清。
- **下拉框弹层口径**（可选）：目前 `.field-select` / `.head-select` 走 Chromium 135+ 的
  base-select 页内弹层，而 `.filter-select` / `.inst-select` / `.dp-select` 仍是系统原生弹层
  （关闭态已统一：同箭头 `--select-arrow`、不透明底、主题描边）。
  要全仓一致就把后三个也改成"挂 `.field-select` + 只覆盖尺寸"的写法。
- **收藏窗口的"下载"仍可用**：卡片上的下载**按钮**已按需求删除，
  但**点卡片**仍会打开下载窗口并跳到该项目详情（构造精简条目 + 现取 `collect_project_item`）。
  若要一并去掉，改 `CollectWindow.vue` 的 `toggleCard`；随后
  `windowParams.project` 那套透传、两个下载窗口的 `consumeTargetProject`、
  后端 `collect_project_item` 命令都会变成无引用，需要一起清理。
- **未使用的 i18n 键**：`collect.download`（下载按钮与右键项都删了）。可删。

---

## 4. 当前工作区状态（交接时的基线）

- 程序**未运行**（`mml-gui.exe` 无、1420 未监听）。
- 最后一次验证：`cargo check -p mml-gui --all-targets` 无 error；
  `npx vue-tsc --noEmit` 退出码 0；`npm run build` ✓。
- 本轮会话改动的重点文件（便于快速定位）：
  `mml-vue/src/windows/collect/CollectWindow.vue`、
  `mml-vue/src/windows/add_resource/AddResourceWindow.vue`、
  `mml-vue/src/windows/add_modpack/ModpackMode.vue`、
  `mml-vue/src/windows/windowManager.ts`、
  `mml-gui/src-tauri/src/windows/collect.rs`、
  `mml-gui/src-tauri/src/image_manager.rs`、
  `mml-vue/src/windows/main/sidebar/MainSidebar.vue`（加载器角标配色）。
