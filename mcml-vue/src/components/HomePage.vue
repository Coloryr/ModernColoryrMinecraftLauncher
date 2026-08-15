<script setup lang="ts">
// 启动器主页：上次启动实例 / 联机大厅 / 每日抽奖 入口 + Minecraft 新闻
import { ref } from "vue";
import { t } from "../lib/i18n";
import type { InstanceInfo, NewsItem } from "../lib/types";
import NewsPanel from "./NewsPanel.vue";
import InstanceIcon from "./InstanceIcon.vue";

defineProps<{
  items: NewsItem[];
  lastInstance: InstanceInfo | null;
}>();

const emit = defineEmits<{
  (e: "select", inst: InstanceInfo): void;
  (e: "quick-launch"): void;
}>();

const toast = ref("");
let toastTimer: number | undefined;

function showToast(msg: string) {
  toast.value = msg;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.value = "";
  }, 2000);
}

function entry(name: string) {
  showToast(t("actions.wip", { name }));
}
</script>

<template>
  <div class="home-page">
    <!-- 上次启动实例（常驻显示） -->
    <div class="last-card" :class="{ empty: !lastInstance }">
      <InstanceIcon v-if="lastInstance" :name="lastInstance.name" :uuid="lastInstance.uuid" :size="52" />
      <span v-else class="last-icon-placeholder">▶</span>
      <div class="last-info">
        <span class="last-title">{{ t("home.lastInstance") }}</span>
        <span v-if="lastInstance" class="last-name">{{ lastInstance.name }}</span>
        <span v-else class="last-name">{{ t("home.lastEmpty") }}</span>
        <span class="last-desc">{{ t("home.lastInstanceDesc") }}</span>
      </div>
      <button
        v-if="lastInstance"
        class="last-play"
        :disabled="!lastInstance"
        @click="emit('quick-launch')"
      >
        ▶ {{ t("home.lastPlay") }}
      </button>
      <button
        v-if="lastInstance"
        class="last-open"
        @click="emit('select', lastInstance)"
      >›</button>
    </div>

    <div class="entry-cards">
      <!-- 联机大厅 -->
      <button class="entry-card lobby" @click="entry(t('home.lobby'))">
        <span class="entry-icon">
          <svg viewBox="0 0 24 24" width="26" height="26" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2" />
            <circle cx="9" cy="7" r="4" />
            <path d="M23 21v-2a4 4 0 0 0-3-3.87M16 3.13a4 4 0 0 1 0 7.75" />
          </svg>
        </span>
        <span class="entry-text">
          <span class="entry-title">{{ t("home.lobby") }}</span>
          <span class="entry-desc">{{ t("home.lobbyDesc") }}</span>
        </span>
        <span class="entry-arrow">›</span>
      </button>

      <!-- 每日抽奖 -->
      <button class="entry-card lottery" @click="entry(t('home.lottery'))">
        <span class="entry-icon">
          <svg viewBox="0 0 24 24" width="26" height="26" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 2 14.5 8.5 21 10l-6.5 1.5L12 18l-2.5-6.5L3 10l6.5-1.5L12 2z" />
            <path d="M19 15l.9 2.1L22 18l-2.1.9L19 21l-.9-2.1L16 18l2.1-.9L19 15z" />
          </svg>
        </span>
        <span class="entry-text">
          <span class="entry-title">{{ t("home.lottery") }}</span>
          <span class="entry-desc">{{ t("home.lotteryDesc") }}</span>
        </span>
        <span class="entry-arrow">›</span>
      </button>
    </div>

    <NewsPanel :items="items" />

    <Transition name="toast">
      <div v-if="toast" class="toast">{{ toast }}</div>
    </Transition>
  </div>
</template>

<style scoped>
.home-page {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

/* 上次启动实例 */
.last-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 16px;
  border-radius: 14px;
  border: 1px solid var(--accent-border);
  background: var(--accent-soft);
}

.last-card.empty {
  border-style: dashed;
  background: var(--bg-card);
}

.last-icon-placeholder {
  width: 52px;
  height: 52px;
  border-radius: 12px;
  background: var(--bg-hover);
  color: var(--text-dim);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  flex-shrink: 0;
}

.last-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.last-title {
  font-size: 11px;
  color: var(--accent);
  font-weight: 600;
  letter-spacing: 0.5px;
}

.last-name {
  font-size: 15px;
  font-weight: 700;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.last-desc {
  font-size: 11.5px;
  color: var(--text-dim);
}

.last-play {
  padding: 9px 18px;
  border-radius: 9px;
  border: none;
  background: var(--accent);
  color: #fff;
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: filter 0.15s;
  white-space: nowrap;
}

.last-play:hover {
  filter: brightness(1.12);
}

.last-open {
  width: 34px;
  height: 34px;
  border-radius: 9px;
  border: 1px solid var(--accent-border);
  background: transparent;
  color: var(--accent);
  font-size: 18px;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.15s;
}

.last-open:hover {
  background: var(--accent);
  color: #fff;
}

.entry-cards {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
}

.entry-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 16px 18px;
  border-radius: 14px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text);
  cursor: pointer;
  text-align: left;
  font-family: inherit;
  transition: all 0.15s;
}

.entry-card:hover {
  transform: translateY(-2px);
  box-shadow: var(--shadow-md);
}

.entry-card.lobby:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.entry-card.lottery:hover {
  border-color: var(--yellow);
  background: rgba(245, 185, 68, 0.12);
}

.entry-icon {
  width: 48px;
  height: 48px;
  border-radius: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.entry-card.lobby .entry-icon {
  background: var(--accent-soft);
  color: var(--accent);
}

.entry-card.lottery .entry-icon {
  background: rgba(245, 185, 68, 0.14);
  color: var(--yellow);
}

.entry-text {
  display: flex;
  flex-direction: column;
  gap: 4px;
  flex: 1;
  min-width: 0;
}

.entry-title {
  font-size: 15px;
  font-weight: 700;
}

.entry-desc {
  font-size: 12px;
  color: var(--text-dim);
}

.entry-arrow {
  font-size: 20px;
  color: var(--text-dim);
  flex-shrink: 0;
}

.entry-card:hover .entry-arrow {
  color: var(--accent);
}

.toast {
  position: fixed;
  bottom: 34px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--bg-card);
  border: 1px solid var(--accent-border);
  color: var(--text);
  font-size: 13px;
  padding: 11px 22px;
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  z-index: 500;
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(8px);
}
</style>
