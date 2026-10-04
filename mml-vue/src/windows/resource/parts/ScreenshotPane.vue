<script setup lang="ts">
// 截图分类：网格 + 大图预览
//
// 预览是**本分类自己的视图状态**（打开哪张、删掉后要不要跟着关掉），所以留在这一层，
// 不往上塞进窗口级 composable；删除 / 清空的确认走共用的 ops。
import { ref } from "vue";
import AsyncImage from "../../../components/ui/AsyncImage.vue";
import BaseButton from "../../../components/ui/BaseButton.vue";
import BaseModal from "../../../components/ui/BaseModal.vue";
import GlyphIcon from "../../../components/ui/GlyphIcon.vue";
import { t } from "../../../lib/i18n";
import { clearScreenshots, deleteScreenshot } from "../../../lib/api";
import ContentHead from "./ContentHead.vue";
import type { useResourceData } from "../composables/useResourceData";
import type { useResourceOps } from "../composables/useResourceOps";
import type { ScreenshotItemDto } from "../../../lib/bindings";

const props = defineProps<{
  data: ReturnType<typeof useResourceData>;
  ops: ReturnType<typeof useResourceOps>;
}>();

const { shots, instanceUuid, loading, shotUrl } = props.data;
const { busy, askConfirm, askDelete, openFolder } = props.ops;

/** 当前预览的截图（null = 没开大图） */
const preview = ref<ScreenshotItemDto | null>(null);

function remove(item: ScreenshotItemDto) {
  askDelete(item.name, async () => {
    await deleteScreenshot(instanceUuid.value, item.name);
    // 删的正是开着的那张：顺手把大图关掉
    if (preview.value?.name === item.name) preview.value = null;
  });
}

function clearAll() {
  askConfirm(t("resource.clear"), t("resource.clearConfirm"), async () => {
    await clearScreenshots(instanceUuid.value);
    preview.value = null;
  });
}
</script>

<template>
  <ContentHead :data="data">
    <h3 class="head-title">{{ t("resource.screenshots") }}</h3>
    <template #actions>
      <button v-if="shots.length" class="mini-btn danger" :disabled="busy" @click="clearAll">
        {{ t("resource.clear") }}
      </button>
    </template>
  </ContentHead>

  <div class="shot-list">
    <div v-if="shots.length" class="shot-grid">
      <div v-for="item in shots" :key="item.name" class="shot-cell">
        <AsyncImage
          class="shot-img"
          :src="shotUrl(item)"
          :alt="item.name"
          :title="item.name"
          @click="preview = item"
        />
        <button class="shot-del" v-tip="t('resource.delete')" @click="remove(item)">
          <GlyphIcon name="close" :size="13" :weight="2.4" />
        </button>
      </div>
    </div>
    <div v-else-if="!loading" class="empty-tip">{{ t("resource.empty") }}</div>
  </div>

  <!-- 大图预览：同样是本分类的浮层，不走窗口级弹窗 -->
  <BaseModal v-if="preview" :title="preview.name" :width="720" @close="preview = null">
    <div class="preview-body">
      <img class="preview-img" :src="shotUrl(preview)" :alt="preview.name" />
    </div>
    <div class="modal-actions">
      <BaseButton @click="openFolder('screenshots', preview.name)">
        {{ t("resource.openFolder") }}
      </BaseButton>
      <BaseButton variant="danger" @click="remove(preview)">
        {{ t("resource.delete") }}
      </BaseButton>
    </div>
  </BaseModal>
</template>

<!-- 大图弹窗的内容是 Teleport 到 body 的，不在 .resource-layout 里，所以这几条自带 scoped 样式 -->
<style scoped>
.preview-body {
  display: flex;
  justify-content: center;
  background: var(--bg-side);
  border-radius: 9px;
  padding: 8px;
}

.preview-img {
  max-width: 100%;
  max-height: 60vh;
  object-fit: contain;
  border-radius: 6px;
}
</style>
