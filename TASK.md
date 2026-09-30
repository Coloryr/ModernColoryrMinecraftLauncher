# TASK：自定义主页面（服主导入 zip 替换启动器主页）

## 背景与目标

服主（整合包作者）想让启动器主页面变成自己做的页面（服务器公告、群号、赞助名单、自定义启动按钮）。
当前启动器主页是写死的 Vue 组件 `HomePage.vue`，服主改不了。

目标：在**设置窗口 → 客户端设置**新增一组「自定义主页面」，服主选一个 zip 导入后，
主窗口的启动器主页替换成 zip 里的 `index.html`；这个自定义页面还能直接调用启动器的
`tauri::command`、订阅启动器事件。

## 已确认的产品决策

| 决策点 | 结论 |
| --- | --- |
| 页面形态 | **整页 HTML**：zip 里是 `index.html` + CSS/JS/图片，启动器用 iframe 加载 |
| 页面能力 | 能调用启动器命令，**放行全部已注册命令**（等价于页面拿到启动器全部 IPC 权限） |
| 事件订阅 | 支持，**小范围白名单**（启动进度等） |
| 替换范围 | **只替换启动器主页**（顶栏「主页」按钮切过去的那一页）。实例列表、启动按钮、设置都保留 |
| 后端范围 | Rust 实现也写（解包、scheme 服务文件、配置读写全部做完，要能直接跑通） |

## 已核实的现状（实现时直接复用，不要另起炉灶）

- **自定义 scheme 范式已存在**：`mml-gui/src-tauri/src/lib.rs:32-36` 注册 `mml-image`，
  handler 在 `image_manager.rs:1036` 的 `url_image`，按路径首段分发（`split_path` 在 `image_manager.rs:105-107`），
  `send_icon`（`image_manager.rs:181-191`）演示了「自定义 status + mime + body」的写法。
  Windows 上 scheme 映射成 `http://mml-image.localhost`（`image_manager.rs:1071-1077` 的 `image_base_url()`），
  前端通过命令 `main_image_base_url` 拿这个前缀（`api.ts:631-633`）。
- **CSP 与权限**：`mml-gui/src-tauri/tauri.conf.json` 里 `app.security.csp` 是 `null`，也没有 `assetProtocol`。
  `capabilities/default.json` 只有一个文件、窗口匹配 `["main", "mml-*"]`。
  **自定义 scheme 不受 capability 管辖，本次不需要改 tauri.conf.json 或 capabilities。**
- **zip 工具在 `mml-core/mml-base/src/archives/`**：`ArchiveType::try_from_path`（`mod.rs:78`）、
  自由函数 `decompress`（`mod.rs:296`）、`BaseArchive::open` / `extract_all(output_dir, unselect, strip_dir, gui)`（`base_archive.rs:577`）。
  最贴近的范例是 `mml-jvms/src/lib.rs:461` 的 `unzip_java`。
- **「选文件 → 导入 → 内联进度条」的现成范例**（照抄对象）：
  `windows/settings.rs:451-476` 的 `settings_import_java`、进度桥 `JavaArchiveGui`（`settings.rs:400-447`）、
  事件 `settings-java-progress`、前端 `SettingsWindow.vue:559-579`（选文件）+ `:1308-1315`（进度条模板）+ `:2131-2151`（样式）。
- **运行根目录**：`mml_base::get_base_dir()`（`mml-core/mml-base/src/lib.rs:56`），与 `gui_config.json` 同级。
- **配置新增字段的四处落点**：`gui_config.rs` 的 `ClientConfig` + `Default`；`dtos/gui_config_dto.rs` 的
  `ClientConfigDto`；两个 `From` impl；前端 `guiConfig.ts` 的接口 + `defaultConfig()`。
  保存后的 `client_changed` 比对是**整结构体 `!=`**（`windows/mod.rs:803-806`），
  新字段只要 `PartialEq` 就自动纳入，**不用改那里**。
