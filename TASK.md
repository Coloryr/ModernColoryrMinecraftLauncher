# 待办与交接

> 记录**本轮已完成**的改动、**尚未完成**的待办，以及交接时的环境基线。
> 开工前先看 §0。

## 0. 开工前必读（本仓库约定）

- **改任何代码前必须先关掉程序**（`mml-gui.exe` / 占着 1420 的 vite / cargo），改完**不要**替用户启动：
  ```powershell
  tasklist /FI "IMAGENAME eq mml-gui.exe"
  netstat -ano | Select-String ":1420.*LISTENING"   # 最后一列是 PID
  taskkill /PID <pid> /T /F
  ```
  其它来路不明的 `node.exe` 不要动。
- **验证只跑最小集**：前端 `npx vue-tsc --noEmit` + `npm run build`（在 `mml-vue/`）；
  动 Rust 时 `cargo check -p mml-gui --all-targets`（仓库根）。
  **不要**用 `... | Select-String "error" | Select-Object -First N` 这类**截断**输出的过滤（会漏 error）。
- 命令与 DTO 的唯一来源是 Rust 源码，改完跑 `cd mml-gui && npm run gen`（或任意 `cargo check`）重新生成
  `mml-vue/src/lib/bindings.ts`，**不要手改**。
- 临时诊断日志统一带 `[TEMP]` 前缀，定位完立即删除。

---

## 1. 立即接手：资源窗口「分组折叠」（**未完成，只差这一步**）

用户要求：**分组可折叠**；默认分组不可重命名/删除（后者**已完成**，见 §2.4）。

要改 `mml-vue/src/windows/resource/parts/ModPane.vue` 与 `resource.css` 四处：

1. `ModPane.vue` 顶部把 `GlyphIcon` 加回导入（当前没有导入它）
2. 加内存态折叠集合 + 两个小函数：
   ```ts
   const collapsed = ref<Set<string>>(new Set());
   function isOpen(key: string) { return !collapsed.value.has(key); }
   function toggleCollapse(key: string) { /* Set 增删后换引用 */ }
   ```
3. 模板：分组头里的 `<span class="mod-group-name">` 换成「箭头 + 名字」的按钮
   （箭头写法照 `windows/collect/CollectWindow.vue` 的分组头：`GlyphIcon name="chevron-down"`
   + `.collapsed { rotate: -90deg }`）；`<div class="mod-group-body">` 加 `v-show="isOpen(sec.key)"`
4. `resource.css` 补 `.mod-group-toggle`（透明按钮、flex、gap）与 `.mod-group-caret`（旋转过渡）两条

**已具备的前提**（别改坏）：折叠与否都不影响投放 —— 分组头所在的 `<section>` 一直挂着
`data-mod-group`（只有 `sec.custom` 才有），拖拽落点判定与折叠状态无关。
可选增强：拖到折叠分组上自动展开（实例侧栏有类似行为）。

---

## 2. 本轮已完成（供复查）

### 2.1 资源管理窗口重构

`windows/resource/ResourceWindow.vue` 从 **1120 行单文件**拆成**薄外壳（~85 行）+ 19 个文件**，
照 `windows/block`（薄窗口 + `parts/` + `composables/`）与 `windows/settings`
（窗口级 **非 scoped** `resource.css`，所有规则挂在 `.resource-layout` 下防外溢）的既有约定：

- `composables/`：`useResourceData`（实例 + 七类列表 + 加载代次号 + 数据包子页）、
  `useResourceOps`（忙碌 / 确认弹窗 / 打开目录 / 重拉）、`useResourceView`（视图偏好）、
  `useModGroups`（模组分组）、`useModDrag`（归组拖拽）
- `parts/`：`CategoryRail` / `ContentHead` / `ResourceRow` / `SaveTabs` / `InstanceChip` /
  `ConfirmModal` / `ServerFormModal` + 八个分类 pane + `mod/{ModList,ModTable,ModTree}`

顺手修掉的真问题（都在这次拆分的范围里）：

| # | 问题 | 修法 |
| --- | --- | --- |
| 1 | 删除 / 清空后列表**不刷新**（原 `runConfirm` 只跑动作） | 确认成功后 `reloadCurrent()` |
| 2 | 「打开文件夹」会**重拉整份列表**（模组要重解析 jar，数秒） | 只读操作单独走一条不重拉的路径 |
| 3 | 分类文案**不跟语言**（`t()` 在 setup 里被冻住） | 只存 `labelKey`，渲染时 `t()` |
| 4 | 分类连点的**加载竞态** | 加 `loadSeq` 代次号 |
| 5 | 无实例时"未选择实例"与该分类空列表**同时出现** | 无实例只显示那一条引导 |
| 6 | 服务器保存中显示的是"正在删除…" | 新增 `actions.saving` |

### 2.2 资源窗口的功能（用户逐条提的）

- **实例名挪到标题栏**（`parts/InstanceChip.vue`，走 `WindowFrame` 的 `head-right` 槽），**不套框**（纯文字）
- **「下载资源」按钮**：开 `add_resource` 窗口
- **左侧分类可拖动排序 + 记住上次打开的类别**：偏好存 localStorage `mml.resourceView`
  （顺序 / 类别 / 模组视图），新增分类自动补末尾
- **模组三种展示方式**（`parts/mod/`，顶栏 SegmentedTabs 切换、记忆）：
  **列表**（图标 + 名称/徽标 + 副标题 + 操作）、**表格**（名称/版本/作者/文件名/简介 + 操作，点表头排序）、
  **树形**（按 **jar-in-jar** 递归展开，内置行只展示）
- **模组自定义分组**：数据落在**实例目录的 `guisetting.json`**（`Mod.Groups`，与 ColorMC 互通），
  成员按 **SHA1**（内容哈希：启用/禁用只是改文件名，uuid 会变）
