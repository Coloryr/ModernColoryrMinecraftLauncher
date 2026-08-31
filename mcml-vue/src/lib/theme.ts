// 主题管理：暗色 / 亮色 + 强调色预设（状态持久化到 gui_config.json）
import { ref } from "vue";
import { saveGuiConfig, type Theme } from "./guiConfig";
export type { Theme } from "./guiConfig";

export type AccentId =
  | "blue"
  | "sky"
  | "cyan"
  | "teal"
  | "green"
  | "lime"
  | "yellow"
  | "orange"
  | "red"
  | "pink"
  | "fuchsia"
  | "purple"
  | "indigo";

/** 强调色预设列表（color 用于设置界面色块预览，check 为选中对勾颜色） */
export const ACCENTS: { id: AccentId; color: string; check: string }[] = [
  { id: "blue", color: "#4f8cff", check: "#fff" },
  { id: "sky", color: "#38bdf8", check: "#fff" },
  { id: "cyan", color: "#22d3ee", check: "#fff" },
  { id: "teal", color: "#2dd4bf", check: "#fff" },
  { id: "green", color: "#34d399", check: "#fff" },
  { id: "lime", color: "#a3e635", check: "#1d2229" },
  { id: "yellow", color: "#facc15", check: "#1d2229" },
  { id: "orange", color: "#fb923c", check: "#fff" },
  { id: "red", color: "#f87171", check: "#fff" },
  { id: "pink", color: "#f472b6", check: "#fff" },
  { id: "fuchsia", color: "#e879f9", check: "#fff" },
  { id: "purple", color: "#a78bfa", check: "#fff" },
  { id: "indigo", color: "#818cf8", check: "#fff" },
];

const THEME_KEY = "mcml.theme";
const ACCENT_KEY = "mcml.accent";

const storedTheme = localStorage.getItem(THEME_KEY) as Theme | null;
export const theme = ref<Theme>(storedTheme === "Light" ? "Light" : "Dark");

const storedAccent = localStorage.getItem(ACCENT_KEY) as AccentId | null;
export const accent = ref<AccentId>(
  ACCENTS.some((a) => a.id === storedAccent) ? (storedAccent as AccentId) : "blue",
);

export function applyTheme() {
  const root = document.documentElement;
  root.dataset.theme = theme.value;
  // 强调色：blue 是默认（theme.css 基础块已定义），其余通过 data-accent 覆盖
  if (accent.value === "blue") {
    delete root.dataset.accent;
  } else {
    root.dataset.accent = accent.value;
  }
}

export function toggleTheme() {
  theme.value = theme.value === "Dark" ? "Light" : "Dark";
  localStorage.setItem(THEME_KEY, theme.value);
  applyTheme();
  saveGuiConfig({ theme: theme.value });
}

export function setAccent(a: AccentId) {
  accent.value = a;
  localStorage.setItem(ACCENT_KEY, a);
  applyTheme();
}

// 跨窗口同步：某个窗口改了主题/颜色后，其它已打开的窗口实时生效
window.addEventListener("storage", (e) => {
  if (e.key === THEME_KEY && (e.newValue === "Dark" || e.newValue === "Light")) {
    theme.value = e.newValue;
    applyTheme();
  } else if (e.key === ACCENT_KEY) {
    accent.value = ACCENTS.some((x) => x.id === e.newValue)
      ? (e.newValue as AccentId)
      : "blue";
    applyTheme();
  }
});
