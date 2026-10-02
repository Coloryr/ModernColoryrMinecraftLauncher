// 窗口管理器：负责窗口打开 / 关闭、单窗口 / 多窗口模式切换
//
// 多窗口模式：
// - Tauri 环境：统一调用 Rust 窗口管理器（src-tauri 的 windows/mod.rs）的
//   open_window / close_window 命令创建 / 关闭真实窗口（创建/关闭逻辑都在 Rust 侧）；
//   命令失败时回退到官方 JS API new WebviewWindow()，再失败回退应用内切换。
// - 浏览器环境：用新标签页模拟独立窗口
// 单窗口模式：应用内页面切换（history 同步，可返回）

import { ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { commands } from "../lib/bindings";
import { saveGuiConfig } from "../lib/guiConfig";
import { isWindowKind, type WindowKind, WINDOW_REGISTRY } from "./registry";

const MODE_KEY = "mml.windowMode";
const WIN_PARAM = "window";
const UUID_PARAM = "uuid";

/** 是否多窗口模式（默认多窗口） */
export const multiWindow = ref(localStorage.getItem(MODE_KEY) !== "Single");

/**
 * 单窗口模式的返回栈：记着"上一层是谁"，关窗时先回上一层
 *
 * 多窗口模式不需要它（每个窗口是独立窗口，关掉自然露出下面那个）。
 * 只记 kind、不持久化——重启后没有可返回的历史。
 */
const backStack = ref<WindowKind[]>([]);

/** 返回栈上限（防御性：正常导航不会超过几个） */
const BACK_STACK_MAX = 16;

/**
 * 单窗口模式下"本次打开带入的参数"
 *
 * 单窗口既没有新窗口、也没有新 URL（Tauri 下连 pushState 都不做），壳层的 focus 事件更是
 * 无从触发，所以目标实例只能这样送达：openWindow 写进来，窗口组件 watch / onActivated 读走。
 */
export const windowParams = ref<{ uuid: string | null }>({ uuid: null });

/**
 * 单窗口模式：下载管理以"悬浮弹窗"出现，而不是把当前页面换掉
 *
 * 它是个工具型窗口——下载在后台跑，用户常常要一边看别的页面一边盯进度，
 * 所以浮在任意页面之上（见 [`openWindow`] 的特例）；有任务时右下角另有入口
 * （`components/DownloadChip.vue`）。
 */
export const downloadPopupOpen = ref(false);

/** 打开下载管理弹窗（单窗口模式） */
export function openDownloadPopup() {
  downloadPopupOpen.value = true;
}

/** 关闭下载管理弹窗（单窗口模式；任务不受影响，仍在后台跑） */
export function closeDownloadPopup() {
  downloadPopupOpen.value = false;
}

/**
 * 单窗口模式：整合包安装进度以"悬浮弹窗"出现（入口是标题栏上的 ModpackTitleIndicator）
 *
 * 与下载管理同一个路子：安装跑在后台，用户可能已经离开"下载整合包"那一页，
 * 所以进度不能只挂在那个页面上。弹窗只影响"看不看得到进度"，关掉它不影响安装。
 */
export const modpackPopupOpen = ref(false);

/** 打开整合包安装进度弹窗 */
export function openModpackPopup() {
  modpackPopupOpen.value = true;
}

/** 关闭整合包安装进度弹窗（安装照常继续） */
export function closeModpackPopup() {
  modpackPopupOpen.value = false;
}

/** 目标实例 uuid：单窗口取本次 openWindow 带入的参数，多窗口取本窗口 URL 上的参数 */
export function targetUuid(): string | null {
  return multiWindow.value ? uuidFromUrl() : windowParams.value.uuid;
}

export function setMultiWindow(v: boolean): Promise<void> {
  multiWindow.value = v;
  // 换了模式，之前的返回栈与带入参数都不再成立
  backStack.value = [];
  windowParams.value = { uuid: null };
  localStorage.setItem(MODE_KEY, v ? "Multi" : "Single");
  // 返回保存的 Promise：要紧接着重启进程的调用方必须等这次写入真的落盘
  // （重启会立刻刷盘并退出，写请求还在路上就丢了 —— 见 useSettingsUi 的窗口模式切换）
  return saveGuiConfig({ windowMode: v ? "Multi" : "Single" });
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
      // Tauri：用窗口标签识别（mml-settings → settings，main → main）
      const label = getCurrentWindow().label;
      const kind = label.replace(/^mml-/, "");
      if (isWindowKind(kind)) return kind;
    } catch (e) {
      console.error("[windowManager] 读取窗口标签失败", e);
    }
  }
  // 浏览器：从 URL 查询参数识别
  const k = new URLSearchParams(window.location.search).get(WIN_PARAM);
  return isWindowKind(k) ? k : "main";
}

