// 应用入口：禁用默认右键菜单，加载 GUI 配置后挂载根组件
import { createApp } from "vue";
import "./styles/index.css";
import App from "./App.vue";
import { loadGuiConfig } from "./lib/guiConfig";
import { applyTheme, theme } from "./lib/theme";
import { applyLocale, locale } from "./lib/i18n";
import { loadAccounts } from "./lib/accountStore";
import {
  restoreHeadConfig,
  restoreSelectedInstance,
  restoreSkinDisplay,
  sidebarCollapsed,
  sidebarSide,
  viewMode,
} from "./lib/settings";
import { restoreBg } from "./lib/appearance";
import { setMultiWindow } from "./windows/windowManager";
import { restoreFontFamily } from "./lib/fonts";
import { installImageFallback } from "./lib/imageFallback";
import { vTip } from "./lib/tip";

// 图片加载失败的统一兜底：装上要趁早，免得启动过程中先加载的那批图漏掉
installImageFallback();

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

/** 启动引导：GUI 状态由 Rust（gui_config.json）提供，浏览器回退本地存储 */
async function bootstrap() {
  const cfg = await loadGuiConfig();
  if (cfg) {
    theme.value = cfg.theme;
    locale.value = cfg.locale;
    sidebarSide.value = cfg.mainWindow.sidebarSide;
    sidebarCollapsed.value = cfg.mainWindow.sidebarCollapsed;
    viewMode.value = cfg.mainWindow.viewMode;
    restoreSelectedInstance(cfg.mainWindow.selectedInstance);
    setMultiWindow(cfg.windowMode !== "Single");
    // 字体 / 皮肤与头像显示模式：只同步内存状态，不回写配置
    restoreFontFamily(cfg.font);
    restoreSkinDisplay(cfg.skinDisplay);
    restoreHeadConfig(cfg.head.headType, cfg.head.x, cfg.head.y);
  }

  // 背景图：不阻塞挂载（大图走 base64 IPC 很慢，挂着等它=黑屏一阵）。
  // 正常启动时背景 dataURL 已从本地存储秒取，无闪底色；
  // 只有首次设置背景的那一次会先出界面、背景随后淡入
  void restoreBg();

  applyTheme();
  applyLocale();
  loadAccounts(); // 账户列表由 Rust 提供（浏览器环境静默跳过）
  createApp(App).directive("tip", vTip).mount("#app");
}

bootstrap();
