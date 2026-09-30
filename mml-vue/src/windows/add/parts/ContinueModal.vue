<script setup lang="ts">
// 添加实例 · 创建成功后的「是否继续添加」询问
// 点遮罩 / 右上角关闭等同于「否」（丢下这次的表单直接关窗）
import { t } from "../../../lib/i18n";
import BaseModal from "../../../components/ui/BaseModal.vue";
import BaseButton from "../../../components/ui/BaseButton.vue";

defineProps<{ name: string }>();

const emit = defineEmits<{
  (e: "continue"): void;
  (e: "finish"): void;
}>();
</script>

<template>
  <BaseModal :title="t('add.addedTitle')" :closable="false" @close="emit('finish')">
    <p class="modal-text">{{ t("add.askContinue", { name }) }}</p>
    <div class="modal-actions">
      <BaseButton @click="emit('finish')">{{ t("add.no") }}</BaseButton>
      <BaseButton variant="primary" @click="emit('continue')">{{ t("add.yes") }}</BaseButton>
    </div>
  </BaseModal>
</template>
