// 国际化：zh-CN / en-US
// 用法：模板中 {{ t("launch.play") }}；带参数 {{ t("detail.launchCount", { count: 3 }) }}
import { ref } from "vue";
import zhCN from "./locales/zh-CN";
import enUS from "./locales/en-US";

export type Locale = "zh-CN" | "en-US";

const LOCALE_KEY = "mcml.locale";
const stored = localStorage.getItem(LOCALE_KEY);

export const locale = ref<Locale>(stored === "en-US" ? "en-US" : "zh-CN");

const messages: Record<Locale, Record<string, string>> = {
  "zh-CN": zhCN as Record<string, string>,
  "en-US": enUS as Record<string, string>,
};

export function setLocale(l: Locale) {
  locale.value = l;
  localStorage.setItem(LOCALE_KEY, l);
  document.documentElement.lang = l;
}

export function applyLocale() {
  document.documentElement.lang = locale.value;
}

// 跨窗口同步：某个窗口改了语言后，其它已打开的窗口实时生效
window.addEventListener("storage", (e) => {
  if (e.key === LOCALE_KEY && (e.newValue === "zh-CN" || e.newValue === "en-US")) {
    locale.value = e.newValue;
    document.documentElement.lang = e.newValue;
  }
});

/** 翻译：key 用点号分隔的扁平键，{param} 插值 */
export function t(key: string, params?: Record<string, string | number>): string {
  const msg = messages[locale.value][key];
  if (msg === undefined) return key;
  if (!params) return msg;
  return msg.replace(/\{(\w+)\}/g, (_, k: string) =>
    params[k] !== undefined ? String(params[k]) : `{${k}}`,
  );
}

export function useI18n() {
  return { t, locale, setLocale };
}
