// 界面设置（侧栏位置 / 收起状态），持久化到 localStorage
import { ref } from "vue";

export type SidebarSide = "left" | "right";

const SIDE_KEY = "mcml.sidebarSide";
const COLLAPSE_KEY = "mcml.sidebarCollapsed";

export const sidebarSide = ref<SidebarSide>(
  localStorage.getItem(SIDE_KEY) === "right" ? "right" : "left",
);

export const sidebarCollapsed = ref(localStorage.getItem(COLLAPSE_KEY) === "1");

export function setSidebarSide(side: SidebarSide) {
  sidebarSide.value = side;
  localStorage.setItem(SIDE_KEY, side);
}

export function setSidebarCollapsed(v: boolean) {
  sidebarCollapsed.value = v;
  localStorage.setItem(COLLAPSE_KEY, v ? "1" : "0");
}
