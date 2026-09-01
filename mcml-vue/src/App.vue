<script setup lang="ts">
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
</script>

<template>
  <KeepAlive>
    <component :is="currentWindow" @close="closeWindow" />
  </KeepAlive>
  <!-- 全局轻提示（所有窗口统一） -->
  <AppToast />
</template>
