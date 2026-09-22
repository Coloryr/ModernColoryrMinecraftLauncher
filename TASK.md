# 方块列表（主界面）实施计划

## Context

启动器主页面的「每日抽奖」占位卡改为「方块列表」：点卡片切入方块网格视图（搜索 / 分类 / 滚动，返回键回主页）。方块贴图由 mcml-tex-draw 渲染；**默认不渲染**，首次打开提示渲染，渲染带进度条，可手动重新渲染；贴图可设为实例图标。

探索确认的关键事实（已验证）：
- mcml-core **没有** re-export mcml-tex-draw，mcml-gui 也未依赖它 → 需在 `mcml-gui/src-tauri/Cargo.toml` 加 `mcml-tex-draw = { path = "../../mcml-core/mcml-tex-draw" }`
- `InputFile` 在 mcml-net（`input_file.rs:19`）；`InputFile::Path.save_file` 本质是 `copy_file_async`，set-icon 直接用 `tokio::fs::copy` 即可
- ConfigObj 有 struct 级 `#[serde(default)]`（config_obj.rs:422），新增字段无需迁移；`mcml_config::init` 先于 `mcml_tex_draw::load`（mcml-core/src/lib.rs:75-77,93），tex-draw 可安全读配置
- 前端从未构造过 mcml-image URL；InstanceIcon.vue 目前只画渐变占位、不加载真实图标 → 本次一并接通
- `t()` miss 时返回 key 本身（i18n/index.ts:41），可作分类名回退
- `blocks()` 顺序不稳定 → 列表命令需排序
- 事件名：`emit_main_block_render` → `"main-block-render"`，由 build.rs/gui-ipc-gen 自动写入 listens.ts
- 进度回调参照 `windows/add_modpack.rs:126-163` 的 TaskPackGui 模式

## 第 0 步：写 TASK.md

用本任务替换 `D:\code\ModernColoryrMinecraftLauncher\TASK.md` 现有内容（现为「优化整合包搜索用时」的旧记录），按其既有格式写：背景 / 需求 / 方案要点 / 验收。

## 第 1 步：mcml-config 开关字段

`mcml-core/mcml-config/src/config_obj.rs`（ConfigObj ~:423）：
```rust
/// 方块贴图已渲染（用户同意后开启；开启后启动时自动补渲染缺失版本）
#[serde(rename = "BlockRender")]
pub block_render: bool,
```
Default 中补 `block_render: false`。

## 第 2 步：mcml-tex-draw 改造

`mcml-core/mcml-tex-draw/src/lib.rs`：
1. `load_blocks(gui, force: bool)`（:30）：短路条件加 `!force &&`（`block_done`/`item_done`），force=true 强制全量重渲染；`spawn_load_task`（:154）传 `(None, false)`
2. `load()`（:138）：仅在 `mcml_config::read_config().block_render` 为 true 时 `spawn_load_task()` —— 实现「默认不渲染」
3. 新增访问器（GUI 无法读 `BlocksObj.name`，均为 pub(crate)）：
   ```rust
   pub fn block_version() -> String                       // BLOCKS.read().id，未渲染为空
   pub fn block_name_key(id: &str) -> Option<String>      // name: id → lang key
   ```

## 第 3 步：image_manager /block/ 端点 + 图标缓存失效

`mcml-gui/src-tauri/src/image_manager.rs`：
1. `image_base_url()`（:476）改为 `pub`
2. `url_image` 分发（:457）加 `"block" => load_block_image(&uri, res)`
3. 新函数（PNG 直接读盘透传，**不进内存缓存**，重渲染即时生效；webview 缓存靠 URL `?v=<version>` 破）：
   ```rust
   fn load_block_image(uri: &[&str], res: UriSchemeResponder) {
       if uri.len() != 2 { send_bad(res); return; }
       let Some(file) = mcml_tex_draw::get_block_path(uri[1]) else { send_bad(res); return };
       match path_helper::read_byte(&file) { Ok(data) => send_png(res, data), Err(_) => send_bad(res) }
   }
   ```
   （`uri[1]` 形如 `minecraft:stone`，冒号保留在同一段内）
4. 实例图标失效：
   ```rust
   pub fn clear_instance_image(uuid: &Uuid) { INSTANCE_IMAGE.write().unwrap().remove(uuid); }
   ```

## 第 4 步：DTO

`mcml-gui/src-tauri/src/dtos/main_dto.rs`（`#[serde(rename_all = "camelCase")]`）：
- `BlockItemDto { id, name, cat, image }` —— cat 为创造分组尾段（buildingBlocks/natural/…），image 为完整 mcml-image URL（带 ?v=）
- `BlockStatusDto { rendered, opt_in, version, running, now, total, text: Option, error: Option }`
- `dtos/mod.rs` 扩充 main_dto 的 re-export

## 第 5 步：main.rs 命令 + 事件

`mcml-gui/src-tauri/src/windows/main.rs`（参照 add_modpack.rs TaskPackGui 模式）：

