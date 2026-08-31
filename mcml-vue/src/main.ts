import { createApp } from "vue";
import "./styles/index.css";
import App from "./App.vue";
import { loadGuiConfig } from "./lib/guiConfig";
import { applyTheme, theme } from "./lib/theme";
import { applyLocale, locale } from "./lib/i18n";
import { loadAccounts } from "./lib/accountStore";
import { sidebarCollapsed, sidebarSide } from "./lib/settings";
import { setMultiWindow } from "./windows/windowManager";

// 禁用右键默认菜单（WebView2 / 浏览器自带的“刷新、返回、打印”等）。
// 文本输入框（input / textarea / contenteditable）保留原生菜单，方便复制粘贴。
document.addEventListener("contextmenu", (e) => {
  const target = e.target as HTMLElement | null;
  if (
    target &&
    (target.closest("input") ||
      target.closest("textarea") ||
      target.closest("[contenteditable]"))
  ) {
    return;
  }
  e.preventDefault();
});

/** 启动引导：GUI 状态由 Rust（gui_config.json）提供，浏览器回退 localStorage */
async function bootstrap() {
  const cfg = await loadGuiConfig();
  if (cfg) {
    theme.value = cfg.theme;
    locale.value = cfg.locale;
    sidebarSide.value = cfg.mainWindow.sidebarSide;
    sidebarCollapsed.value = cfg.mainWindow.sidebarCollapsed;
    setMultiWindow(cfg.windowMode !== "Single");
  }
  applyTheme();
  applyLocale();
  loadAccounts(); // 账户列表由 Rust 提供（浏览器环境静默跳过）
  createApp(App).mount("#app");
}

bootstrap();
