// GUI 状态（由 Rust 提供）：gui_config.json（界面状态）+ windows.json（窗口几何）
// 主窗口使用固定 uuid，与 Rust windows/state.rs 一致。
import { invoke } from "@tauri-apps/api/core";

export interface GuiConfig {
  /** dark / light */
  theme: string;
  /** zh-CN / en-US */
  locale: string;
  /** multi / single */
  windowMode: string;
  /** left / right */
  sidebarSide: string;
  sidebarCollapsed: boolean;
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

/** 主窗口固定 uuid（与 Rust 一致） */
export const MAIN_WINDOW_UUID = "8f6b1c2e-3d4a-4e5b-9c6d-7e8f9a0b1c2d";

/** 读取 GUI 状态；非 Tauri 环境返回 null（浏览器回退 localStorage） */
export async function loadGuiConfig(): Promise<GuiConfig | null> {
  try {
    return await invoke<GuiConfig>("get_gui_config");
  } catch {
    return null;
  }
}

/** 合并保存 GUI 状态到 gui_config.json */
export async function saveGuiConfig(patch: Partial<GuiConfig>): Promise<void> {
  try {
    const cur = (await loadGuiConfig()) ?? defaultConfig();
    await invoke("save_gui_config", { config: { ...cur, ...patch } });
  } catch {
    /* 浏览器环境忽略 */
  }
}

function defaultConfig(): GuiConfig {
  return {
    theme: localStorage.getItem("mcml.theme") === "light" ? "light" : "dark",
    locale: localStorage.getItem("mcml.locale") === "en-US" ? "en-US" : "zh-CN",
    windowMode: localStorage.getItem("mcml.windowMode") === "single" ? "single" : "multi",
    sidebarSide: localStorage.getItem("mcml.sidebarSide") === "right" ? "right" : "left",
    sidebarCollapsed: localStorage.getItem("mcml.sidebarCollapsed") === "1",
  };
}

/** 保存窗口几何到 windows.json（按 uuid 更新） */
export async function saveWindowState(state: WindowState): Promise<void> {
  try {
    await invoke("save_window_state", { state });
  } catch {
    /* 浏览器环境忽略 */
  }
}
