<script setup lang="ts">
// 应用外壳：负责主题应用与窗口路由。
// 单窗口模式：currentKind 在应用内切换（history 同步）；
// 多窗口模式：本页面即当前窗口（?window=xxx），功能窗口为新标签页/真实窗口。
// KeepAlive：切换窗口时保留各窗口组件的状态（如主窗口的实例列表、选中项）。
import { computed } from "vue";
import { applyTheme } from "./lib/theme";
import { applyLocale } from "./lib/i18n";
import { closeWindow, currentKind } from "./windows/windowManager";
import MainWindow from "./windows/main/MainWindow.vue";
import SettingsWindow from "./windows/settings/SettingsWindow.vue";
import StatsWindow from "./windows/stats/StatsWindow.vue";
import SkinWindow from "./windows/skin/SkinWindow.vue";
import HelpWindow from "./windows/help/HelpWindow.vue";
import ResourceWindow from "./windows/resource/ResourceWindow.vue";
import AccountWindow from "./windows/account/AccountWindow.vue";

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
};

const currentWindow = computed(() => windowMap[currentKind.value]);
</script>

<template>
  <KeepAlive>
    <component :is="currentWindow" @close="closeWindow" />
  </KeepAlive>
</template>