- **主页渲染点只有三个**，全在 `windows/main/MainWindow.vue`：
  空实例分支（`:1440-1458`，无条件显示）、列表模式分支（`:1462-1547`，`v-if="newsActive"`）、
  分组/平铺兜底分支（`:1594`）。`HomePage` 全仓只在这里被引用。

## ipc-gen 硬性约束（写新命令必须遵守，否则静默不注册）

来自 `mml-gui/ipc-gen/` 的字符串扫描逻辑（非 AST）：

1. 标记必须是精确字面量 `#[tauri::command]` / `#[gui_macros::emit]`，不能带括号参数。
2. 标记与 `fn ` 之间只能有 `///` 文档和 `#[...]` 属性。
3. 参数必须是平铺的 `name: Type`，不能用解构、元组模式、`self`。
4. Tauri 注入参数必须写裸类型名（`AppHandle` / `WebviewWindow` / `State`），写 `tauri::AppHandle` 会漏进 TS 签名。
5. 返回类型用 `Result<T, String>`；Error 类型不进 TS。
6. 命令名 `窗口名_方法名`，全局唯一（重名会让 `generate_handler!` 编译失败）。
7. 命令函数必须 `pub`；新 DTO 必须放 `src/dtos/`，字段**一行一个、类型不跨行**，
   derive 与 `struct` 之间**不能有空行**，结构体本身要 `pub struct` 且顶格新起一行。
8. 新增命令/DTO 后跑 `cd mml-gui && npm run gen` 重新生成 `bindings.ts` / `listens.ts`，**勿手改这两个文件**。

---

## 一、数据与存储

**磁盘配置**：`ClientConfig` 只新增一个字段。

- `custom_home: bool`，默认 `false` —— 是否启用自定义主页面。

**不引入版本号字段**：iframe 破缓存用的版本号取 `custom_home` 目录的 mtime，
由 `custom_home_status` 现算。这样配置里没有第二个需要同步的状态。

**解包目录**：`mml_base::get_base_dir()/custom_home/`。

**导入语义 = 全量替换**，且必须原子，不能出现「导入失败留下半个包」：

1. 解到 `custom_home.tmp/`（先删掉可能残留的旧 tmp）。
2. 定位入口：`tmp/index.html` 存在就用它；否则若 `tmp/` 下**恰好一个子目录**且其中
   有 `index.html`，把那个子目录当作根（把内容上提一层，或直接记下这个相对前缀）。
3. 找不到 `index.html` → 报错 `err.customHomeNoIndex`，删掉 tmp，**现有包保持不变**。
4. 校验通过：删旧的 `custom_home/`，`rename(tmp, custom_home)`。

**入口定位结果不要只存在于内存**：`custom_home_status` 每次都要能重新算出来，
所以第 2 步若是「子目录当根」，请把上提动作做成真的移动文件，而不是记一个前缀，
否则删掉旧目录后前缀信息就没了。

## 二、Rust 侧

### 2.1 新模块 `mml-gui/src-tauri/src/windows/custom_home.rs`

并在 `windows/mod.rs` 里加 `pub mod custom_home;`。

命令（组键会变成 `customHome`，前端是 `commands.customHome.import` 等）：

| 命令 | 签名 | 说明 |
| --- | --- | --- |
| `custom_home_import` | `(app: AppHandle, path: String) -> Result<CustomHomeInfoDto, String>` | 解包 + 原子替换，返回新的状态 |
| `custom_home_status` | `() -> Result<CustomHomeInfoDto, String>` | 查询已导入情况与入口 URL |
| `custom_home_remove` | `() -> Result<(), String>` | 删除 `custom_home/` 目录 |
| `custom_home_open_dir` | `() -> Result<(), String>` | 在资源管理器里打开该目录，方便服主调试 |

