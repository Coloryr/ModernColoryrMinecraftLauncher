<script setup lang="ts">
// 服务器添加 / 编辑表单弹窗
//
// 只管排版与输入：草稿对象与保存逻辑在 ServerPane（它是列表的拥有者），
// 保存中不关弹窗 —— 失败提示已由调用方弹出，输入内容留着让用户改。
import BaseButton from "../../../components/ui/BaseButton.vue";
import BaseModal from "../../../components/ui/BaseModal.vue";
import { t } from "../../../lib/i18n";
import type { ServerFormDraft } from "../types";

defineProps<{ form: ServerFormDraft; busy: boolean }>();
const emit = defineEmits<{ (e: "submit"): void; (e: "close"): void }>();
</script>

<template>
  <BaseModal
    :title="form.edit ? t('resource.serverEdit') : t('resource.serverAdd')"
    :closable="false"
    @close="emit('close')"
  >
    <div class="form-grid">
      <label class="form-label">{{ t("resource.serverName") }}</label>
      <input v-model="form.name" class="form-input" type="text" />
      <label class="form-label">{{ t("resource.serverIp") }}</label>
      <input v-model="form.ip" class="form-input" type="text" />
      <!-- 接受材质是客户端选项，只在编辑已有条目时出现（新增走客户端默认） -->
      <template v-if="form.edit">
        <label class="form-label">{{ t("resource.acceptTextures") }}</label>
        <input v-model="form.acceptTextures" type="checkbox" />
      </template>
    </div>
    <div class="modal-actions">
      <BaseButton :disabled="busy" @click="emit('close')">{{ t("resource.cancel") }}</BaseButton>
      <BaseButton variant="primary" :disabled="busy" @click="emit('submit')">
        {{ busy ? t("actions.saving") : t("resource.save") }}
      </BaseButton>
    </div>
  </BaseModal>
</template>

<!-- 弹窗内容是 Teleport 到 body 的，不在 .resource-layout 里，所以这几条自带 scoped 样式 -->
<style scoped>
.form-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  align-items: center;
  gap: 10px 12px;
}

.form-label {
  font-size: 12px;
  color: var(--text-dim);
}

.form-input {
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-side);
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  min-width: 0;
}

.form-input:focus {
  outline: none;
  border-color: var(--accent);
}
</style>
