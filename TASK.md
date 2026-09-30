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

桥接脚本本身作为独立文件 `mml-gui/src-tauri/resources/custom_home_bridge.js`，
用 `include_str!("../../resources/custom_home_bridge.js")` 引入（不要塞成 Rust 字符串字面量，难维护）。

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
- `mml-gui/src-tauri/resources/custom_home_bridge.js`
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
  「三、桥接协议」与 `mml-gui/src-tauri/resources/custom_home_bridge.js`）。示例至少演示：
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
| `mml-gui/src-tauri/src/windows/custom_home.rs` | **新增**：4 条命令 + 2 个事件 + `mml-home` handler。**最终形态**：压缩包常驻句柄 + 按条目现读（不再解压），见「偏差 6」 |
| `mml-gui/src-tauri/resources/custom_home_bridge.js` | **新增**：iframe 侧桥接脚本（`window.mml` = `invoke` / `on` / `onTheme` / `theme` / `ready`），由 `custom_home.rs` 用 `include_str!("../../resources/custom_home_bridge.js")` 编译期嵌入。**后续调整**：原先放在 `src-tauri/src/windows/` 下，已挪到 `resources/` 资源目录；后又增加主题下发（见「手工验收反馈」一节） |
| `mml-gui/src-tauri/src/windows/mod.rs` | `pub mod custom_home;` |
| `mml-gui/src-tauri/src/lib.rs` | 注册 `mml-home` 异步 scheme |

**资源目录调整（本轮）**：桥接脚本 `custom_home_bridge.js` 从 `src-tauri/src/windows/` 挪到
`src-tauri/resources/`，`custom_home.rs` 的 `include_str!` 路径同步改为 `../../resources/...`。
仍由 `include_str!` 编译期嵌入二进制，**没有**加进 `tauri.conf.json` 的 `bundle.resources`
（加了只会在安装包里多一份运行时没人读的副本）。

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
| `mml-vue/src/windows/settings/SettingsWindow.vue` | 客户端设置页新增「自定义主页面」分组（开关 / 导入 / 打开目录 / 删除 / 状态行 / 内联进度条），进页面 `refreshCustomHome()`；`client` 默认值补 `customHome: false` |
| `mml-vue/src/windows/main/MainWindow.vue` | **最后补齐**（见下） |

**最后一步（已补齐）**——`MainWindow.vue`：

- `customHome` ref（`custom_home_status` 结果）+ `useCustomHome = computed(...)`（`enabled && installed && entryUrl`，任一不满足回落内置主页）；
- `loadCustomHome()`：进 `doInit()` 的 `Promise.all`、`onClientConfig()`（启用开关走 `client-config-change`）、并订阅 `custom-home-change`（导入 / 删除不改 `gui_config`，靠这个事件回拉）；
- **三处**主页渲染点各加 `<CustomHomePage>` 分支：空实例分支、列表模式 `Transition` 里的 `key="home"` 节点、分组 / 平铺兜底分支的 `Transition`（该分支自定义页用 `key="custom-home"`，因为同一个 `v-if` 链里的分支 key 必须互不相同，编译器强制）；
- `clientConfig` 的默认字面量补 `customHome: false`。

`npm --prefix mml-vue run build` 退出码 0（先修掉两个**既有**类型错误：上一轮给 `ClientConfig` 加必填字段后，`MainWindow.vue` 与 `SettingsWindow.vue` 的 `ref<ClientConfig>({...})` 默认值没跟上，`vue-tsc` 报 `Property 'customHome' is missing`）。

### 未完成（恢复时从这里接着做）

- **代码改动已全部完成**（Rust + 前端 + `mml-ui-demo`；`npm run gen` / `cargo check -p mml-gui` /
  `npm --prefix mml-vue run build` / `mml-ui-demo` 的 `npm run build` 均通过）。