`custom_home_import` 是重活，照 `settings_import_java`（`settings.rs:451-476`）写成
`pub async fn` + `tauri::async_runtime::spawn_blocking`。

DTO 放新文件 `mml-gui/src-tauri/src/dtos/custom_home_dto.rs`，并在 `dtos/mod.rs` 登记模块与重导出：

```rust
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomHomeInfoDto {
    /// 是否已导入（目录存在且入口页存在）
    pub installed: bool,
    /// 配置文件里的启用开关
    pub enabled: bool,
    /// 入口页完整 URL（未导入时为空串），已带破缓存参数
    pub entry_url: String,
    /// 内容版本号（取 custom_home 目录 mtime），用于 iframe 破缓存
    pub version: String,
    /// 解包出的文件数
    pub file_count: u32,
}
```

`entry_url` 由 Rust 拼好（`{scheme_base}/index.html?v={version}`），
**不要在 TS 里重算 scheme 前缀** —— 参考 `image_base_url()` 在 `image_manager.rs:1071` 的写法，
Windows 是 `http://mml-home.localhost`、其余平台 `mml-home://localhost`。

进度事件（照 `settings.rs:389-393` 的 `emit_settings_java_progress`）：

```rust
#[gui_macros::emit]
pub fn emit_custom_home_progress(app: &AppHandle, data: CustomHomeProgressDto) { ... }
```

事件名 `custom-home-progress`，DTO `CustomHomeProgressDto { state: String, now: u32, total: u32, sub_text: String }`
（字段名/类型照 `JavaImportProgressDto` 的既有形状，前端进度条逻辑可直接复用）。
进度桥实现 `mml_base::archives::IBaseArchiveGui`，照 `JavaArchiveGui`（`settings.rs:400-447`）。

### 2.2 自定义 scheme `mml-home`

在 `mml-gui/src-tauri/src/lib.rs` 第 32-36 行那处旁边再注册一个
`register_asynchronous_uri_scheme_protocol("mml-home", ...)`，handler 放在
`windows/custom_home.rs` 里（例如 `pub async fn url_custom_home(req, res)`）。

行为要求：

- **根目录**：`get_base_dir()/custom_home`。
- **路径映射**：path 为 `/` 或空 → `index.html`；否则去掉前导 `/` 后拼到根目录下。
- **忽略 query string**（`?v=...` 只是破缓存用）与 fragment。
- **路径穿越必须挡死**：拒绝任何 `..` 段、反斜杠、`:`、以及 URL 解码后再出现的这些；
  拼出的绝对路径规范化后必须仍以根目录为前缀。越界 → 404。
  这条要有针对性验证（见验收 5）。
- **MIME 按扩展名**：html/css/js/mjs/json/svg/png/jpg/jpeg/gif/webp/ico/woff/woff2/ttf/otf/txt。
  未知扩展名回 `application/octet-stream`。
- **HTML 注入桥接脚本**：返回 `.html` 时，在 `</head>` 之前插入
  `<script src="/__mml_bridge.js"></script>`；没有 `</head>` 就插在开头。
  仅当响应体是合法 UTF-8 时注入，否则原样返回（页面仍能显示，只是没有桥接）。
- **`/__mml_bridge.js`** 是内置资源，不进 `custom_home/`，直接返回桥接脚本，
  `Content-Type: application/javascript`。
- 未导入 / 文件不存在 → 404。响应带 `Access-Control-Allow-Origin: *`（照 `send_png` 的写法）。

桥接脚本本身作为独立文件 `mml-gui/src-tauri/src/windows/custom_home_bridge.js`，
用 `include_str!("custom_home_bridge.js")` 引入（不要塞成 Rust 字符串字面量，难维护）。

## 三、前端

### 3.1 桥接协议

`custom_home_bridge.js` 在 iframe 内暴露：