function urlFor(kind: WindowKind, params?: { uuid?: string }): string {
  const url = new URL(window.location.href);
  if (kind === "main") {
    url.searchParams.delete(WIN_PARAM);
  } else {
    url.searchParams.set(WIN_PARAM, kind);
  }
  if (params?.uuid) {
    url.searchParams.set(UUID_PARAM, params.uuid);
  } else {
    url.searchParams.delete(UUID_PARAM);
  }
  return url.toString();
}

export function kindFromUrl(): WindowKind {
  const k = new URLSearchParams(window.location.search).get(WIN_PARAM);
  return isWindowKind(k) ? k : "main";
}

/** 从 URL 读取目标实例 uuid（日志窗口等带参窗口用），没有返回 null */
export function uuidFromUrl(): string | null {
  return new URLSearchParams(window.location.search).get(UUID_PARAM);
}

/** 打开一个窗口（功能入口等调用）
 *
 * - `params.uuid`：目标实例 uuid（游戏日志窗口用，定位要查看的实例）
 */
export function openWindow(kind: WindowKind, params?: { uuid?: string }) {
  console.log("[windowManager] openWindow", kind, {
    multiWindow: multiWindow.value,
    tauri: isTauri(),
    ...params,
  });

  // 单窗口模式（浏览器 / Tauri 一致）：应用内页面切换
  if (!multiWindow.value) {
    // 下载管理是工具型窗口：浮在当前页面之上，不把页面换掉（关掉它也不会"返回"到哪去）
    if (kind === "download") {
      openDownloadPopup();
      return;
    }
    // 已经停在这个窗口（例如就在日志页又右键了另一个实例）：不压栈，只把新参数送过去
    if (currentKind.value !== kind) {
      backStack.value.push(currentKind.value);
      if (backStack.value.length > BACK_STACK_MAX) backStack.value.shift();
      currentKind.value = kind;
    }
    // 目标实例只能靠这里送达：单窗口没有新窗口 / 新 URL，也没有壳层的 focus 事件
    windowParams.value = { uuid: params?.uuid ?? null };
    if (!isTauri()) window.history.pushState({}, "", urlFor(kind, params));
    return;
  }

  if (isTauri()) {
    // 统一走 Rust 窗口管理器：创建 / 聚焦在 src-tauri 的 windows/mod.rs 处理。
    // open_window 是 async 命令（不在 Windows 主线程创建窗口，避免冻结）。
    // 命令失败（例如窗口创建被拒）时回退到官方 JS API。
    commands.windows
      .openWindow(kind, params?.uuid ?? null)
      .catch((e) => {
        console.error("[windowManager] Rust 打开窗口失败，回退 JS API", kind, e);
        createViaJs(kind, params);
      });
    return;
  }

  // 浏览器多窗口：新标签页模拟
  window.open(urlFor(kind, params), kind, "noopener");
}

/** 回退路径：用官方 JS API 创建 / 聚焦真实 WebviewWindow */
function createViaJs(kind: WindowKind, params?: { uuid?: string }) {
  const label = `mml-${kind}`;
  WebviewWindow.getByLabel(label).then(async (existing) => {
    if (existing) {
      // 窗口已存在则聚焦（目标实例靠 game-log / log-focus 事件链路自行同步）
      existing.setFocus();
      return;
    }
    // 尺寸从后端注册表取（与 Rust 建窗同源）；后端不可用时给个兜底值
    const sizes = await commands.windows.getWindowSizes().catch(() => []);
    const size = sizes.find((s) => s.kind === kind);
    const info = WINDOW_REGISTRY.find((w) => w.kind === kind);
    const win = new WebviewWindow(label, {
      url: urlFor(kind, params).replace(window.location.origin, ""),
      title: info?.title ?? kind,
      width: size?.width ?? 900,
      height: size?.height ?? 620,
      resizable: true,
    });
    win.once("tauri://created", () => {
      console.log("[windowManager] 已创建窗口（JS 回退）", label);
    });
    win.once("tauri://error", (e) => {
      console.error("[windowManager] 创建窗口失败，回退到应用内切换", label, e);
      // 失败时回退：应用内切换，保证功能可用
      currentKind.value = kind;
      window.history.pushState({}, "", urlFor(kind, params));
    });
  });
}