- **手工验收一条都还没跑**（需要用户自己起 `cd mml-gui && npm run tauri dev`）：
  - 主功能验收 1~7：**代码侧已就绪，导入 zip 后主窗口应显示自定义页面**
    （未导入 / 未启用则回落内置主页，不白屏）。
  - §八 的示例工程：`mml-ui-demo/mml-ui-demo.zip` 导入后应能列实例 / 启动游戏 / 显示启动阶段。
- 用户实测已确认「自定义主页面能加载」，并反馈了两个问题（**没铺满 / 不跟随亮色**），
  两项都已修（见「手工验收反馈后修的三个问题」）。需要复验：铺满效果、亮色跟随
  （含主题设为 `System` 后跟随系统切换），以及原验收 1~7。
  修这两项时改了 Rust 侧资源（桥接脚本），所以 `mml-gui` 需要重新编译——用户重启
  `tauri dev` 即可，无需其他操作。

### `mml-ui-demo` 实现情况（§八）

| 文件 | 说明 |
| --- | --- |
| `mml-ui-demo/index.html` | 入口模板（构建后 `dist/index.html` 在根目录，`base: "./"` 保证相对引用） |
| `mml-ui-demo/src/App.vue` | **服务器主页**示例：`SERVER`（名字 / 标语 / 滚动公告 / 进服方式 / 必读 / 开服时间 / 赞助名单 / 页脚）与 `EVENTS`（活动日历）两个常量即全部内容；交互部分 = 一键进服（`main_launch_game`）+ 实例列表（`main_get_instances`）+ 启动阶段（`launch-state` / `game-exit` / `launch-error`） |
| `mml-ui-demo/src/bridge.ts` | `window.mml` 使用封装：宿主探测（探针命令 `main_load_state`）、带超时 invoke、类型化订阅、主题读取 |
| `mml-ui-demo/src/mml.d.ts` | `window.mml` 与 payload 类型（含 `MmlTheme` / `onTheme`；**本工程自己维护**，不从启动器仓库引） |
| `mml-ui-demo/src/vite-env.d.ts` | `vite/client` 类型（`import "./style.css"` 需要） |
| `mml-ui-demo/scripts/pack.mjs` | 零依赖打包：优先 bsdtar，退回 PowerShell `Compress-Archive`；打 `dist/` 的**内容**，zip 根目录即 `index.html` |
| `mml-ui-demo/README.md` | 服主文档：改哪里（SERVER / EVENTS 对照表）、`window.mml` API、事件白名单、四个坑、**暗色 / 亮色跟随**、**全部 IPC 权限的安全说明** |

已核实的构建结果：`npm run build`（`vue-tsc --noEmit` + `vite build` + 打包）退出码 0，
产出 `dist/index.html` 与 `mml-ui-demo.zip`（2 个顶层条目，28.7 KB）。
zip 条目为 `index.html`、`assets/`、`assets/index-*.{js,css}`——**正斜杠、入口在根**，
符合启动器的入口定位规则；`dist/index.html` 保留 `</head>`，桥接脚本能被注入。

浏览器预览用 `npm run dev`（1520 端口，避开 mml-vue 的 1420）：连不上启动器时页面照常渲染，
顶部显示提示条、实例列表退回示例数据。

### 手工验收反馈后修的三个问题（用户实测：页面加载了但没铺满、且不跟随亮色）

#### 1. 页面没占满内容区（铺满 + 去内边距）

原因有两个，都在启动器侧：

- **高度解析不出来**：`.detail` 只在主轴上有确定尺寸（它是横向 flex 的子项），
  子元素 `height:100%` 解析不出高度 → iframe 被压扁。
- **内边距与底部留白**：主页容器 `.news-page` 带 `padding: 18px 28px 130px`，
  其中 130px 是给 MOTD 悬浮卡片的留白；自定义页面不需要这些。