```js
window.mml = {
  invoke(cmd, args) -> Promise<any>,   // 调任意已注册命令
  on(event, cb) -> () => void,         // 订阅白名单事件，返回取消订阅函数
  ready: Promise,                      // 父窗口握手完成
}
```

消息格式（全部经 `postMessage`，targetOrigin `"*"`，iframe 是 opaque origin）：

- iframe → 父：`{ __mml: 1, id, type: "invoke", cmd, args }`
- iframe → 父：`{ __mml: 1, id, type: "subscribe", event }`
- iframe → 父：`{ __mml: 1, type: "unsubscribe", id }`
- 父 → iframe 应答：`{ __mml: 1, id, ok: true, data }` 或 `{ __mml: 1, id, ok: false, error }`
- 父 → iframe 事件：`{ __mml: 1, type: "event", id, event, payload }`

**父窗口侧必须校验** `event.source === iframeEl.contentWindow` 且 `event.data?.__mml === 1`，
否则忽略。

### 3.2 新文件 `mml-vue/src/components/CustomHomePage.vue`

- prop：`entryUrl: string`。
- 渲染 `<iframe :src="entryUrl" sandbox="allow-scripts allow-forms allow-popups allow-modals">`。
- **故意不给 `allow-same-origin`**：iframe 的 origin 保持 opaque，页面拿不到启动器页面的
  localStorage / cookie。`postMessage` 在 opaque origin 下照常可用。
- iframe 铺满内容区（`width/height: 100%; border: 0`）。
- 挂载时监听 `window` 的 `message`；卸载时移除监听并清理所有事件订阅。

### 3.3 新文件 `mml-vue/src/lib/customHomeBridge.ts`

父窗口侧的消息处理与白名单，集中放这里：

- 处理 `invoke`：直接 `import { invoke } from "@tauri-apps/api/core"` 后
  `await invoke(cmd, args)`。**放行全部命令**，不做白名单过滤——
  Tauri 对未注册命令会自行报错，原样把错误字符串回给页面即可。
- 处理 `subscribe` / `unsubscribe`：查下面这张表，用 `@tauri-apps/api/event` 的 `listen`
  （不要走 `api.ts` 那些带类型转换的包装，桥接要转发原始 payload）。
- 事件白名单（一处，要扩就改这里）：

  | 事件名 | 常量（`listens.ts`） | payload |
  | --- | --- | --- |
  | `launch-state` | `LaunchState` | `StateEvent` |
  | `launch-error` | `LaunchError` | `ErrorEvent` |
  | `game-exit` | `GameExit` | `ExitEvent` |
  | `game-log` | `GameLog` | `LogEvent` |
  | `instance-change` | `InstanceChange` | `{ type: string }` |

  `game-log` 是流式的、量大，实现时留意节流；如果实测刷屏就把它从白名单里去掉并在本文档里说明。
- 同一个事件被多个 id 订阅时只 `listen` 一次、多路分发。

### 3.4 设置窗口 `mml-vue/src/windows/settings/SettingsWindow.vue`

在「客户端设置」页（`v-else-if="tab === 'client'"`，`:1374` 起）的 `group-title` 序列里
新增一组「自定义主页面」：

- **开关**：启用自定义主页面 → `client.customHome`，`@update:model-value` 走 `applyClient()`
  （与 `motdCard` 等既有开关一致，`:221-223`）。
- **导入按钮**：`@tauri-apps/plugin-dialog` 的 `open()`，`filters: [{ name: "Zip", extensions: ["zip"] }]`
  —— 照 `importJavaArchive`（`:559-579`）写。
- **打开目录 / 删除** 两个按钮，分别调 `commands.customHome.openDir()` / `.remove()`。
- **状态行**：未导入 / 已导入（文件数）/ 启用中。
- **内联进度条**：照 `importJavaArchive` 那套（模板 `:1308-1315`、样式 `:2131-2151`），
  订阅新事件 `onCustomHomeProgress`。
