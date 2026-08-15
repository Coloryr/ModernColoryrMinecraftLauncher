# mcml-vue

MCML 启动器的前端界面（独立 Vue 3 项目，不依赖 Tauri）。

## 运行

```bash
npm install
npm run dev      # http://localhost:1420
```

## 说明

- 当前使用模拟数据（`src/lib/api.ts`），可在纯浏览器中运行，方便先做界面。
- 接入真实后端时，将 `App.vue` 的导入从 `./lib/api` 切换为 `./lib/api-real`
  （真实实现通过 Tauri invoke 调用 Rust 命令，见 `../mcml-gui/src-tauri`）。
- 界面由 `mcml-gui`（Tauri 壳）通过 `frontendDist` / `devUrl` 引用本项目的构建产物与开发服务器。
