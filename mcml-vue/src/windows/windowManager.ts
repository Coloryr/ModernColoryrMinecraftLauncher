// 窗口管理器：负责窗口打开 / 关闭、单窗口 / 多窗口模式切换
//
// 多窗口模式：
// - Tauri 环境：用官方 JS API new WebviewWindow() 创建真实窗口
//   （避免从 Rust 同步命令创建——Windows 上会阻塞主线程导致应用冻结）
// - 浏览器环境：用新标签页模拟独立窗口
// 单窗口模式：应用内页面切换（history 同步，可返回）

import { ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { isWindowKind, type WindowKind, WINDOW_REGISTRY } from "./registry";

const MODE_KEY = "mcml.windowMode";
const WIN_PARAM = "window";

/** 是否多窗口模式（默认多窗口） */
export const multiWindow = ref(localStorage.getItem(MODE_KEY) !== "single");

export function setMultiWindow(v: boolean) {
  multiWindow.value = v;
  localStorage.setItem(MODE_KEY, v ? "multi" : "single");
}

/** 是否运行在 Tauri 环境 */
export function isTauri(): boolean {
  return typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
}

/** 当前窗口标识 */
export const currentKind = ref<WindowKind>(resolveKind());

function resolveKind(): WindowKind {
  if (isTauri()) {
    try {
      // Tauri：用窗口标签识别（mcml-settings → settings，main → main）
      const label = getCurrentWindow().label;
      const kind = label.replace(/^mcml-/, "");
      if (isWindowKind(kind)) return kind;
    } catch (e) {
      console.error("[windowManager] 读取窗口标签失败", e);
    }
  }
  // 浏览器：从 URL 查询参数识别
  const k = new URLSearchParams(window.location.search).get(WIN_PARAM);
  return isWindowKind(k) ? k : "main";
}

function urlFor(kind: WindowKind): string {
  const url = new URL(window.location.href);
  if (kind === "main") {
    url.searchParams.delete(WIN_PARAM);
  } else {
    url.searchParams.set(WIN_PARAM, kind);
  }
  return url.toString();
}

export function kindFromUrl(): WindowKind {
  const k = new URLSearchParams(window.location.search).get(WIN_PARAM);
  return isWindowKind(k) ? k : "main";
}

/** 打开一个窗口（功能入口等调用） */
export function openWindow(kind: WindowKind) {
  console.log("[windowManager] openWindow", kind, {
    multiWindow: multiWindow.value,
    tauri: isTauri(),
  });

  if (isTauri()) {
    // Tauri 环境始终创建真实 WebviewWindow（多窗口）
    // 注意：从 Rust 命令里同步创建窗口在 Windows 上可能阻塞主线程，
    // 导致新窗口白屏、整个应用卡死。这里改用官方 JS API new WebviewWindow() 创建（标准路径）。
    const label = `mcml-${kind}`;
    WebviewWindow.getByLabel(label).then((existing) => {
      if (existing) {
        // 窗口已存在则聚焦
        existing.setFocus();
        return;
      }
      const info = WINDOW_REGISTRY.find((w) => w.kind === kind);
      const win = new WebviewWindow(label, {
        url: "index.html",
        title: info?.title ?? kind,
        width: info?.width ?? 1100,
        height: info?.height ?? 720,
        resizable: true,
      });
      win.once("tauri://created", () => {
        console.log("[windowManager] 已创建窗口", label);
      });
      win.once("tauri://error", (e) => {
        console.error("[windowManager] 创建窗口失败，回退到应用内切换", label, e);
        // 失败时回退：应用内切换，保证功能可用
        currentKind.value = kind;
        window.history.pushState({}, "", urlFor(kind));
      });
    });
  } else if (multiWindow.value) {
    // 浏览器：新标签页模拟
    window.open(urlFor(kind), kind, "noopener");
  } else {
    currentKind.value = kind;
    window.history.pushState({}, "", urlFor(kind));
  }
}

/** 关闭当前窗口（单窗口模式的返回按钮触发：切回主页面） */
export function closeWindow() {
  if (isTauri()) {
    // Tauri：关闭当前 WebviewWindow（多窗口模式子窗口用系统原生按钮关闭）
    getCurrentWindow().close();
  } else if (multiWindow.value) {
    // 浏览器多窗口：由脚本打开的标签页允许 window.close()
    window.close();
  } else {
    // 单窗口模式：应用内切回主页面
    currentKind.value = "main";
    window.history.pushState({}, "", urlFor("main"));
  }
}

// 浏览器前进 / 后退同步（单窗口模式）
window.addEventListener("popstate", () => {
  currentKind.value = kindFromUrl();
});