- 进页面刷新：复用既有模式，在 `watch(tab, ...)`（`:350-352`）里像 `refreshLockInstances()`
  那样补一个 `refreshCustomHome()`。

### 3.5 主窗口 `mml-vue/src/windows/main/MainWindow.vue`

- 新增 `customHome` 状态：从 `custom_home_status` 拿 `CustomHomeInfoDto` 存进一个 ref，
  在 `doInit()`（`:1174-1187`）里拉一次；导入/删除后由设置窗口发事件或主窗口重新拉取
  （选后者更简单：监听 `client-config-change` 时顺带重拉一次状态）。
- `useCustomHome = computed(() => info.enabled && info.installed && !!info.entryUrl)`
  —— **降级策略**：启用了但没导入（或目录被手动删了）时回落到内置 `HomePage`，不要白屏。
- 在**三处** `HomePage` 渲染点各加一个分支：
  `<CustomHomePage v-if="useCustomHome" :entry-url="info.entryUrl" />` 否则原来的 `<HomePage ... />`。
  三处是 `:1440-1458`（空实例）、`:1462-1547`（列表模式）、`:1594`（分组/平铺兜底），
  以及列表模式里 `newsActive` 的 `Transition` 两个 key 节点要保持进出动画正常。
- 顶栏「主页」按钮与 `newsActive` 逻辑**不变**：自定义主页只是替换了那一页的内容。

### 3.6 其余前端改动

- `mml-vue/src/lib/guiConfig.ts`：`ClientConfig` 接口（`:81-98`）与 `defaultConfig()`
  的 `client`（`:240-249`）加 `customHome: boolean`。
- `mml-vue/src/lib/api.ts`：照 `:497-533` 的写法加 `onCustomHomeProgress`。
- i18n `zh-CN.ts` / `en-US.ts`：新增 `winSettings.customHome*` 一组
  （分组标题、描述、启用开关、导入、打开目录、删除、未导入、已导入、导入中、导入失败、
  以及错误键 `err.customHomeNoIndex` 对应的提示）。

## 四、安全边界（实现时不要擅自收紧或放宽）

- 自定义页面通过 `invoke` 能调到**任何**已注册命令，包括删实例、改配置、装整合包。
  这与「服主给的整合包本身就是代码」的信任级别一致，**是本次确认过的设计**。
- 页面拿不到启动器页面的 localStorage / cookie（iframe 无 `allow-same-origin`）。
- scheme handler 的路径穿越防护是本次唯一的强制安全项，必须做且必须验证。
- 不做 zip 内容审计、不限制文件类型与大小。**已知取舍**：zip 里可以塞任意文件，
  但只能通过 `mml-home` scheme 读出来，不会自动执行。

## 五、涉及文件清单

**Rust（新增）**
- `mml-gui/src-tauri/src/windows/custom_home.rs`
- `mml-gui/src-tauri/src/windows/custom_home_bridge.js`
- `mml-gui/src-tauri/src/dtos/custom_home_dto.rs`

**Rust（修改）**
- `mml-gui/src-tauri/src/gui_config.rs`（`ClientConfig` 字段 + `Default`）
- `mml-gui/src-tauri/src/dtos/gui_config_dto.rs`（DTO 字段 + 两个 `From`）
- `mml-gui/src-tauri/src/dtos/mod.rs`（登记新 DTO 模块）
- `mml-gui/src-tauri/src/windows/mod.rs`（`pub mod custom_home;`）
- `mml-gui/src-tauri/src/lib.rs`（注册 `mml-home` scheme）

**前端（新增）**
- `mml-vue/src/components/CustomHomePage.vue`
- `mml-vue/src/lib/customHomeBridge.ts`

**前端（修改）**
- `mml-vue/src/lib/guiConfig.ts`
- `mml-vue/src/lib/api.ts`
- `mml-vue/src/windows/settings/SettingsWindow.vue`
- `mml-vue/src/windows/main/MainWindow.vue`
- `mml-vue/src/lib/i18n/locales/zh-CN.ts`、`en-US.ts`

