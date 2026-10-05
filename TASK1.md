# 待办：首次启动时从 ColorMC 迁移

> 临时任务文件（见 AGENTS.md §6.1）：做完即删，不作为仓库文档长期保留。
> 已确认的前提见下面「调研结论」，不要再重新推测。

## 目标

**首次启动**时检测到本机有 ColorMC 的工作目录，就弹窗询问用户要不要把数据搬过来，
给三个选项：

| 选项 | 行为 |
| --- | --- |
| **复制** | 把 ColorMC 工作目录里的数据复制到 M²L 运行目录（ColorMC 那份**保留**） |
| **移动** | 同上，但搬完**删除**源目录（或至少在成功后删；失败要能回滚） |
| **不迁移** | 什么都不做，并且**记下这次选择**，以后不再问 |

只在**第一次启动**出现：问过一次（任何一种选择）之后就不再弹。

## 调研结论（已核实，可直接用）

### ColorMC 的工作目录怎么找

来源：`E:\code\ColorMC\src\ColorMC.Launcher\Program.cs` 的 `Main()`（第 84–114 行）。
优先级从高到低：

1. **`%LOCALAPPDATA%\ColorMC\run`** 里记的路径（文件内容就是一个目录路径）。
   仅当该路径**确实存在**时才采用。
2. 否则按平台取默认值：
   - Linux：`~/.ColorMC/`
   - macOS：`/Users/shared/ColorMC/`
   - **Windows：`<exe 所在目录>\colormc\`**（`AppContext.BaseDirectory + "colormc"`）
3. 若上面这个目录**不可写**（它自己会建个 `test` 文件试写再删），
   ColorMC 会退到 **`%APPDATA%\ColorMC\`**。

> 注意：这条探测链要**按顺序**试，第一个"存在且像 ColorMC 目录"的才算命中。
> 只判断目录存在还不够 —— 见下面的「怎么判定真的是 ColorMC 目录」。

### 怎么判定"真的是 ColorMC 目录"

不要只看目录存在（`%LOCALAPPDATA%\ColorMC\` 里可能只有 `run` 文件或零散文件）。
建议按**特征文件**判定，命中其一即认为有数据可迁移：

- `game.json` / `instances/`（实例与分组数据）
- `config.json`（启动器配置）
- `auth.json`（账户）
- `block.json`、`favorites.json`、`collect.json` 之类

本机实测：`%LOCALAPPDATA%\ColorMC\` 下**只有 `auth.json` + `block.json`，没有 `run` 文件**，
也没有 `colormc/` 子目录 —— 所以"目录存在但没 run"是**真实会遇到**的情况，判定要按特征文件来。

### 与 M²L 的对应关系（迁移要搬什么）

M²L 运行目录 = `mml-gui/src-tauri/src/main.rs` 的 `get_run_path()`（发布版读
`%LOCALAPPDATA%\mml\run`，取不到用 exe 同目录的 `mml/`）。

| ColorMC | M²L | 说明 |
| --- | --- | --- |
| `instances/<实例名>/` | 同名目录 | 实例目录结构本来就互通（`guisetting.json` 是照 ColorMC 的结构抄的） |
| `game.json` / 分组数据 | 同名 | 分组表 |
| `config.json` | `config.json` | 内核配置（结构可能有差异，**不能整份覆盖**，见下） |
| `auth.json` | — | 账户：ColorMC 的格式与 M²L 不同，**先只搬文件不解析**，由用户自己重登或后续做转换 |

> **重点**：`config.json`、`auth.json` 这些**不能整份盖过去** —— M²L 的字段是
> ColorMC 的子集/超集，直接覆盖会丢字段或让 M²L 读不了。要么逐字段合并，
> 要么**只搬实例目录**（最有价值的那部分），配置让用户重新设。
> 这一条要在实现时定，别默认"整目录复制"。

## 实现要点

### 1. "第一次启动"的状态存哪

不要把标记塞进 `gui_config.json`（那是"界面设置"，语义不符，而且用户可能删它来重置）。
建议在**运行目录**放一个独立标记（例如 `mml-migrate-checked`），理由是：
- 它表达的是"这台机器上的这个运行目录已经问过了"；
- 删掉它就能重新触发询问，方便调试。

### 2. 后端：探测 + 迁移都是后端活

判据符合 AGENTS.md §12「前端拿不到的能力」：**读别的目录、复制/删除文件、拿本机路径**
—— 必须写 `#[tauri::command]`，不要试图在前端做。

建议的命令（名称照 §4「窗口名_方法名」）：

- `window_check_colormc() -> Option<String>`：返回探测到的 ColorMC 工作目录（没有则 `None`）；
- `window_migrate_colormc(source: String, move: bool) -> Result<...>`：执行复制/移动；
- `window_skip_colormc()`：记下"不再询问"。

进度：迁移可能搬很多文件（实例目录很大），照 **`gui_hook` 的 `IProgressGui`** 上报
（与模组扫描、方块渲染同一套），前端显示 `x/x`。

### 3. 前端：在哪里弹

主窗口启动时（`MainWindow.vue` 的挂载流程，或 `App.vue`）先问后端有没有 ColorMC 目录，
有且没问过就弹 `BaseModal`。三个按钮 + 一段说明（来源目录、会搬什么、移动会删除源）。

**弹窗要能关掉**（用户可能还没决定）——但关掉算不算"问过"要想清楚：
建议**只有三个按钮才算回答**，直接关掉下次还问。

## 验收

- 首次启动（无标记文件）且本机有 ColorMC 目录 → 弹窗；
- 选复制：M²L 运行目录里出现实例/数据，**ColorMC 目录原样还在**；
- 选移动：数据搬过去，源目录被删（或在失败时保留并报错）；
- 选不迁移：不再弹，且**重启也不弹**；
- 没有 ColorMC 目录时**不弹**、不报错；
- 迁移后重新启动，M²L 能正常看到迁移过来的实例。
- 验证照最小集：动 Rust 跑 `cargo check -p mml-gui --all-targets`；
  前端 `npx vue-tsc --noEmit` + `npm run build`。

## 待定（实现前要定的）

1. **搬哪些内容**：只搬 `instances/`，还是连配置一起？（见上面"不能整份覆盖"的提醒）
2. **移动时删不删源目录**：删整个目录，还是只删搬走的那些？
3. **账户 `auth.json`**：ColorMC 格式与 M²L 不同，是否要转换（还是只搬文件 / 干脆不搬）？
4. **标记文件叫什么、放哪**（见上）。