改法（`MainWindow.vue`）：三处渲染点的自定义分支改成**独立占位块**——
空实例分支与列表模式用 `.custom-home-page`，分组 / 平铺兜底分支因为要撑满带 padding、
自身可滚动的 `.detail`，用 `.custom-home-fill`（`flex:1` + 负 margin 抵消 padding）：

| 位置 | 现在 |
| --- | --- |
| 空实例分支 | `<section v-if="useCustomHome" class="custom-home-page">`（否则才是 `.news-page`） |
| 列表模式 `Transition` | `key="custom-home"` 的 `.custom-home-page` / `key="home"` 的 `.news-page` |
| 分组 / 平铺兜底 `Transition` | `key="custom-home"` 的 `.custom-home-fill` / `key="home"` 的 HomePage |

`CustomHomePage.vue` 的 iframe 保持 `flex:1; width/height:100%`，父容器现在有确定高度了。

#### 2. 不跟随启动器主题（亮色模式）

iframe 是**独立文档**，拿不到启动器 `<html data-theme>` 上的 CSS 变量——原规格只解决了
「能不能调命令 / 订阅事件」，没管主题，所以自定义页面只有自己的暗色配色。现在加一条
**主题下发通道**（不动白名单与命令表）：

- 桥接脚本（`resources/custom_home_bridge.js`）：`window.mml` 增加 `theme` 与 `onTheme(cb)`；
  收到主题后写到自己的 `<html data-mml-theme="Dark|Light">` 并同步 `color-scheme`；
  父窗口应答前先用 `prefers-color-scheme` 兜底，避免首屏闪一下错误主题。
- 父窗口（`lib/customHomeBridge.ts`）：握手应答 `{ __mml:1, type:"ready", theme }` 带上当前主题；
  之后 `watch(resolvedTheme, ...)` 主动推 `{ __mml:1, type:"theme", theme }`
  （覆盖设置窗口切换与 System 跟随系统两种情况），卸载时停掉 watch。
- 示例工程：`style.css` 改成「暗色为默认 + `[data-mml-theme="Light"]` 覆盖变量」，
  `App.vue` 里硬编码的 `#ff7a7a` 换成变量；README 增加「暗色 / 亮色」一节。

主题值用的是启动器自己的 `Theme` 取值（`Dark` / `Light`，`System` 已由 `resolvedTheme()` 解析）。

#### 3. 主题跟随补强（System 模式下系统切换）

上一项只保证「启动器推什么页面就显示什么」，但**启动器主题设为「跟随系统」时**，若系统是在
窗口打开之后才切换深浅色，启动器侧 `matchMedia` 未必随窗口重开而更新，页面就不会变。
所以桥接脚本自己也在盯：`prefers-color-scheme` 变化时，**只要父窗口还没下发过明确主题**，
就按系统偏好更新自己（`themeFromSystemGuess` 标志；父窗口一旦下发即交权）。
这同时让「浏览器直接预览」这个场景也能跟随系统。

#### 4. 示例工程改成真正的「服务器主页」（本轮）

原来那个示例是个通用小面板，和「服主给服务器做定制」的定位不符。现在 `App.vue` 改成
服务器门面，内容集中在两个常量里，服主照着改即可：

- `SERVER`：服务器名 / 标语 / 滚动公告（ticker）/ 进服方式（`join`，第一项是服务器地址，用来
  匹配「推荐」实例）/ 进服必读 `rules` / 开服时间 `schedule` / 赞助名单 `sponsors` / 页脚 `contact`；
- `EVENTS`：活动日历（日期 + 标题 + 说明）。

页面结构：门面横幅（含「一键进服」主按钮）→ 环境提示条 → 左栏（最近活动 / 进服必读 /
选择客户端）→ 右栏（怎么进服 / 开服时间 / 赞助名单）→ 启动状态 → 页脚。
「一键进服」与列表里的「启动」都调 `main_launch_game`，状态区显示 `launch-state` 阶段
与 `game-exit` 结果；`serverInstance` 会优先挑 `serverUrl` 等于本服地址的实例并打上「推荐」。
样例数据全部换成服务器口吻（暮色群岛 / play.example.com / 群号等）。