**生成物（勿手改）**：`mml-vue/src/lib/bindings.ts`、`mml-vue/src/lib/listens.ts` —— 由 `npm run gen` 产出。

## 六、验收标准

### 前置（项目约定）

- 改任何代码前先停掉 `tauri dev` 与 `mml-gui.exe`（`tasklist` / `netstat -ano | grep :1420`），
  改完**不要**重启，由用户自己起。
- 临时验证代码（假数据、自动开窗、调试输出）必须标 `TEMP` 注释，验证完立即还原。
- 临时产物放根目录 `target/temp`，不要写进版本控制。

### 自动化

1. `cd mml-gui && npm run gen` —— `bindings.ts` / `listens.ts` 出现新命令与 `custom-home-progress`。
2. `cargo check -p mml-gui` —— 只跑这个 crate，不要全量 workspace。
3. `npm --prefix mml-vue run build` —— `vue-tsc --noEmit` + vite。

### 手工（用户跑）

1. **正常导入**：造一个 zip（`index.html` + `style.css` + 一张图），
   设置 → 客户端设置 → 导入 → 主窗口主页应变成该页面，样式与图片正常。
2. **能调命令**：页面里放一个按钮，`mml.invoke("main_get_instances")` 把实例名渲染出来，
   再放一个「启动游戏」按钮调 `main_launch_game`，点了能真启动。
3. **能订阅事件**：`mml.on("launch-state", ...)` 能在页面上显示启动阶段文本。
4. **降级**：关掉「启用自定义主页面」→ 回落到内置主页；
   手动删掉 `custom_home/` 目录 → 也回落，不白屏。
5. **路径穿越**：zip 里放一个 `evil.html` 引用 `../../../gui_config.json`，
   打开该页应看到 404 而不是配置文件内容。
6. **失败不破坏现有包**：先正常导入一版，再导入一个不含 `index.html` 的 zip，
   应报错且**上一次的页面仍然正常**（旧的 `custom_home/` 没被动过）。
7. **删除**：点删除 → 目录消失、状态回到未导入。

## 七、明确不做

- 不改 `tauri.conf.json` 与 `capabilities/`（不需要）。
- 不给自定义页面做 zip 内容审计、签名校验、文件类型白名单。
- 不做在线获取自定义主页（只支持本地 zip）。
- 不替换实例列表 / 启动按钮 / 设置等其它区域，只替换启动器主页那一页。
- 不改 `windows/mod.rs:803-806` 的 `client_changed` 比对逻辑（整结构体 `!=` 已覆盖新字段）。

---

## 八、后续任务：`mml-ui-demo`（Vue 自定义主页示例工程）

> 本节是**另一个任务**，排在「自定义主页面」之后做。

**目标**：给服主一个「用 Vue 写自定义主页面」的现成样板 —— 仓库根新建 `mml-ui-demo/`，
它是一个**独立**的 Vue3 + Vite 工程；`npm run build` 产出静态站点，再打包成 zip，
直接在「设置 → 客户端设置 → 自定义主页面 → 导入」里导入，就能当启动器主页跑。

**为什么单独立一个工程**：`mml-vue/` 是启动器自己的前端（依赖 IPC 包装、主题、窗口管理
等启动器上下文），不能拿来当自定义主页；示例工程必须能脱离启动器构建链独立构建，
产物只有静态文件。

**要求（实现时遵守）**

- 目录：仓库根 `mml-ui-demo/`。**不**加入根 `Cargo.toml` 的 workspace members，
  也**不**挂进 `mml-vue` 的构建流程；`node_modules` 独立。
- 技术栈：`vue` + `vite`（`vue-tsc` 可选），不 import `mml-vue/src/` 下的任何代码。
- 构建产物：`npm run build` → `dist/`，且 `index.html` 必须在 `dist/` **根目录**
  （自定义主页面的入口定位规则：优先包内根 `index.html`；否则整包只有一个顶层目录时，
  取该目录里的 `index.html`）。