- **默认三个状态分组**：已启用 / 已禁用 / 识别失败（只装"不在自建分组里"的模组，空的收起，只读）
- **搜索框**：命中名称/modid/文件名/作者/版本/简介；三视图与分组内都过滤；搜索词不持久化
- **行内操作悬停显示**（浮在行右端、平时不占宽度，`pointer-events` 挡住误触）

### 2.3 界面清理（用户逐条提的）

- **不要框框套框框**：`.item-list` / `.shot-list` 去掉外层卡片，行自己就是卡片
  （底色 `--bg-side` → `--bg-card`）；分组 + 表格视图下每组表头不再画浮条
- **标题不被挤掉**：`WindowFrame` 的 `.frame-head h1` 加 `flex-shrink: 0` + 不换行
- **工具条控件同高 35px**（`SegmentedTabs` 本来就是 35，`.mini-btn` 默认 28）：搜索框与
  `.head-actions .mini-btn` 统一到 35
- 收藏窗口的拖拽幽灵卡片、`[TEMP]` 日志清理、两处 Vue 监听泄漏、`cargo check` 的 `unused app` 等
  更早的一批见 git 历史（都已验证通过）

### 2.4 内核修复（界面报出来的问题，顺手查到的）

- **`core` 判定错误**：`read_core_mod` 解析 `META-INF/MANIFEST.MF`（几乎每个 Forge/NeoForge jar 都有）
  却**无条件** `core = true` → 界面上每个模组都挂"核心"徽标。现在只有真的命中
  `FMLCorePlugin` / `TweakClass` 才算
- **模组图标从来没被读过**：`ModItemObj.icon` 只有声明和默认值，全仓没有赋值 →
  界面上永远是 "M" 占位。新增 `read_mod_icon` / `find_mod_icon_path` / `forge_logo_file`：
  Forge/NeoForge 的 `mods.toml` `logoFile`（**跳过被注释掉的那行**，模板里默认是注释）、
  Fabric / Quilt 的 `icon`（字符串或按尺寸的对象），上限 512 KiB；
  单元测试 `icon_tests`（2 个用例）通过。**老式 `mcmod.info` 的 `logoFile` 暂未支持**

---

## 3. 已知待办（零散）

- **模组备注**：`guisetting.json` 里 `Mod.ModName`（文件名 → 用户说明）是现成字段，未接界面
- **状态分组当落点**：拖模组到「已禁用」= 禁用（目前状态分组不可投放，拖过去等于取消）
- **分组顺序按名字排序**：`Groups` 是 `HashMap`，没有"建立顺序"（自建分组同理）
- **模组图标**：`mcmod.info`（1.12 老包）的 `logoFile` 未支持；图标 data URL 固定按 `image/png` 声明
  （JPG 图标的包靠浏览器嗅探兜底）
- **收藏窗口的「点卡片仍能下载」**（可选）：改 `CollectWindow.vue` 的 `toggleCard`，
  随后 `windowParams.project` 透传、两个下载窗口的 `consumeTargetProject`、后端 `collect_project_item`
  都会变成无引用，需一起清理
- **下拉框弹层口径统一**（可选）：`.filter-select` / `.inst-select` / `.dp-select` 仍是系统原生弹层
- **既有失败测试**（与本轮无关，未改）：`cargo test -p mml-names --lib` 的
  `lang_and_i18_lifecycle` 断言 `ConfigError` 文案是"保存失败"，而实现早已是"处理失败"
- 注意：`mml-gui/src-tauri/src/gui_setting.rs` 里有一处**不是本轮改动**的未提交修改
  （`Groups` 的注释由"模组文件名集合"改成"模组SHA1集合"）

---

## 4. 当前工作区状态（基线）

- 程序**未运行**（`mml-gui.exe` 无、1420 未监听）。
- 最后一次验证：`cargo check -p mml-gui --all-targets` 无 error；`npx vue-tsc --noEmit` 退出码 0；
  `npm run build` ✓；`cargo test -p mml-game --lib icon_tests` 2 passed。
- 本轮重点文件：
  - 前端：`mml-vue/src/windows/resource/**`（外壳 + 5 composables + 20 parts）、
    `mml-vue/src/lib/api.ts`（模组分组包装）、`mml-vue/src/lib/i18n/locales/*`、
    `mml-vue/src/components/ui/WindowFrame.vue`、`mml-vue/src/components/ui/BaseModal.vue`、
    `mml-vue/src/lib/pageActive.ts`、`mml-vue/src/lib/progress.ts`、
    `mml-vue/src/composables/useCollectDrag.ts`、`mml-vue/src/windows/collect/CollectWindow.vue`
  - Rust：`mml-gui/src-tauri/src/windows/resource.rs`（模组 DTO 递归 + 5 条分组命令，
    分组走 `crate::gui_setting`）、`dtos/resource_dto.rs`、`windows/add*.rs`、`windows/mod.rs`、
    `mml-core/mml-game/src/{game_mods.rs,add_game.rs}`
- **待用户实测**：
  1. 模组页：三视图切换 / 排序 / 树形展开内置模组；搜索；自建分组的增删改 + 拖动归组；
     默认三个状态分组正确（禁用几个、放一个坏 jar 试「识别失败」）
  2. 模组图标：Forge/NeoForge 包应显示 `logoFile` 里的图，Fabric/Quilt 显示 `icon`；
     没有图标字段的仍是 "M" 占位；**不应**再有满屏"核心"徽标
  3. 界面：工具条一行齐平 35px、列表无外框、标题栏实例名是纯文字
  4. 分类拖动排序 + 重开窗口后顺序与上次类别是否记住
