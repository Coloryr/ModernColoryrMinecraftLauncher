<script setup lang="ts">
// 方块列表 · 详情弹窗：大图预览 + ID（可复制）/ 分类 + 上一个 / 下一个 + 设为实例图标
//
// `BaseModal` 不处理 Esc，且内容被 Teleport 到 body，所以键盘在这里自己接管：
// Esc 关闭、← / → 在**当前过滤结果**里前后切换（与网格里的方向键同一顺序）。
import { t } from "../../../lib/i18n";
import { showToast } from "../../../lib/toast";
import { copyText } from "../../../lib/clipboard";
import { useModalKeys } from "../../../composables/useModalKeys";
import BaseModal from "../../../components/ui/BaseModal.vue";
import BaseButton from "../../../components/ui/BaseButton.vue";
import AsyncImage from "../../../components/ui/AsyncImage.vue";
import HighlightText from "../../../components/ui/HighlightText.vue";
import type { BlockItemDto } from "../../../lib/bindings";

const props = defineProps<{
  block: BlockItemDto;
  /** 分类显示名（父级已翻译） */
  category: string;
  /** 搜索词：详情里同样高亮，方便看清为什么命中 */
  keyword: string;
  canPrev: boolean;
  canNext: boolean;
  /** 皮肤方块（可删除） */
  removable: boolean;
  /** 正在设图标（父级控制，禁用按钮防连点） */
  busy?: boolean;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "prev"): void;
  (e: "next"): void;
  (e: "remove"): void;
  (e: "set-icon"): void;
}>();

useModalKeys((e) => {
  if (e.key === "Escape") emit("close");
  else if (e.key === "ArrowLeft" && props.canPrev) emit("prev");
  else if (e.key === "ArrowRight" && props.canNext) emit("next");
});

async function copyId() {
  showToast((await copyText(props.block.id)) ? t("blocks.copied") : t("blocks.copyFail"));
}
</script>

<template>
  <BaseModal :title="block.name" :width="470" :closable="false" @close="emit('close')">
    <div class="detail-body">
      <div class="detail-stage">
        <AsyncImage class="detail-img" :src="block.image" :alt="block.name" />
      </div>
      <div class="detail-info">
        <div class="detail-row">
          <span class="detail-label">{{ t("blocks.detailId") }}</span>
          <div class="detail-id">
            <code class="detail-code"><HighlightText :text="block.id" :query="keyword" /></code>
            <BaseButton size="sm" variant="ghost" v-tip="t('blocks.copyId')" @click="copyId">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <rect x="9" y="9" width="12" height="12" rx="2" />
                <path d="M5 15H4a1 1 0 0 1-1-1V4a1 1 0 0 1 1-1h10a1 1 0 0 1 1 1v1" />
              </svg>
            </BaseButton>
          </div>
        </div>
        <div class="detail-row">
          <span class="detail-label">{{ t("blocks.detailCat") }}</span>
          <span class="cat-chip">{{ category }}</span>
        </div>
        <p class="detail-nav-hint">{{ t("blocks.detailNavHint") }}</p>
      </div>
    </div>

    <div class="modal-actions">
      <!-- 前后切换靠左，与右侧的关闭 / 设图标分开 -->
      <span class="nav-group">
        <BaseButton size="sm" :disabled="!canPrev" v-tip="t('blocks.prev')" @click="emit('prev')">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m15 18-6-6 6-6" />
          </svg>
        </BaseButton>
        <BaseButton size="sm" :disabled="!canNext" v-tip="t('blocks.next')" @click="emit('next')">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="m9 6 6 6-6 6" />
          </svg>
        </BaseButton>
      </span>

      <BaseButton v-if="removable" variant="danger" @click="emit('remove')">
        {{ t("blocks.skinRemove") }}
      </BaseButton>
      <BaseButton @click="emit('close')">{{ t("blocks.close") }}</BaseButton>
      <BaseButton variant="primary" :disabled="busy" @click="emit('set-icon')">
        {{ t("blocks.setIcon") }}
      </BaseButton>
    </div>
  </BaseModal>
</template>

<style scoped>
.detail-body {
  display: flex;
  gap: 16px;
  align-items: flex-start;
}

.detail-stage {
  flex-shrink: 0;
  width: 160px;
  height: 160px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-side);
}

.detail-img {
  width: 132px;
  height: 132px;
  image-rendering: pixelated;
}

.detail-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding-top: 4px;
}

.detail-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.detail-label {
  font-size: 11.5px;
  color: var(--text-dim);
}

/* ID 行：代码块 + 复制按钮 */
.detail-id {
  display: flex;
  align-items: center;
  gap: 6px;
}

.detail-code {
  flex: 1;
  min-width: 0;
  padding: 4px 8px;
  border: 1px solid var(--border);
  border-radius: 7px;
  background: var(--bg-side);
  font-family: ui-monospace, Consolas, "Courier New", monospace;
  font-size: 12px;
  color: var(--text);
  word-break: break-all;
}

.cat-chip {
  align-self: flex-start;
  padding: 2px 9px;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 12px;
  font-weight: 600;
}

.detail-nav-hint {
  margin-top: auto;
  font-size: 11.5px;
  color: var(--text-dim);
  opacity: 0.8;
}

.nav-group {
  margin-right: auto;
  display: flex;
  gap: 6px;
}
</style>
