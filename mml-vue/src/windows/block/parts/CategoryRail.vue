<script setup lang="ts">
// 方块列表 · 左侧分类栏：全部 + 各分类条目数，底部显示渲染版本
import { t } from "../../../lib/i18n";

interface CatItem {
  /** 分类 ID（"" = 全部） */
  id: string;
  label: string;
  count: number;
}

defineProps<{
  items: CatItem[];
  active: string;
  version: string;
}>();

const emit = defineEmits<{ (e: "pick", id: string): void }>();
</script>

<template>
  <aside class="cat-rail" role="tablist" aria-orientation="vertical">
    <button
      v-for="c in items"
      :key="c.id || '__all'"
      type="button"
      role="tab"
      class="cat-item"
      :class="{ on: active === c.id }"
      :aria-selected="active === c.id"
      @click="emit('pick', c.id)"
    >
      <span class="cat-name">{{ c.label }}</span>
      <span class="cat-num">{{ c.count }}</span>
    </button>
    <div v-if="version" class="cat-foot">{{ t("blocks.version", { v: version }) }}</div>
  </aside>
</template>

<style scoped>
.cat-rail {
  width: 168px;
  min-width: 168px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-y: auto;
  padding-right: 2px;
}

.cat-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 11px;
  border: none;
  border-radius: 9px;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: background 0.12s, color 0.12s;
}

.cat-item:hover {
  background: var(--bg-hover);
}

.cat-item:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

.cat-item.on {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

.cat-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cat-num {
  flex-shrink: 0;
  font-size: 11.5px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.cat-item.on .cat-num {
  color: var(--accent);
}

.cat-foot {
  margin-top: auto;
  padding: 10px 11px 2px;
  font-size: 11.5px;
  color: var(--text-dim);
}
</style>
