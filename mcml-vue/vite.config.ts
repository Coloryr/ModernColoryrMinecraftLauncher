import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  server: {
    // 与 Tauri（mcml-gui）的 devUrl 保持一致；不使用 Tauri 时可自行修改
    port: 1420,
    strictPort: true,
  },
});
