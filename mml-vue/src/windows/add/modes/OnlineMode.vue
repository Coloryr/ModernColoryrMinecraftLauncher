<script setup lang="ts">
// 添加实例 · 模式四：在线实例（下载地址）
import { t } from "../../../lib/i18n";

defineProps<{ url: string }>();

const emit = defineEmits<{ (e: "update:url", v: string): void }>();
</script>

<template>
  <label class="field-label">{{ t("add.url") }} <span class="req">*</span></label>
  <input
    :value="url"
    class="field-input"
    :placeholder="t('add.urlPlaceholder')"
    spellcheck="false"
    @input="emit('update:url', ($event.target as HTMLInputElement).value)"
  />
  <p class="hint">{{ t("add.urlHint") }}</p>
</template>

<style scoped>
.req {
  color: var(--red);
}

.hint {
  font-size: 12px;
  color: var(--text-dim);
  margin-top: 8px;
}
</style>
