import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";

// 自定义主页面示例工程
//
// 产物（dist/）由启动器用 mml-home:// 协议从解包目录 custom_home/ 的根提供，
// 所以资源引用一律用相对路径（base: "./"）——入口页在包根或「整包只有一个子目录」的
// 两种定位结果下都能正确加载。
export default defineConfig({
  plugins: [vue()],
  base: "./",
  server: {
    // 只在浏览器里预览用（无 IPC 数据）；避开 mml-vue 的 1420
    port: 1520,
    strictPort: true,
  },
});
