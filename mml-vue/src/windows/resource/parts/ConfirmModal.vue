<script setup lang="ts">
// 删除 / 清空确认弹窗（全窗口共用一份，内容来自 useResourceOps 的 confirmBox）
//
// 按钮改用全仓通用的 BaseButton + 全局 .modal-actions：原先这里是手写的 .modal-btn，
// 尺寸与配色都是本窗口自己一套，和别的窗口的确认框不一致。
import BaseButton from "../../../components/ui/BaseButton.vue";
import BaseModal from "../../../components/ui/BaseModal.vue";
import { t } from "../../../lib/i18n";
import type { useResourceOps } from "../composables/useResourceOps";

const props = defineProps<{ ops: ReturnType<typeof useResourceOps> }>();
const emit = defineEmits<{ (e: "close"): void }>();

const { confirmBox, confirmBusy, runConfirm } = props.ops;
</script>

<template>
  <BaseModal v-if="confirmBox" :title="confirmBox.title" :closable="false" @close="emit('close')">
    <p class="delete-tip">{{ confirmBox.text }}</p>
    <div class="modal-actions">
      <BaseButton :disabled="confirmBusy" @click="emit('close')">
        {{ t("resource.cancel") }}
      </BaseButton>
      <BaseButton variant="danger" :disabled="confirmBusy" @click="runConfirm">
        {{ confirmBusy ? t("actions.deleting") : t("actions.confirm") }}
      </BaseButton>
    </div>
  </BaseModal>
</template>