#### 5. 浅色仍然不生效的排查与本轮加固

用户复测反馈「还是没有浅色样式」。排查过程（**用无头 Edge 跑了真实页面，没有截图**：
`target/temp/bridge_test/` 下自建了父窗口 harness + 页面，`--dump-dom` 读 DOM 状态）：

- 构建产物里两套配色变量都在（`[data-mml-theme=Light]` 确实进了 CSS），**不是样式丢失**；
- harness 里父窗口回 `ready + theme=Light` 后，页面 `<html>` 的 `data-mml-theme` **确实变成
  Light**，主题推送也生效 —— 桥接这条链路是通的；
- 结论：如果页面还是暗色，只可能是**页面自己没写浅色样式**（比如导入了自己写的、只有一套
  暗色的 HTML），或者跑的是**旧 zip**。桥接没法给别人的 HTML 变出配色，但可以做两件事：

本轮加固（都是为了让"没写主题样式的页面"也能亮）：

1. **裸 HTML 兜底样式**：桥接脚本在应用主题时，若页面**自己声明了配色**（body 有内联背景 /
   有 `link[rel=stylesheet]` / head 里有非空 `<style>`）就什么都不做；否则注入一份最小样式表
   （底色 / 文字色 / 链接色，两套变量），至少把整页白板压成跟启动器一致的深浅色。
2. **握手语义修正**（顺带修的既有小问题）：桥接脚本在收到应答前每 100ms 重发 `ready`，
   父窗口只回一条；实测第一次应答可能早于 iframe 的 message 监听就绪，导致 iframe **永远
   卡在握手前**（`window.mml.ready` 不 resolve、invoke 全部永挂）。现在父窗口**每条 ready
   都回**（幂等），并给握手完成前的 `invoke` 回一条错误而不是让它悬着。

示例工程侧同时把两套 token 补全（`--accent-grad` / `--accent-soft` / `--tag-dim-bg` /
`--ok-soft` 等），亮色下不再复用暗色的半透明底。

#### 6. 改成「只存压缩包 + 直接文件流读取」（用户要求，本轮最终形态）

用户要求：**不再解压到文件夹，直接文件流读取**。现在磁盘上只有一个包，没有任何解压产物：

| 东西 | 位置 | 说明 |
| --- | --- | --- |
| 服主导入的包 | `base_dir/custom_home.zip` | **唯一持久产物**，导入时只校验 + 复制 |
| 常驻句柄 | 进程内存（`static HANDLE`） | 启动 / 导入 / 删除时（重）打开，条目表只读一次 |
| 解压目录 | **没有** | 协议请求直接从 zip 条目现读（`BaseArchive::read`） |

Rust 侧（`windows/custom_home.rs`）：

- 新增 `HomeArchive`：`BaseArchive` + **规范化条目索引**（`HashMap<规范化名, 包内原名>`）。
  索引是必需的：zip 条目名可能是反斜杠（PowerShell `Compress-Archive` 就这么写），
  而 URL 路径一定是正斜杠；`entry_name()` 依次尝试 原名 / `./` 前缀 / 反斜杠版本。
- `reload()`：**（重）打开常驻句柄**，启动、导入、删除各调一次；包打不开或没有入口页时
  句柄留 `None`，协议层自然 404、主窗口回落内置主页。
- 协议 handler `url_custom_home`：不再碰文件系统，改成
  `resolve_entry(archive, path)`（路径安全 + 入口定位）→ `archive.read(entry)` → 按 MIME 返回。
  **路径穿越防护**依旧保留：原始串与百分号解码后都要过 `is_safe_rel`，且只接受
  **精确命中包内条目**（精确匹配而不是前缀拼接，不存在拼出包外的可能）。
