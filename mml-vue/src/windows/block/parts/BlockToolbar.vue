<script setup lang="ts">
// 方块列表 · 工具条：搜索（可清空）+ 图标尺寸档 + 添加皮肤 / 重新渲染 + 计数
import { computed } from "vue";
import { t } from "../../../lib/i18n";
import BaseButton from "../../../components/ui/BaseButton.vue";
import SegmentedTabs from "../../../components/ui/SegmentedTabs.vue";
import { BLOCK_SIZE_ORDER, isBlockSize, type BlockSize } from "../types";

const props = defineProps<{
  keyword: string;
  size: BlockSize;
  /** 渲染中：重新渲染禁用 */
  running: boolean;
}>();

const emit = defineEmits<{
  (e: "update:keyword", v: string): void;
  (e: "update:size", v: BlockSize): void;
  (e: "add-skin"): void;
  (e: "re-render"): void;
}>();

const sizeOptions = computed(() =>
  BLOCK_SIZE_ORDER.map((s) => ({ value: s as string, label: t(`blocks.size.${s}`) })),
);

/** 分段控件给出的是 string，这里收口成 BlockSize */
function onSize(v: string) {
  if (isBlockSize(v)) emit("update:size", v);
}
</script>

<template>
  <div class="block-top">
    <div class="search-wrap">
      <input
        :value="keyword"
        class="field-input block-search"
        type="text"
        :placeholder="t('blocks.search')"
        spellcheck="false"
        autocomplete="off"
        @input="emit('update:keyword', ($event.target as HTMLInputElement).value)"
        @keydown.esc="emit('update:keyword', '')"
      />
      <button
        v-if="keyword"
        type="button"
        class="search-clear"
        v-tip="t('blocks.searchClear')"
        @click="emit('update:keyword', '')"
      >
        ✕
      </button>
    </div>

    <!-- 图标尺寸档：小 / 中 / 大 -->
    <SegmentedTabs :options="sizeOptions" :model-value="size" @update:model-value="onSize" />

    <BaseButton @click="emit('add-skin')">{{ t("blocks.addSkin") }}</BaseButton>
    <BaseButton :disabled="running" @click="emit('re-render')">{{ t("blocks.reRender") }}</BaseButton>
  </div>
</template>

<style scoped>
.block-top {
  display: flex;
  align-items: center;
  gap: 8px;
  /* 在标题栏里要能收缩：窗口最窄时优先挤搜索框，不去裁按钮 */
  flex-shrink: 1;
  min-width: 0;
}

.search-wrap {
  position: relative;
  /* 现在固定住在标题栏里：给个确定宽度，不再靠 flex:1 撑（标题栏里没有富余空间可分）；
     窗口很窄时允许收缩到这里的最小值 */
  width: 200px;
  min-width: 120px;
  flex: 0 1 auto;
  display: flex;
}

/* 压到与工具条按钮同高（BaseButton md / SegmentedTabs 都是 35px）：
   .field-input 默认 min-height: var(--field-h) = 42px，这里显式覆盖 */
.block-top .block-search {
  flex: 1;
  min-width: 0;
  height: 35px;
  min-height: 0;
  padding: 0 30px 0 12px;
  font-size: 13px;
}

.search-clear {
  position: absolute;
  top: 50%;
  right: 6px;
  transform: translateY(-50%);
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-dim);
  font-size: 11px;
  line-height: 1;
  cursor: pointer;
}

.search-clear:hover {
  background: var(--bg-hover);
  color: var(--text);
}
</style>