/**
 * 关掉当前窗口：标题栏 ✕ 的动作（真的关窗）
 *
 * - 多窗口模式：关掉当前这个功能窗口
 * - 单窗口模式：整个应用只有主窗口一个真实窗口，等于退出启动器
 *   （"回上一层"是另一回事，走 closeWindow —— 标题栏的 ‹ 箭头与各窗口的"取消"都走它）
 */
export function quitWindow() {
  if (!isTauri()) {
    // 浏览器预览：非脚本打开的标签页关不掉，失败就作罢
    window.close();
    return;
  }
  // 单窗口模式只有主窗口是真的：那里 kind 是"当前页"而不是窗口标签，拿它去找窗口会 not found
  const target = multiWindow.value ? currentKind.value : "main";
  commands.windows.closeWindow(target).catch((e) => {
    console.error("[windowManager] 关闭窗口失败，回退 JS API", target, e);
    getCurrentWindow().close();
  });
}

/**
 * 收起当前页面 / 窗口（各窗口的「取消」按钮、标题栏的 ‹ 箭头触发）
 *
 * - 多窗口模式：每个功能就是一个独立窗口，收起 = 关掉它
 * - 单窗口模式：应用内页面切换，弹返回栈回上一层（没有上一层就回主页面）
 *
 * ✕ 不走这里，走 quitWindow：单窗口模式下 ✕ 是真的退出应用，不是退一层
 */
export function closeWindow() {
  const kind = currentKind.value;

  // 单窗口模式：先回上一层（谁打开的），没有上一层再回主页面
  if (!multiWindow.value) {
    const prev = backStack.value.pop();
    if (prev && prev !== kind) {
      goTo(prev);
      return;
    }
    if (kind !== "main") {
      goTo("main");
      return;
    }
    // 已经在主页面：主页面就是应用本身，关它 = 退出应用。
    // 不能走上面"切回主页面"的分支——那样标题栏的 ✕ 会变成空操作（启动画面为此单独绕过过）
    if (isTauri()) {
      commands.windows.closeWindow("main").catch((e) => {
        console.error("[windowManager] 关闭主窗口失败，回退 JS API", e);
        getCurrentWindow().close();
      });
      return;
    }
    // 浏览器预览：非脚本打开的标签页关不掉，失败就作罢
    window.close();
    return;
  }

  if (isTauri()) {
    // 多窗口：走 Rust 窗口管理器关闭当前真实窗口（kind → 标签映射在 windows/mod.rs）。
    // 命令失败时回退到官方 JS API 关闭当前窗口。
    commands.windows.closeWindow(kind).catch((e) => {
      console.error("[windowManager] Rust 关闭窗口失败，回退 JS API", kind, e);
      getCurrentWindow().close();
    });
  } else {
    // 浏览器多窗口：由脚本打开的标签页允许 window.close()
    window.close();
  }
}

/** 单窗口模式的应用内跳转（同步返回栈之外的显示状态与 URL） */
function goTo(kind: WindowKind) {
  currentKind.value = kind;
  // 返回后不再带着上一次的目标实例，避免窗口"记得"不该记的东西
  windowParams.value = { uuid: null };
  if (!isTauri()) window.history.pushState({}, "", urlFor(kind));
}

// 浏览器前进 / 后退同步（单窗口模式）
window.addEventListener("popstate", () => {
  // 浏览器历史是另一套栈，和应用内返回栈没法对齐：一后退就清掉，免得两边打架
  backStack.value = [];
  currentKind.value = kindFromUrl();
});
