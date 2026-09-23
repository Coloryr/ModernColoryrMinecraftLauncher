// 界面设置（侧栏位置 / 收起状态 / 列表显示模式），持久化到 localStorage + gui_config.json
import { ref } from "vue";
import { normalizeViewMode, saveGuiConfig, type SidebarSide, type ViewMode } from "./guiConfig";
export type { SidebarSide, ViewMode } from "./guiConfig";

const SIDE_KEY = "mml.sidebarSide";
const COLLAPSE_KEY = "mml.sidebarCollapsed";
const VIEW_KEY = "mml.viewMode";
const SELECT_KEY = "mml.selectedInstance";

export const sidebarSide = ref<SidebarSide>(
  localStorage.getItem(SIDE_KEY) === "Right" ? "Right" : "Left",
);

// 侧栏是否收起：默认收起（只有显式存过 "0" 才是展开）
export const sidebarCollapsed = ref(localStorage.getItem(COLLAPSE_KEY) !== "0");

// 实例列表显示模式：默认列表；用户改过后用用户的值
export const viewMode = ref<ViewMode>(normalizeViewMode(localStorage.getItem(VIEW_KEY)));

export function setSidebarSide(side: SidebarSide) {
  sidebarSide.value = side;
  localStorage.setItem(SIDE_KEY, side);
  saveGuiConfig({ mainWindow: { sidebarSide: side } });
}

export function setSidebarCollapsed(v: boolean) {
  sidebarCollapsed.value = v;
  localStorage.setItem(COLLAPSE_KEY, v ? "1" : "0");
  saveGuiConfig({ mainWindow: { sidebarCollapsed: v } });
}

export function setViewMode(v: ViewMode) {
  viewMode.value = v;
  localStorage.setItem(VIEW_KEY, v);
  saveGuiConfig({ mainWindow: { viewMode: v } });
}

/** 当前选中实例 uuid（空串 = 未选中） */
export const selectedInstance = ref(localStorage.getItem(SELECT_KEY) ?? "");

function mirrorSelected(uuid: string) {
  if (uuid) localStorage.setItem(SELECT_KEY, uuid);
  else localStorage.removeItem(SELECT_KEY);
}

/** 启动引导：从 gui_config 恢复选中实例（只同步内存 + localStorage，不回写配置文件） */
export function restoreSelectedInstance(uuid: string) {
  selectedInstance.value = uuid;
  mirrorSelected(uuid);
}

/** 选中实例变化：写 localStorage + gui_config.json（uuid 未变则直接返回，避免无谓的 IPC 与落盘） */
export function setSelectedInstance(uuid: string) {
  if (selectedInstance.value === uuid) return;
  selectedInstance.value = uuid;
  mirrorSelected(uuid);
  saveGuiConfig({ mainWindow: { selectedInstance: uuid } });
}
