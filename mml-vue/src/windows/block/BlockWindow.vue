<script setup lang="ts">
// 方块列表窗口：独立窗口承载方块网格（搜索 / 分类 / 设为实例图标）
// 当前实例从 gui_config 的选中实例恢复（与主窗口共享同一份配置）
import { onMounted, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BlockPanel from "../../components/BlockPanel.vue";
import { t } from "../../lib/i18n";
import { api } from "../../lib/api";
import { loadGuiConfig } from "../../lib/guiConfig";
import type { InstanceInfoDto } from "../../lib/bindings";

defineEmits<{ (e: "close"): void }>();

const currentInstance = ref<InstanceInfoDto | null>(null);

onMounted(async () => {
  try {
    const [cfg, instances] = await Promise.all([loadGuiConfig(), api.getInstances()]);
    const uuid = cfg?.mainWindow.selectedInstance;
    if (uuid) currentInstance.value = instances.find((i) => i.uuid === uuid) ?? null;
  } catch {
    currentInstance.value = null;
  }
});
</script>

<template>
  <WindowFrame body-fill :title="t('home.blocks')" @close="$emit('close')">
    <BlockPanel :current-instance="currentInstance" />
  </WindowFrame>
</template>
