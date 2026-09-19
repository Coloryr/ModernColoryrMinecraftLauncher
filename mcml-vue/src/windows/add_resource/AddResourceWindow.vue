<script setup lang="ts">
// 添加资源窗口（占位）：按资源类型浏览 / 获取等待实现
import WindowFrame from "../../components/ui/WindowFrame.vue";
import { t } from "../../lib/i18n";

defineEmits<{ (e: "close"): void }>();
</script>

<template>
  <WindowFrame :title="t('winTitle.addResource')" @close="$emit('close')">
    <p class="pending">{{ t("winCommon.pending") }}</p>
  </WindowFrame>
</template>

<style scoped>
.pending {
  margin: 24px 0;
  color: var(--text-dim);
  text-align: center;
}
</style>
