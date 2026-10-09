<script setup lang="ts">
// 方块列表 · 添加玩家头颅：输入正版用户名或 UUID（同名覆盖）
import { ref } from "vue";
import { t } from "../../../lib/i18n";
import { useModalKeys } from "../../../composables/useModalKeys";
import BaseModal from "../../../components/ui/BaseModal.vue";
import BaseButton from "../../../components/ui/BaseButton.vue";

const props = defineProps<{
  /** 提交中：禁用输入与按钮，Esc 也不关 */
  busy: boolean;
}>();

const emit = defineEmits<{
  (e: "submit", input: string): void;
  (e: "close"): void;
}>();

const input = ref("");

useModalKeys((e) => {
  if (e.key === "Escape" && !props.busy) emit("close");
});

function submit() {
  const value = input.value.trim();
  if (!value || props.busy) return;
  emit("submit", value);
}
</script>

<template>
  <BaseModal :title="t('blocks.addSkin')" :closable="false" @close="!busy && emit('close')">
    <label class="field-label">{{ t("blocks.skinInput") }}</label>
    <input v-model="input" class="field-input" :placeholder="t('blocks.skinInputHint')" spellcheck="false"
      autocomplete="off" :disabled="busy" @keyup.enter="submit" />

    <div class="modal-actions">
      <BaseButton :disabled="busy" @click="emit('close')">{{ t("blocks.cancel") }}</BaseButton>
      <BaseButton variant="primary" :disabled="busy || !input.trim()" @click="submit">
        {{ busy ? t("blocks.skinAdding") : t("blocks.addSkin") }}
      </BaseButton>
    </div>
  </BaseModal>
</template>
