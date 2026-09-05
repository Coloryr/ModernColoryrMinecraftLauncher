<script setup lang="ts">
import { computed, defineAsyncComponent, type Component } from "vue";
import { applyTheme } from "./lib/theme";
import { applyLocale } from "./lib/i18n";
import { closeWindow, currentKind } from "./windows/windowManager";
import type { WindowKind } from "./windows/registry";
import AppToast from "./components/ui/AppToast.vue";

applyTheme();
applyLocale();

// 各窗口按需加载：每个窗口编译为独立 chunk，打开时才拉取对应 JS
const windowMap: Record<WindowKind, Component> = {
  main: defineAsyncComponent(() => import("./windows/main/MainWindow.vue")),
  settings: defineAsyncComponent(() => import("./windows/settings/SettingsWindow.vue")),
  stats: defineAsyncComponent(() => import("./windows/stats/StatsWindow.vue")),
  skin: defineAsyncComponent(() => import("./windows/skin/SkinWindow.vue")),
  help: defineAsyncComponent(() => import("./windows/help/HelpWindow.vue")),
  resource: defineAsyncComponent(() => import("./windows/resource/ResourceWindow.vue")),
  account: defineAsyncComponent(() => import("./windows/account/AccountWindow.vue")),
  add: defineAsyncComponent(() => import("./windows/add/AddInstanceWindow.vue")),
};

const currentWindow = computed(() => windowMap[currentKind.value]);
</script>

<template>
  <KeepAlive>
    <component :is="currentWindow" @close="closeWindow" />
  </KeepAlive>
  <!-- 全局轻提示（所有窗口统一） -->
  <AppToast />
</template>
