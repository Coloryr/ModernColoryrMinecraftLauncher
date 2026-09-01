// GUI 状态（由 Rust 提供）：gui_config.json（界面状态）+ window_save.json（窗口几何）
// 主窗口使用固定 uuid，与 Rust window_manager.rs 一致。
// IPC wire 使用 TS 命名（camelCase），经 Rust dtos::GuiConfigDto 转换；
// 磁盘 gui_config.json 使用 Rust 命名（snake_case），前端不直接接触。
// 枚举值即 Rust 变体名：theme/windowMode/sidebarSide 为 PascalCase，locale 为 zh_cn/en_us。
import { invoke } from "@tauri-apps/api/core";

export type Theme = "Dark" | "Light";
/** 与 core Lang 变体同名：zh_cn / en_us */
export type Locale = "zh_cn" | "en_us";
export type WindowMode = "Multi" | "Single";
export type SidebarSide = "Left" | "Right";

/** 主窗口配置（对应 Rust MainWindowConfig，wire 为 mainWindow） */
export interface MainWindowConfig {
  /** Left / Right */
  sidebarSide: SidebarSide;
  /** 是否收起侧栏 */
  sidebarCollapsed: boolean;
}

export interface GuiConfig {
  /** Dark / Light */
  theme: Theme;
  /** zh_cn / en_us */
  locale: Locale;
  /** Multi / Single */
  windowMode: WindowMode;
  mainWindow: MainWindowConfig;
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
    return await invoke<GuiConfig>(WindowGetGuiConfig);
  } catch {
    return null;
  }
}

/** GUI 状态局部更新（mainWindow 内部可只给一个字段，saveGuiConfig 会深合并） */
export interface GuiConfigPatch {
  theme?: Theme;
  locale?: Locale;
  windowMode?: WindowMode;
  mainWindow?: Partial<MainWindowConfig>;
}

/** 合并保存 GUI 状态到 gui_config.json（mainWindow 内部做深合并，避免互相覆盖） */
export async function saveGuiConfig(patch: GuiConfigPatch): Promise<void> {
  try {
    const cur = (await loadGuiConfig()) ?? defaultConfig();
    await invoke(WindowSaveGuiConfig, {
      config: {
        ...cur,
        ...patch,
        mainWindow: { ...cur.mainWindow, ...patch.mainWindow },
      },
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
      sidebarCollapsed: localStorage.getItem("mcml.sidebarCollapsed") === "1",
    },
  };
}
