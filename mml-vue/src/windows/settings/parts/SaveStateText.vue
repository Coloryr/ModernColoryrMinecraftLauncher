<script setup lang="ts">
// 设置窗口 · 保存状态文字（配合 composables/useSaveState）
import { computed } from "vue";
import { t } from "../../../lib/i18n";
import type { SaveState } from "../composables/useSaveState";

const props = defineProps<{
  state: SaveState;
  /** 显式保存按钮的场景：idle 时显示"有未保存的改动"而不是空白 */
  pending?: boolean;
}>();

const text = computed(() => {
  switch (props.state) {
    case "saving":
      return t("winSettings.stateSaving");
    case "saved":
      return t("winSettings.stateSaved");
    case "error":
      return t("winSettings.stateFailed");
    default:
      return props.pending ? t("winSettings.statePending") : "";
  }
});

const kind = computed(() => (props.state === "idle" && props.pending ? "pending" : props.state));
</script>

<template>
  <span v-if="text" class="save-state" :class="kind" role="status" aria-live="polite">
    {{ text }}
  </span>
</template>
