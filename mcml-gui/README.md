# mcml-gui

MCML 启动器的 Tauri 壳。

## 结构

```
mcml-vue  前端界面（独立 Vue 3 项目，纯浏览器可运行）
mcml-gui  Tauri 壳（Rust 后端 + 打包）
mcml-core 启动器核心（Rust，暂未链接）
```

`mcml-gui` 本身不含前端代码，通过 `src-tauri/tauri.conf.json` 引用 `../mcml-vue`：

- `devUrl` → `http://localhost:1420`（mcml-vue 的 Vite 开发服务器）
- `frontendDist` → `../mcml-vue/dist`（mcml-vue 的构建产物）
- `beforeDevCommand` / `beforeBuildCommand` → 自动进入 `mcml-vue` 执行 npm 脚本

## 运行

```bash
# 纯浏览器预览界面（不需要 Tauri）
cd ../mcml-vue && npm install && npm run dev

# 以 Tauri 桌面应用运行
npm install
npm run tauri dev
```

## 状态

当前阶段只做界面：前端使用模拟数据（`mcml-vue/src/lib/api.ts`），
后端（`src-tauri/src/lib.rs`）保持最小化，未链接 mcml-core。
接入后端时恢复 `src-tauri/Cargo.toml` 中的 mcml 依赖与 `lib.rs` 中的命令，
并把 `mcml-vue/src/lib/api-real.ts`（真实 invoke 实现）切换为默认。