- 打包：构建同时产出**一个 zip**。zip 内根目录直接是 `index.html`
  （不要把 `dist/` 这层目录一起打进去）。打包手段实现时定：Node 脚本调
  PowerShell `Compress-Archive`（零依赖）或加一个 `archiver` 之类的依赖。
- 页面能力：只用 Rust 注入的 `window.mml`（`invoke` / `on` / `ready`，协议见本文件
  「三、桥接协议」与 `mml-gui/src-tauri/src/windows/custom_home_bridge.js`）。示例至少演示：
  - `mml.invoke("main_get_instances")` → 渲染实例列表；
  - `mml.invoke("main_launch_game", { uuid })` → 启动游戏；
  - `mml.on("launch-state", ...)` → 显示启动阶段文本。
- 类型提示：`window.mml` 的 `.d.ts` 放在示例工程内自己维护，**不要**动启动器仓库里的类型；
  自定义页面的 `invoke` 是「原始命令名 + 原始 camelCase 参数」，没有 `bindings.ts` 那层包装。
- 仓库卫生：`mml-ui-demo/.gitignore` 至少忽略 `node_modules/`、`dist/`、`*.zip`。
- 验收：`npm run build` 产出 `dist/index.html` 与一个 zip；该 zip 导入启动器后主窗口能正常
  显示该页面、能列出实例、能启动游戏、能显示启动阶段。

**要在文档里对服主说明**：自定义页面能调用启动器的**全部**已注册命令（等价于拿到启动器
全部 IPC 权限），这是「自定义主页面」的既定设计（见本文件「四、安全边界」）。

---

## 九、实现进度（本文件最后一次更新的状态）

> 上一次会话中途停止，**源码改动留在工作区**（没有还原）。本节记录做到哪、差什么，
> 以及与原规格的偏差，方便直接接着做。

### 前置约定执行情况

- 动手前确认过没有 `tauri dev` / `mml-gui.exe` 在跑（端口 1420 空闲），改完**没有**重启，
  启动交给用户。

### 已完成（已通过验证）

**Rust**——`cd mml-gui && npm run gen`（= 对 `src-tauri` 清单的 `cargo check`）退出码 0：

| 文件 | 改动 |
| --- | --- |
| `mml-gui/src-tauri/src/gui_config.rs` | `ClientConfig` 新增 `custom_home: bool`（默认 `false`） |
| `mml-gui/src-tauri/src/dtos/gui_config_dto.rs` | `ClientConfigDto` 新增 `customHome` + 两个 `From` 同步 |
| `mml-gui/src-tauri/src/dtos/custom_home_dto.rs` | **新增**：`CustomHomeInfoDto` / `CustomHomeProgressDto` |
| `mml-gui/src-tauri/src/dtos/mod.rs` | 登记新 DTO 模块与重导出 |
| `mml-gui/src-tauri/src/windows/custom_home.rs` | **新增**：4 条命令 + 2 个事件 + `mml-home` handler（含路径穿越防护、MIME 表、HTML 注入、`percent_decode`） |
| `mml-gui/src-tauri/src/windows/custom_home_bridge.js` | **新增**：iframe 侧桥接脚本（`window.mml`），`include_str!` 引入 |
| `mml-gui/src-tauri/src/windows/mod.rs` | `pub mod custom_home;` |
| `mml-gui/src-tauri/src/lib.rs` | 注册 `mml-home` 异步 scheme |

生成物（由 `npm run gen` 产出，勿手改）：`bindings.ts` 里出现
`commands.customHome.{import,status,remove,openDir}` 与
`CustomHomeInfoDto` / `CustomHomeProgressDto`；`listens.ts` 里出现
`CustomHomeProgress` / `CustomHomeChange`。

