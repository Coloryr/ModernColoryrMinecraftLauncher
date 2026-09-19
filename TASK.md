# TASK：把 IPC 源生成器从 build.rs 抽成独立包

## 背景

`mcml-gui/src-tauri/build.rs` 现在 **681 行**，混着三件事：

1. **Tauri 构建胶水**：`tauri_build::build()`、`cargo:rerun-if-changed` 指令
2. **纯生成逻辑**（约 15 个自由函数，占绝大多数行数）：扫源码、解析命令签名、解析
   struct / enum、Rust 类型 → TS 类型映射、拼生成文本
3. **环境耦合 + 编排**：读 `CARGO_MANIFEST_DIR` / `OUT_DIR`、算输出路径、写文件

第 2 类是整个仓库里最需要单测、却**一行测试都没有**的部分（`mcml-gui/` 下不存在任何
`#[test]`）。抽成独立包之后，它可以脱离 cargo 构建脚本单独喂输入、断言输出。

目标：把第 2 类抽成 `mcml-gui/ipc-gen/` 包，`build.rs` 只剩胶水与编排（约 40 行），
生成物**保持字节一致**。

## 参照物

`mcml-gui/macros/`（包名 `gui-macros`）是现成模板，照它摆：

- 位于 `mcml-gui/` 下、与 `src-tauri/` 平级
- `Cargo.toml` 只有 `[package]` + `[lib]`，**不带 `[workspace]`**（自成根，靠 path 被引用）
- `src-tauri/Cargo.toml` 用 `{ path = "../macros" }` 引

新包同样不放 `[workspace]`；因为只在构建期用，走 **`[build-dependencies]`**
（`macros` 走 `[dependencies]` 是因为它要在 `src/` 的代码里用）。

## 新包设计

**位置 / 名字**：`mcml-gui/ipc-gen/`，包名 `gui-ipc-gen`。

```
mcml-gui/ipc-gen/
  Cargo.toml          # name = "gui-ipc-gen", edition 2024, 无依赖
  src/lib.rs          # 配置结构 + generate() 入口 + write_if_changed()
  src/scan.rs         # walk_rs / module_path / fns_after / commands_in / scan_types / rename_all_of / split_top_level
  src/ts.rs           # generic_inner / ts_type / ts_field / camel / pascal / screaming + TS 文本拼装
  src/emit_rs.rs      # invokes_gen.rs 文本（listens 常量 + tauri_commands! 宏）
  tests/generate.rs   # 端到端：临时目录放几个 .rs，断言生成的 TS
```

**公开 API**（关键：**纯生成**与环境耦合分开，这是能单测的前提）：

```rust
pub struct GenConfig {
    /// 要扫描的 Rust 源码根（通常 <crate>/src）
    pub src_dir: PathBuf,
    /// TS 类型来源：相对 src_dir 的目录前缀 + 需要取枚举的单文件
    /// 默认值 = 本仓现状（["dtos/", "models/", "gui_config.rs"]）
    pub type_sources: TypeSources,
    /// 外部 crate 类型 → TS 覆盖（如 mcml-names 的 Lang -> `"zh_cn" | "en_us"`）；
    /// 不在表里的类型名按「本 crate 生成」处理
    pub external_types: BTreeMap<String, String>,
}

pub struct Generated {
    pub bindings_ts: String,
    pub listens_ts: String,
    pub invokes_gen_rs: String,
}

/// 纯函数：只读源码、只返回文本，不碰环境变量也不写文件
pub fn generate(cfg: &GenConfig) -> Result<Generated, GenError>;

/// 只在内容变化时写 + 返回是否写了（cargo 指令由调用方负责打）
pub fn write_if_changed(path: &Path, content: &str) -> io::Result<bool>;
```

**要参数化掉的硬编码**：

| 现状硬编码 | 处理 |
| --- | --- |
| `manifest.join("src")` | `GenConfig.src_dir` |
| `dtos/` + `models/` + `gui_config.rs`（TS 类型来源） | `GenConfig.type_sources`，带默认值 |
| `"Lang" => "\"zh_cn" \| "en_us\""` | `GenConfig.external_types` |
| `invokes_gen.rs` / `listens.ts` / `bindings.ts` 的路径 | 由 build.rs 拼好传给 `write_if_changed` |
| `#[tauri::command]` / `#[gui_macros::emit]` 标记串 | 包内常量（本仓约定，不做成配置） |
| 注入参数表（`AppHandle` / `WebviewWindow` / `Window` / `State`） | 同上，包内常量 |

## 抽完的 build.rs（约 40 行）