- 静态状态 `BlockRenderState { running: AtomicBool, now/total: AtomicUsize, text/error: Mutex<Option<String>> }`，`static BLOCK_RENDER: LazyLock<_>`
- `struct BlockRenderGui { app: AppHandle }` 实现 `IProgressGui`：写状态并 `emit_main_block_render(&app, block_status())`（回调全同步，简单）
- `fn block_status() -> BlockStatusDto`：读 BLOCK_RENDER + config.block_render + `blocks()` 非空 + `block_version()`
- 事件：`#[gui_macros::emit] pub fn emit_main_block_render(app, BlockStatusDto)`
- 命令：
  - `main_block_list(lang: String) -> Vec<BlockItemDto>`：Lang 按字符串解析（en_us / 其余 zh_cn）；name = `block_name_key` → `get_lang` miss 回退 id 尾段；image = `{base}/block/{id}?v={ver}`；按 cat + name 排序
  - `main_block_status() -> BlockStatusDto`
  - `async main_block_render_start(app, force: bool) -> Result<bool, String>`：`running.swap` 防重入（已跑返回 Ok(false)）；写 `write_config().block_render = true` + `mcml_config::save()`（guard 先 drop 再 save）；spawn `load_blocks(Some(gui), force)`，结束时写 error/running=false 并 emit
  - `async main_block_set_icon(app, uuid, id) -> Result<bool, String>`：解析 uuid → `get_instance`（miss → `err.instanceNotFound`）；**锁内只取 `get_icon_file()` 后 drop 再 await**（std 锁跨 await 破坏 Send）；`tokio::fs::copy(get_block_path(&id)?, dest)`；`clear_instance_image`；`emit_instance_change(&app, "edit")` 让 InstanceIcon 刷新

## 第 6 步：前端 api

bindings.ts / listens.ts 由 build.rs 自动生成（先跑 cargo check）。`mcml-vue/src/lib/api.ts` 增加：`getBlockList(lang)`、`getBlockStatus()`、`blockRenderStart(force)`、`blockSetIcon(uuid, id)`、`onBlockRender(cb)`，类型引入 BlockItemDto/BlockStatusDto。

## 第 7 步：BlockPanel.vue（新组件）

`mcml-vue/src/components/BlockPanel.vue`，props `currentInstance`，由 `status` 驱动三态：
- **未渲染且未在跑**：提示卡（renderPromptTitle/renderPromptDesc）+「开始渲染」按钮 → `blockRenderStart(false)`；有 error 时显示失败 + 重试
- **running**：进度条 now/total + `status.text`
- **rendered**：工具栏（搜索框、分类 chips「全部+各分组」、重新渲染按钮）+ 网格（AsyncImage 图标 + 名称）；数据 `getBlockList(locale)`，`watch(locale)` 重取；过滤 = 分类匹配 + id/name 含关键字；条目 `content-visibility: auto`（上千条）；点条目：无选中实例 → toast `blocks.noInstance`；否则 `blockSetIcon` → 成功/失败 toast

## 第 8 步：HomePage.vue + InstanceIcon.vue

- HomePage：抽奖卡（:104-117）替换为「方块列表」卡（立方体 SVG + home.blocks/blocksDesc）；`showBlocks` ref 切换视图，主体 `v-if="!showBlocks"` 包住 entry-cards + NewsPanel，else 渲染 `<BlockPanel :current-instance="currentInstance"/>`；返回键 label 在 `home.backToList` / `blocks.back` 间切换
- InstanceIcon.vue：加载真实图标 `${base}/instance/${uuid}`（base 经新小命令 `main_image_base_url()` 取 `image_base_url()`），渐变+字母作 `@error` 回退；监听 `InstanceChange`（type=edit 且 uuid 匹配）时加 `?v=Date.now()` 破缓存刷新

## 第 9 步：i18n（zh-CN + en-US）

删 `home.lottery` / `home.lotteryDesc`。新增：`home.blocks`、`home.blocksDesc`、`blocks.back/search/catAll/renderPromptTitle/renderPromptDesc/renderNow/rendering/reRender/renderFailed/retry/noInstance/setIconOk/empty/count`、`blocks.cat.*`（buildingBlocks/natural/functionalBlocks/redstoneBlocks/tools/combat/foodAndDrinks/ingredients/spawnEggs/opBlocks/construction/misc，未知值回退原文）。

## 边界情况

- 渲染中关主窗口：spawn 的任务持 AppHandle 继续，事件无人听无害；重开窗口 `main_block_status()` 重同步
- 首渲染中断：`block_render` 在 spawn 前已置 true，下次启动版本短路自动续渲染缺失部分
- 重渲染覆盖同版本 PNG：无内存缓存即时生效；webview 缓存靠 `?v=`（同版本重渲染纹理相同，可接受）
- `blocks()` 空 → rendered=false → 提示视图，永不出空网格；get_lang miss → id 尾段回退；get_block_path miss → 400 → AsyncImage 灰块
- 伪 uuid 实例（mcml- 前缀）设图标报 `err.instanceNotFound`
- 双击渲染按钮：AtomicBool::swap 拦截

## 验证

1. `cargo check -p mcml-config`、`cargo check -p mcml-tex-draw`（workspace 根）
2. `cd mcml-gui/src-tauri && cargo check` —— 同时再生成 bindings.ts/listens.ts，确认 `BlockRender` const 与 `commands.main.block*` 出现
3. `cargo test -p mcml-tex-draw`（现有 init/names 测试保持绿）
4. `cd mcml-vue && npx vue-tsc --noEmit && npx vite build`
5. 手动：首开提示 → 渲染进度 → 完成出网格；重启自动补渲染；搜索/分类/设为图标即时生效（InstanceIcon + 主页卡片）；语言切换改名；渲染中重复点击无效