**前端**——`npm --prefix mml-vue run build`（`vue-tsc --noEmit` + vite）退出码 0：

| 文件 | 改动 |
| --- | --- |
| `mml-vue/src/lib/customHomeBridge.ts` | **新增**：父窗口侧桥接（来源校验、白名单、`invoke` 转发、事件攒批 50ms） |
| `mml-vue/src/components/CustomHomePage.vue` | **新增**：`sandbox="allow-scripts allow-forms allow-popups allow-modals"` 的 iframe，挂载时接桥接 |
| `mml-vue/src/lib/guiConfig.ts` | `ClientConfig.customHome` + `defaultConfig()` 的 client |
| `mml-vue/src/lib/api.ts` | `onCustomHomeProgress` / `onCustomHomeChange` |
| `mml-vue/src/lib/i18n/locales/zh-CN.ts`、`en-US.ts` | `winSettings.secCustomHome` 等一组键 + `err.customHomeNoIndex` |
| `mml-vue/src/windows/settings/SettingsWindow.vue` | 客户端设置页新增「自定义主页面」分组（开关 / 导入 / 打开目录 / 删除 / 状态行 / 内联进度条），进页面 `refreshCustomHome()` |

### 未完成（恢复时从这里接着做）

- **`mml-vue/src/windows/main/MainWindow.vue` 一行都没改** —— 这是整个功能还差的关键一步：
  - 新增 `customHome` ref（`custom_home_status` 结果）与
    `useCustomHome = computed(() => info.enabled && info.installed && !!info.entryUrl)`；
  - `doInit()` 里拉一次；`onClientConfig()`（`client-config-change`）里顺带重拉一次；
    再订阅 `custom-home-change` 重拉（见下面的偏差 1）；
  - **三处** `HomePage` 渲染点各加分支（行号以当前文件为准，见「已核实的现状」里的描述，
    现文件已比原规格记录的行号更靠下）：空实例分支、列表模式 `Transition` 的 `key="home"`
    节点、分组/平铺兜底分支。两个 `Transition` 的 `key="home"` 节点要保证进出动画正常。
- 手工验收 1~7 一条都还没跑（需要用户自己起 `cd mml-gui && npm run tauri dev`）。
  **当前状态：导入 zip 后主窗口仍显示内置主页**（不会白屏，只是自定义页面不生效）。
- 本节「八」的 `mml-ui-demo` 尚未开始（目录也还没建）。

### 与前面章节的偏差（需要确认是否接受）

1. **多了一个事件 `custom-home-change`**（`emit_custom_home_change`）。原规格只定义了
   `custom-home-progress`，并假设主窗口监听 `client-config-change` 就能跟上；但导入 / 删除
   都**不改** `gui_config.client`，`client-config-change` 不会发，主窗口无从知道要重拉状态
   （验收 1、7 会失效）。所以导入成功后与删除后各广播一次 `custom-home-change`。
2. `custom_home_remove` 的签名多了一个注入参数 `app: AppHandle`（TS 侧会被过滤掉，
   前端 API 仍是 `remove()`）—— 就是为了发上面那个事件。
3. `CustomHomeProgressDto.sub_text` 用的是 `String`（按 §2.1 的字面定义）；
   `JavaImportProgressDto` 是 `Option<String>`。空串表示没有当前文件名。
4. `custom_home_open_dir` 在目录不存在时会先 `create_dir_all` 再打开（按钮因此始终可用）。
5. 细节：注入用的 `</head>` 匹配是**大小写不敏感**的；MIME 表额外收了 `htm`；
   百分号解码对非法转义原样保留（`100%.png` 这类文件名仍可用），解码后的 `..` / `\` / `:`
   照旧拒绝。

### 卫生

- 没有 `TEMP` 调试代码，`target/temp` 下没有新增中间产物。
- 上述源码改动都在工作区里，未提交。

