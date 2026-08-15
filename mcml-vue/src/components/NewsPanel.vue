<script setup lang="ts">
import { t } from "../lib/i18n";
import type { NewsItem } from "../lib/types";

defineProps<{ items: NewsItem[] }>();
</script>

<template>
  <div class="news-panel">
    <div class="panel-head">
      <h2>{{ t("news.head") }}</h2>
    </div>

    <div class="news-list">
      <div v-for="item in items" :key="item.id" class="news-item">
        <img :src="item.image" class="banner" alt="" />
        <div class="news-meta">
          <span class="news-tag">{{ item.tag }}</span>
          <span class="news-date">{{ item.date }}</span>
        </div>
        <h3 class="news-title">{{ item.title }}</h3>
      </div>
      <div v-if="items.length === 0" class="empty-tip">{{ t("news.empty") }}</div>
    </div>
  </div>
</template>

<style scoped>
.news-panel {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.panel-head h2 {
  font-size: 17px;
  font-weight: 700;
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
  padding: 8px 14px 14px;
}
</style>