- 入口定位统一到 `resolve_entry` / `HomeArchive::single_top_dir`：`/` 或空路径 → `index.html`；
  包根没有就试「唯一顶层目录/index.html」（即整包套一层目录的布局）。
- 删掉的：`ensure_extracted` / `cache_stale` / `clear_cache` / `clear_legacy` /
  `custom_home_prepare` 命令 / `custom_home-progress` 事件与 `CustomHomeProgressDto`
  （导入只是复制文件，没有可上报的进度）。`file_count` 改为**包内条目数**。
- `custom_home_open_dir` 改为「定位压缩包」：有包就在资源管理器里选中它，没有就打开运行根目录。
- `lib.rs`：`setup` 里一句 `windows::custom_home::reload()`；回落到只等 `mml_core::load()`
  就发 `load-done`（不再有解压任务要等），退出也不再需要清理 `RunEvent`。

前端：删掉进度条与 `onCustomHomeProgress`（事件已不存在），文案改成「正在读取压缩包…」、
「定位压缩包」，README 与 i18n 同步。

#### 6.1 过程中的两次中间形态（已被上面取代，留档说明为何废弃）

- **先做过「每次启动解压到缓存目录、退出清理」**：能满足「不留常驻解压目录」，但每次启动
  都要解一遍，且引入 zip 与缓存的新旧比较、启动页要不要等解压等问题。
- **再做过「解压与核心加载都结束才关启动页」**：解决了「主窗口出现时页面还没解压出来」的
  404 空档，但把启动时间变成了 `max(核心加载, 解压)`。
- 最终按用户要求改成**直接读包**，这两个问题都不存在了：没有解压步骤，也没有缓存，
  启动流程回到原来的样子（只等核心加载）。

### 与前面章节的偏差（需要确认是否接受）

1. **多了一个事件 `custom-home-change`**（`emit_custom_home_change`）。原规格只定义了
   `custom-home-progress`，并假设主窗口监听 `client-config-change` 就能跟上；但导入 / 删除
   都**不改** `gui_config.client`，`client-config-change` 不会发，主窗口无从知道要重拉状态
   （验收 1、7 会失效）。所以导入成功后与删除后各广播一次 `custom-home-change`。
2. `custom_home_remove` 的签名多了一个注入参数 `app: AppHandle`（TS 侧会被过滤掉，
   前端 API 仍是 `remove()`）—— 就是为了发上面那个事件。
3. `CustomHomeProgressDto.sub_text` 用的是 `String`（按 §2.1 的字面定义）；
   `JavaImportProgressDto` 是 `Option<String>`。空串表示没有当前文件名。
   （改动 6 之后它只在启动解压时发，用户看不到内联进度条——导入本身已经不解包了。）
4. `custom_home_open_dir` 在目录不存在时会先 `create_dir_all` 再打开（按钮因此始终可用）。
5. 细节：注入用的 `</head>` 匹配是**大小写不敏感**的；MIME 表额外收了 `htm`；
   百分号解码对非法转义原样保留（`100%.png` 这类文件名仍可用），解码后的 `..` / `\` / `:`
   照旧拒绝。
6. 新语义下的取舍：**改解压目录里的文件不再生效**（每次启动重建），服主要改内容必须重新导入
   包；另外 `custom_home.zip` 与缓存目录在运行根目录里并列，服主手动删掉 zip 就等于取消导入。


### 卫生

- 没有 `TEMP` 调试代码；`target/temp` 下只剩我为验证 `Compress-Archive` / `tar` 打包行为建的
  临时目录 `target/temp/packtest`（在 git 忽略范围内），本次结束前清掉。
- 上述源码改动都在工作区里，未提交；`mml-ui-demo/` 是新增目录，它自己的 `.gitignore`
  忽略 `node_modules/`、`dist/`、`*.zip`。

