// GUI 状态（由 Rust 提供）：gui_config.json（界面状态）+ window_save.json（窗口几何）
// 主窗口使用固定 uuid，与 Rust window_manager.rs 一致。
// IPC wire 使用 TS 命名（camelCase），经 Rust dtos::GuiConfigDto 转换；
// 磁盘 gui_config.json 使用 Rust 命名（snake_case），前端不直接接触。
// 枚举值即 Rust 变体名：theme/windowMode/sidebarSide 为 PascalCase，locale 为 zh_cn/en_us。
import { commands } from "./bindings";

export type Theme = "Dark" | "Light";
/** 与 core Lang 变体同名：zh_cn / en_us */
export type Locale = "zh_cn" | "en_us";
export type WindowMode = "Multi" | "Single";
export type SidebarSide = "Left" | "Right";
/** 实例列表显示模式（默认 list，用户改过后用用户的值） */
export type ViewMode = "list" | "group" | "grid";

/** 把任意值规范成合法的显示模式（非法 / 缺省 → list） */
export function normalizeViewMode(value: string | null | undefined): ViewMode {
  return value === "group" || value === "grid" ? value : "list";
}

/** 把任意值规范成合法的头像类型（非法 / 缺省 → Head2DA） */
export function normalizeHeadType(value: string | null | undefined): HeadType {
  return value === "Head3DA" || value === "Head3DB" || value === "Head2DB" ? value : "Head2DA";
}

/** 主窗口配置（对应 Rust MainWindowConfig，wire 为 mainWindow） */
export interface MainWindowConfig {
  /** Left / Right */
  sidebarSide: SidebarSide;
  /** 是否收起侧栏 */
  sidebarCollapsed: boolean;
  /** list / group / grid */
  viewMode: ViewMode;
  /** 当前选中实例 uuid（空串 = 未选中） */
  selectedInstance: string;
}

/** 头像类型（枚举值即 Rust 变体名） */
export type HeadType = "Head2DA" | "Head3DA" | "Head3DB" | "Head2DB";

/** 头像设置（对应 Rust HeadConfig，wire 为 head） */
export interface HeadConfig {
  /** Head2DA / Head3DA / Head3DB / Head2DB */
  headType: HeadType;
  /** 3D 旋转 X */
  x: number;
  /** 3D 旋转 Y */
  y: number;
}

/** 收藏窗口的类型过滤（对应 Rust CollectConfig，wire 为 collect） */
export interface CollectConfig {
  /** 显示整合包 */
  modpack: boolean;
  /** 显示模组 */
  showMod: boolean;
  /** 显示资源包 */
  resourcePack: boolean;
  /** 显示光影包 */
  shaderpack: boolean;
}

export interface GuiConfig {
  /** Dark / Light */
  theme: Theme;
  /** zh_cn / en_us */
  locale: Locale;
  /** Multi / Single */
  windowMode: WindowMode;
  mainWindow: MainWindowConfig;
  head: HeadConfig;
  collect: CollectConfig;
}

export interface WindowState {
  uuid: string;
  /** 窗口标签（main / mcml-settings …） */
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
}

/** 读取 GUI 状态；非 Tauri 环境返回 null（浏览器回退 localStorage） */
export async function loadGuiConfig(): Promise<GuiConfig | null> {
  try {
    return await commands.windows.getGuiConfig();
  } catch {
    return null;
  }
}

/** GUI 状态局部更新（mainWindow / head 内部可只给一个字段，saveGuiConfig 会深合并） */
export interface GuiConfigPatch {
  theme?: Theme;
  locale?: Locale;
  windowMode?: WindowMode;
  mainWindow?: Partial<MainWindowConfig>;
  head?: Partial<HeadConfig>;
  collect?: Partial<CollectConfig>;
}

/** 合并保存 GUI 状态到 gui_config.json（mainWindow 内部做深合并，避免互相覆盖） */
export async function saveGuiConfig(patch: GuiConfigPatch): Promise<void> {
  try {
    const cur = (await loadGuiConfig()) ?? defaultConfig();
    await commands.windows.saveGuiConfig({
      ...cur,
      ...patch,
      mainWindow: { ...cur.mainWindow, ...patch.mainWindow },
      head: { ...cur.head, ...patch.head },
      collect: { ...cur.collect, ...patch.collect },
    });
  } catch {
    /* 浏览器环境忽略 */
  }
}

function defaultConfig(): GuiConfig {
  return {
    theme: localStorage.getItem("mcml.theme") === "Light" ? "Light" : "Dark",
    locale: localStorage.getItem("mcml.locale") === "en_us" ? "en_us" : "zh_cn",
    windowMode: localStorage.getItem("mcml.windowMode") === "Single" ? "Single" : "Multi",
    mainWindow: {
      sidebarSide: localStorage.getItem("mcml.sidebarSide") === "Right" ? "Right" : "Left",
      sidebarCollapsed: localStorage.getItem("mcml.sidebarCollapsed") !== "0",
      viewMode: normalizeViewMode(localStorage.getItem("mcml.viewMode")),
      selectedInstance: localStorage.getItem("mcml.selectedInstance") ?? "",
    },
    head: {
      headType: normalizeHeadType(localStorage.getItem("mcml.headType")),
      x: Number(localStorage.getItem("mcml.headX")) || 0,
      y: Number(localStorage.getItem("mcml.headY")) || 0,
    },
    collect: {
      modpack: localStorage.getItem("mcml.collect.modpack") !== "0",
      showMod: localStorage.getItem("mcml.collect.showMod") !== "0",
      resourcePack: localStorage.getItem("mcml.collect.resourcePack") !== "0",
      shaderpack: localStorage.getItem("mcml.collect.shaderpack") !== "0",
    },
  };
}
