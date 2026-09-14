// 界面设置（侧栏位置 / 收起状态），持久化到 localStorage + gui_config.json
import { ref } from "vue";
import { saveGuiConfig, type SidebarSide } from "./guiConfig";
export type { SidebarSide } from "./guiConfig";

const SIDE_KEY = "mcml.sidebarSide";
const COLLAPSE_KEY = "mcml.sidebarCollapsed";

export const sidebarSide = ref<SidebarSide>(
  localStorage.getItem(SIDE_KEY) === "Right" ? "Right" : "Left",
);

// 侧栏是否收起：默认收起（只有显式存过 "0" 才是展开）
export const sidebarCollapsed = ref(localStorage.getItem(COLLAPSE_KEY) !== "0");

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
