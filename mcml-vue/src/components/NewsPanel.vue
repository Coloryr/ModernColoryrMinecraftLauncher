<script setup lang="ts">
import { t } from "../lib/i18n";
import type { NewsItem } from "../lib/types";

withDefaults(
  defineProps<{
    items: NewsItem[];
    /** 加载中（刷新按钮转圈并禁用） */
    loading?: boolean;
    /** 当前页码（从 1 开始） */
    page?: number;
    /** 是否还有下一页 */
    hasMore?: boolean;
  }>(),
  { loading: false, page: 1, hasMore: true },
);

const emit = defineEmits<{
  (e: "refresh"): void;
  (e: "prev"): void;
  (e: "next"): void;
  (e: "open", url: string): void;
}>();

/** 分类标签配色：按 tag 字符串哈希取色相，同一分类颜色稳定且各不相同 */
function tagColor(tag: string) {
  let h = 0;
  for (const c of tag) h = (h * 31 + c.charCodeAt(0)) % 360;
  return {
    background: `hsla(${h}, 65%, 55%, 0.16)`,
    color: `hsl(${h}, 75%, 70%)`,
  };
}
</script>

<template>
  <div class="news-panel">
    <div class="panel-head">
      <h2>{{ t("news.head") }}</h2>
      <button
        class="refresh-btn"
        :class="{ spinning: loading }"
        :disabled="loading"
        :title="t('news.refresh')"
        @click="emit('refresh')"
      >
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 1 1-2.64-6.36" />
          <polyline points="21 3 21 9 15 9" />
        </svg>
      </button>
    </div>

    <div class="news-list">
      <div
        v-for="item in items"
        :key="item.id"
        class="news-item"
        @click="emit('open', item.url)"
      >
        <img :src="item.image" class="banner" alt="" />
        <div class="news-meta">
          <span class="news-tag" :style="tagColor(item.tag)">{{ item.tag }}</span>
          <span class="news-date">{{ item.date }}</span>
        </div>
        <h3 class="news-title">{{ item.title }}</h3>
      </div>
      <div v-if="items.length === 0" class="empty-tip">
        <span class="empty-icon">📰</span>
        <span>{{ loading ? t("news.loading") : t("news.empty") }}</span>
      </div>
    </div>

    <!-- 分页（页码从 1 开始） -->
    <div v-if="items.length > 0" class="news-pager">
      <button
        class="pager-btn"
        :disabled="page <= 1 || loading"
        @click="emit('prev')"
      >
        {{ t("news.prev") }}
      </button>
      <span class="pager-info">{{ t("news.page", { n: page }) }}</span>
      <button
        class="pager-btn"
        :disabled="!hasMore || loading"
        @click="emit('next')"
      >
        {{ t("news.next") }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.news-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.panel-head h2 {
  font-size: 17px;
  font-weight: 700;
}

.refresh-btn {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all 0.15s;
}

.refresh-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: var(--accent);
}

.refresh-btn:disabled {
  cursor: default;
  opacity: 0.6;
}

.refresh-btn svg {
  animation: news-rotate 0.9s linear infinite paused;
}

.refresh-btn.spinning svg {
  animation-play-state: running;
}

@keyframes news-rotate {
  to {
    transform: rotate(360deg);
  }
}

.news-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 16px;
}

.news-item {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
  transition: all 0.15s;
  cursor: pointer;
}

.news-item:hover {
  border-color: var(--accent);
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
}

.banner {
  width: 100%;
  height: 110px;
  object-fit: cover;
  display: block;
}

.news-meta {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px 0;
}

.news-tag {
  font-size: 10px;
  padding: 1px 8px;
  border-radius: 20px;
  background: rgba(79, 140, 255, 0.16);
  color: #8fb0ff;
}

.news-date {
  font-size: 11px;
  color: var(--text-dim);
}

.news-title {
  font-size: 13.5px;
  font-weight: 600;
  line-height: 1.5;
  padding: 14px 14px 14px;
}

.news-pager {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 14px;
}

.pager-btn {
  padding: 6px 16px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
  font-size: 12.5px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
}

.pager-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: var(--accent);
}

.pager-btn:disabled {
  opacity: 0.4;
  cursor: default;
}

.pager-info {
  font-size: 12.5px;
  color: var(--text-dim);
  min-width: 56px;
  text-align: center;
}

.empty-tip {
  grid-column: 1 / -1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 60px 0;
  color: var(--text-dim);
  font-size: 13px;
}

.empty-icon {
  font-size: 32px;
  opacity: 0.55;
}
</style>
