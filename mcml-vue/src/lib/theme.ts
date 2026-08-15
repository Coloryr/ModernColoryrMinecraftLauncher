// 主题管理：暗色 / 亮色
import { ref } from "vue";

export type Theme = "dark" | "light";

const THEME_KEY = "mcml.theme";

const stored = localStorage.getItem(THEME_KEY) as Theme | null;
export const theme = ref<Theme>(stored === "light" ? "light" : "dark");

export function applyTheme() {
  document.documentElement.dataset.theme = theme.value;
}

export function toggleTheme() {
  theme.value = theme.value === "dark" ? "light" : "dark";
  localStorage.setItem(THEME_KEY, theme.value);
  applyTheme();
}
