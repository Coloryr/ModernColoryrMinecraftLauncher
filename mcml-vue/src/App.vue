<script setup lang="ts">
// 应用外壳：负责主题应用与窗口路由。
// 单窗口模式：currentKind 在应用内切换（history 同步）；
// 多窗口模式：本页面即当前窗口（?window=xxx），功能窗口为新标签页/真实窗口。
// KeepAlive：切换窗口时保留各窗口组件的状态（如主窗口的实例列表、选中项）。
import { computed, onMounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { applyTheme } from "./lib/theme";
import { applyLocale } from "./lib/i18n";
import { closeWindow, currentKind, isTauri } from "./windows/windowManager";
import { MAIN_WINDOW_UUID, saveWindowState } from "./lib/guiConfig";
import MainWindow from "./windows/main/MainWindow.vue";
import SettingsWindow from "./windows/settings/SettingsWindow.vue";
import StatsWindow from "./windows/stats/StatsWindow.vue";
import SkinWindow from "./windows/skin/SkinWindow.vue";
import HelpWindow from "./windows/help/HelpWindow.vue";
import ResourceWindow from "./windows/resource/ResourceWindow.vue";
import AccountWindow from "./windows/account/AccountWindow.vue";
import AddInstanceWindow from "./windows/add/AddInstanceWindow.vue";
import AppToast from "./components/ui/AppToast.vue";

applyTheme();
applyLocale();

const windowMap = {
  main: MainWindow,
  settings: SettingsWindow,
  stats: StatsWindow,
  skin: SkinWindow,
  help: HelpWindow,
  resource: ResourceWindow,
  account: AccountWindow,
  add: AddInstanceWindow,
};

const currentWindow = computed(() => windowMap[currentKind.value]);

// 主窗口：移动 / 缩放时把几何保存到 windows.json（固定 uuid），下次启动由 Rust 恢复
onMounted(() => {
  if (!isTauri()) return;
  let timer: number | undefined;
  const record = async () => {
    try {
      const win = getCurrentWindow();
      if (win.label !== "main") return;
      const [pos, size] = await Promise.all([win.outerPosition(), win.innerSize()]);
      saveWindowState({
        uuid: MAIN_WINDOW_UUID,
        label: "main",
        x: pos.x,
        y: pos.y,
        width: size.width,
        height: size.height,
      });
    } catch {
      /* 忽略 */
    }
  };
  const debounced = () => {
    clearTimeout(timer);
    timer = setTimeout(record, 400);
  };
  const win = getCurrentWindow();
  win.onResized(debounced);
  win.onMoved(debounced);
});
</script>

<template>
  <KeepAlive>
    <component :is="currentWindow" @close="closeWindow" />
  </KeepAlive>
  <!-- 全局轻提示（所有窗口统一） -->
  <AppToast />
</template>
