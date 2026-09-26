<script setup lang="ts">
// 根组件：按当前窗口标识（currentKind）挂载对应窗口，附全局 Toast
import { computed, defineAsyncComponent, type Component } from "vue";
import { applyTheme } from "./lib/theme";
import { applyLocale } from "./lib/i18n";
import { applyAnimations, bgImage, bgLayerStyle } from "./lib/appearance";
import { closeWindow, currentKind } from "./windows/windowManager";
import type { WindowKind } from "./windows/registry";
import AppToast from "./components/ui/AppToast.vue";

applyTheme();
applyLocale();
applyAnimations();

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
  add_modpack: defineAsyncComponent(() => import("./windows/add_modpack/AddModpackWindow.vue")),
  add_resource: defineAsyncComponent(() => import("./windows/add_resource/AddResourceWindow.vue")),
  collect: defineAsyncComponent(() => import("./windows/collect/CollectWindow.vue")),
  download: defineAsyncComponent(() => import("./windows/download/DownloadWindow.vue")),
  block: defineAsyncComponent(() => import("./windows/block/BlockWindow.vue")),
};

const currentWindow = computed(() => windowMap[currentKind.value]);
</script>

<template>
  <!-- 背景图层（设置里选的本地图片，所有窗口共用） -->
  <div v-if="bgImage" class="app-bg" :style="bgLayerStyle" />
  <KeepAlive>
    <component :is="currentWindow" @close="closeWindow" />
  </KeepAlive>
  <!-- 全局轻提示（所有窗口统一） -->
  <AppToast />
</template>