```rust
fn main() {
    tauri_build::build();

    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=build.rs");

    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let src = manifest.join("src");
    let ts_dir = manifest.join("../../mcml-vue/src/lib");
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());

    let gen = gui_ipc_gen::generate(&gui_ipc_gen::GenConfig {
        src_dir: src,
        ..Default::default()
    })
    .expect("生成 IPC 绑定失败");

    gui_ipc_gen::write_if_changed(&out_dir.join("invokes_gen.rs"), &gen.invokes_gen_rs).unwrap();
    for path in [ts_dir.join("listens.ts"), ts_dir.join("bindings.ts")] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    gui_ipc_gen::write_if_changed(&ts_dir.join("listens.ts"), &gen.listens_ts).unwrap();
    gui_ipc_gen::write_if_changed(&ts_dir.join("bindings.ts"), &gen.bindings_ts).unwrap();
}
```

**「输出文件也登记 rerun-if-changed」和「内容没变就不写」这两条不能丢**：否则删掉
`bindings.ts` 后 cargo 不会重新生成（`rerun-if-changed` 只盯 `src` / `build.rs`），
而每次都写又会刷新 mtime、反过来让 build.rs 反复重跑。

## 测试

`mcml-gui/` 下目前零测试。新包加 `src/` 内的 `#[cfg(test)] mod tests`（纯解析函数）+
`tests/generate.rs`（端到端，用 `std::env::temp_dir()` 建临时 src 目录，跑完清理）。

覆盖这几轮踩过的坑：

| 用例 | 断言 |
| --- | --- |
| 注入参数三种顺序（`app` / `window, app` / `app, window`） | 都不出现在 TS 参数里 |
| `Result<T, String>` / 无返回类型 / `-> ()` | 分别 → `T` / `void` / `void` |
| `Option<Option<T>>` 字段 | 可缺省 + `T \| null` |
| 只 derive `Deserialize` 的 DTO | 字段全可缺省 |
| 无 `rename_all` 的 struct | 键名保持 snake（如 `is_dir`） |
| 枚举带文档注释 + `#[default]` | 变体不丢，`rename_all` 生效 |
| `Vec<&'static str>` | → `string[]` |
| 跨行命令签名 | 参数与返回都解析正确 |
| 分组与公共前缀剥离 | `add_modpack_search` → `commands.addModpack.search`（不是 `modpackSearch`） |

## 另外要修：DirEntry 是唯一 snake_case 的类型

`mcml-gui/src-tauri/src/dtos/add_dto.rs` 的 `DirEntry` 是整条 IPC 链路上**唯一**没有
`#[serde(rename_all = "camelCase")]` 的类型，线上键因此是 `is_dir`：

```ts
export type DirEntry = {
  name: string,
  is_dir: boolean,        // ← 全仓唯一一处 snake
};

// 参照：其它类型都是 camelCase
export type AccountOAuthStateDto = {
  state: string,
  message: string | null,
};
```

**修法**：给 `DirEntry` 补上 `#[serde(rename_all = "camelCase")]`（线上键变 `isDir`），
并同步改前端唯一使用点 `mcml-vue/src/windows/add/AddInstanceWindow.vue:417-418`
的 `en.is_dir` → `en.isDir`。

**注意**：这是改**线上契约**（Rust 侧），不是改生成器。生成器必须继续忠实读
`#[serde(...)]`——`DirEntry` 现在生成的 `is_dir` 是**正确反映**了 Rust 的写法，
让生成器去强行转 camelCase 反而是掩盖问题。

## 不做

- 不改生成物内容 / 格式（目标是**字节一致**的重构）
- 不动 Rust 侧 `tauri_commands!` 注册机制与 events 那套
- 不引入第三方依赖（新包只用 std）
- 不建 golden 文件目录（断言字符串即可，免得再加 gitignore）

## 验证

按 AGENTS §3：动手前先确认程序没在跑；在跑就先关掉。

1. **重构前留指纹**：`md5sum mcml-vue/src/lib/{bindings,listens}.ts`
2. `cd mcml-gui/ipc-gen && cargo test -p gui-ipc-gen` —— 单测通过
3. `cd mcml-gui/src-tauri && cargo check -p mcml-gui` —— 0 错误；**再比对两个 TS 的 md5，
   必须与第 1 步完全一致**（本次重构的核心断言）
4. `cd mcml-vue && npx vue-tsc --noEmit` + `npx vite build` —— 通过
5. 更新 `AGENTS.md §4`（现写「由 `mcml-gui/src-tauri/build.rs` 扫描生成」，
   要改成生成器在新包、build.rs 只是调用方）
