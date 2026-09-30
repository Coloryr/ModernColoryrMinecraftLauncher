<script setup lang="ts">
// 方块列表窗口：独立窗口承载方块网格（搜索 / 分类 / 设为实例图标）
// 当前实例从 gui_config 的选中实例恢复（与主窗口共享同一份配置）
// 渲染进行中后端拒绝关闭（close-blocked 事件），这里弹二次确认：中断渲染再关
import { onMounted, onUnmounted, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BlockPanel from "./BlockPanel.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import { t } from "../../lib/i18n";
import { api, blockRenderCancel, onCloseBlocked } from "../../lib/api";
import { loadGuiConfig } from "../../lib/guiConfig";
import type { InstanceInfoDto } from "../../lib/bindings";

const emit = defineEmits<{ (e: "close"): void }>();

const currentInstance = ref<InstanceInfoDto | null>(null);
let unlisten: (() => void) | null = null;

/** 关闭被渲染拦下时的二次确认框 */
const confirmClose = ref(false);

onMounted(async () => {
  try {
    const [cfg, instances] = await Promise.all([loadGuiConfig(), api.getInstances()]);
    const uuid = cfg?.mainWindow.selectedInstance;
    if (uuid) currentInstance.value = instances.find((i) => i.uuid === uuid) ?? null;
  } catch {
    currentInstance.value = null;
  }
  unlisten = await onCloseBlocked(() => {
    confirmClose.value = true;
  });
});

onUnmounted(() => unlisten?.());

/** 中断渲染并关闭：取消后 render_running 立即放行，再次关闭不会被拦 */
async function confirmCloseRender() {
  confirmClose.value = false;
  try {
    await blockRenderCancel();
  } catch {
    // 取消失败（渲染已结束等）不挡关窗
  }
  emit("close");
}
</script>

<template>
  <WindowFrame body-fill :title="t('home.blocks')" @close="emit('close')">
    <BlockPanel :current-instance="currentInstance" />

    <!-- 渲染中关窗：确认后中断渲染再关 -->
    <BaseModal v-if="confirmClose" :title="t('blocks.closeRenderTitle')" :closable="false" @close="confirmClose = false">
      <p class="delete-tip">{{ t("blocks.closeRenderConfirm") }}</p>
      <div class="modal-actions">
        <BaseButton @click="confirmClose = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" @click="confirmCloseRender">
          {{ t("blocks.closeRenderOk") }}
        </BaseButton>
      </div>
    </BaseModal>
  </WindowFrame>
</template>
