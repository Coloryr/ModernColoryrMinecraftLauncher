// GUI 状态（由 Rust 提供）：gui_config.json（界面状态）+ window_save.json（窗口几何）
// 主窗口使用固定 uuid，与 Rust window_manager.rs 一致。
// IPC wire 使用 TS 命名（camelCase），经 Rust dtos::GuiConfigDto 转换；
// 磁盘 gui_config.json 使用 Rust 命名（snake_case），前端不直接接触。
// 枚举值即 Rust 变体名：theme/windowMode/sidebarSide 为 PascalCase，locale 为 zh_cn/en_us。
import { commands } from "./bindings";

export type Theme = "Dark" | "Light" | "System";
/** 与 core Lang 变体同名：zh_cn / en_us */
export type Locale = "zh_cn" | "en_us";
export type WindowMode = "Multi" | "Single";
export type SidebarSide = "Left" | "Right";
/** 实例列表显示模式（默认 list，用户改过后用用户的值） */
export type ViewMode = "list" | "group" | "grid";
/** 皮肤显示模式（账户界面皮肤预览形态，枚举值即 Rust 变体名） */
export type SkinDisplay = "Skin2DA" | "Skin2DB" | "Skin3D";

/** 把任意值规范成合法的皮肤显示模式（非法 / 缺省 → Skin2DA） */
export function normalizeSkinDisplay(value: string | null | undefined): SkinDisplay {
  return value === "Skin2DB" || value === "Skin3D" ? value : "Skin2DA";
}

/** 把任意值规范成合法的显示模式（非法 / 缺省 → list） */
export function normalizeViewMode(value: string | null | undefined): ViewMode {
  return value === "group" || value === "grid" ? value : "list";
}

/** 把任意值规范成合法的头像类型（非法 / 缺省 → Head2DA） */
export function normalizeHeadType(value: string | null | undefined): HeadType {
  return value === "Head3DA" || value === "Head3DB" || value === "Head3DC" || value === "Head2DB" ? value : "Head2DA";
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
export type HeadType = "Head2DA" | "Head3DA" | "Head3DB" | "Head3DC" | "Head2DB";

/** 头像设置（对应 Rust HeadConfig，wire 为 head） */
export interface HeadConfig {
  /** Head2DA / Head3DA / Head3DB / Head3DC / Head2DB */
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
  /** Dark / Light / System */
  theme: Theme;
  /** zh_cn / en_us */
  locale: Locale;
  /** Multi / Single */
  windowMode: WindowMode;
  mainWindow: MainWindowConfig;
  head: HeadConfig;
  collect: CollectConfig;
  /** 界面字体族名（空串 = 默认字体栈） */
  font: string;
  /** 皮肤显示模式：Skin2DA / Skin2DB / Skin3D */
  skinDisplay: SkinDisplay;
  /** 背景图来源（文件路径 / 网址，空串 = 无背景图） */
  bgSource: string;
  /** 背景图不透明度（%，5–100） */
  bgOpacity: number;
  /** 背景图模糊（px，0–40） */
  bgBlur: number;
  /** 背景图原始分辨率（%，10–100） */
  bgNativeSize: number;
}

export interface WindowState {
  uuid: string;
  /** 窗口标签（main / mml-settings …） */
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
  font?: string;
  skinDisplay?: SkinDisplay;
  bgSource?: string;
  bgOpacity?: number;
  bgBlur?: number;
  bgNativeSize?: number;
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
  const stored = localStorage.getItem("mml.theme");
  return {
    // 没有显式选择时跟随系统深浅色（与 theme.ts 的首绘兜底一致）
    theme:
      stored === "Light" || stored === "Dark" || stored === "System"
        ? stored
        : matchMedia("(prefers-color-scheme: dark)").matches
          ? "Dark"
          : "Light",
    locale: localStorage.getItem("mml.locale") === "en_us" ? "en_us" : "zh_cn",
    windowMode: localStorage.getItem("mml.windowMode") === "Single" ? "Single" : "Multi",
    mainWindow: {
      sidebarSide: localStorage.getItem("mml.sidebarSide") === "Right" ? "Right" : "Left",
      sidebarCollapsed: localStorage.getItem("mml.sidebarCollapsed") !== "0",
      viewMode: normalizeViewMode(localStorage.getItem("mml.viewMode")),
      selectedInstance: localStorage.getItem("mml.selectedInstance") ?? "",
    },
    head: {
      headType: normalizeHeadType(localStorage.getItem("mml.headType")),
      x: Number(localStorage.getItem("mml.headX")) || 0,
      y: Number(localStorage.getItem("mml.headY")) || 0,
    },
    collect: {
      modpack: localStorage.getItem("mml.collect.modpack") !== "0",
      showMod: localStorage.getItem("mml.collect.showMod") !== "0",
      resourcePack: localStorage.getItem("mml.collect.resourcePack") !== "0",
      shaderpack: localStorage.getItem("mml.collect.shaderpack") !== "0",
    },
    font: localStorage.getItem("mml.font") ?? "",
    skinDisplay: normalizeSkinDisplay(localStorage.getItem("mml.skinDisplay")),
    bgSource: "",
    bgOpacity: Number(localStorage.getItem("mml.bgOpacity")) || 100,
    bgBlur: Number(localStorage.getItem("mml.bgBlur")) || 0,
    bgNativeSize: 100,
  };
}
