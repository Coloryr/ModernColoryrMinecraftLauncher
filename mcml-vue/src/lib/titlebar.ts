// 自绘标题栏：平台判定 + 拖动 / 最大化助手
//
// 两套样式：`windows`（右端三个方按钮）/ `macos`（左端红黄绿圆点）。
// 4 月 1 日对调两套样式（愚人节彩蛋）。
//
// 窗口动作都走应用自己的 Rust 命令（见 window_manager.rs 末尾），不走 JS 的
// `getCurrentWindow()`，省得往 capabilities/default.json 加窗口权限。
// 唯一的例外是「关闭」——它复用 windowManager 的 closeWindow()，那条链路才会跑
// 关闭保护（下载中 / 查询中拒关）与几何保存。

import { commands } from "./bindings";
import { isTauri } from "../windows/windowManager";

export type OsKind = "windows" | "linux" | "macos";
export type TitleBarStyle = "windows" | "macos";

/** 运行平台。用 UA 判定——Tauri 与浏览器预览（dev-frontend.bat）都可用，
 *  不必引入 tauri-plugin-os 依赖，也不必加一个异步命令 */
export function detectOs(): OsKind {
  const ua = navigator.userAgent;
  if (/Macintosh|Mac OS X/.test(ua)) {
    return "macos";
  }
  if (/Linux|X11/.test(ua)) {
    return "linux";
  }
  return "windows";
}

/** 是否 4 月 1 日（本地时间） */
export function isAprilFools(now = new Date()): boolean {
  return now.getMonth() === 3 && now.getDate() === 1;
}

/** 本次运行使用的标题栏样式：启动时定一次，会话中不随其它状态变化 */
export const titleBarStyle: TitleBarStyle = (() => {
  const isMac = detectOs() === "macos";
  return isAprilFools() ? (isMac ? "windows" : "macos") : isMac ? "macos" : "windows";
})();

/** 标题栏上按下时不应触发拖动 / 最大化的元素 */
const INTERACTIVE = "button, a, input, select, textarea, label, [data-no-drag]";

/** 判定双击的最大间隔 */
const DOUBLE_CLICK_MS = 400;

function isInteractive(target: EventTarget | null): boolean {
  return target instanceof HTMLElement && target.closest(INTERACTIVE) !== null;
}

let lastDownAt = 0;
let lastDownTarget: EventTarget | null = null;

/**
 * 标题栏按下：单击拖动窗口，双击最大化 / 还原
 *
 * 双击在这里自己判——`start_dragging()` 会把指针交给系统，之后的 `dblclick`
 * 事件不一定能到，所以不能依赖 `@dblclick`。
 */
export function onTitleBarPointerDown(e: PointerEvent) {
  if (!isTauri() || e.button !== 0 || isInteractive(e.target)) {
    return;
  }

  const now = Date.now();
  if (now - lastDownAt < DOUBLE_CLICK_MS && e.target === lastDownTarget) {
    // 双击：只在这一次里最大化，且不再起拖拽
    lastDownAt = 0;
    lastDownTarget = null;
    commands.windowManager.toggleMaximize().catch(() => {});
    return;
  }

  lastDownAt = now;
  lastDownTarget = e.target;
  commands.windowManager.startDragging().catch(() => {});
}

/** 最小化当前窗口 */
export function minimizeWindow() {
  if (!isTauri()) {
    return;
  }
  commands.windowManager.minimize().catch(() => {});
}

/** 最大化 / 还原当前窗口，返回新状态（非 Tauri 环境返回 null） */
export async function toggleMaximizeWindow(): Promise<boolean | null> {
  if (!isTauri()) {
    return null;
  }
  try {
    return await commands.windowManager.toggleMaximize();
  } catch {
    return null;
  }
}

/** 当前是否最大化 */
export async function isWindowMaximized(): Promise<boolean> {
  if (!isTauri()) {
    return false;
  }
  try {
    return await commands.windowManager.isMaximized();
  } catch {
    return false;
  }
}
