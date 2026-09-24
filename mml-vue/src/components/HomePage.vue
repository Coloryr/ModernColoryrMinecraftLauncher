<script setup lang="ts">
// 启动器主页：上次启动实例 / 联机大厅 / 方块列表 入口 + Minecraft 新闻
// empty=true 时（无任何实例）顶部展示空实例引导块（含添加实例按钮）
import { ref } from "vue";
import { t } from "../lib/i18n";
import { showToast } from "../lib/toast";
import type { InstanceInfoDto, NewsItem } from "../lib/bindings";
import NewsPanel from "./NewsPanel.vue";
import InstanceIcon from "./InstanceIcon.vue";
import BlockPanel from "./BlockPanel.vue";

withDefaults(
  defineProps<{
    items: NewsItem[];
    currentInstance: InstanceInfoDto | null;
    empty?: boolean;
    loading?: boolean;
    page?: number;
    hasMore?: boolean;
  }>(),
  { loading: false, page: 1, hasMore: true },
);

const emit = defineEmits<{
  (e: "select", inst: InstanceInfoDto): void;
  (e: "quick-launch"): void;
  /** 点击页头返回按钮：返回实例列表（关闭启动器主页） */
  (e: "back"): void;
  (e: "add-instance"): void;
  (e: "add-account"): void;
  (e: "add-java"): void;
  (e: "refresh"): void;
  (e: "prev"): void;
  (e: "next"): void;
  (e: "open", url: string): void;
}>();

function entry(name: string) {
  showToast(t("actions.wip", { name }));
}

/** 方块列表视图（点方块卡切入，返回键回主页） */
const showBlocks = ref(false);
</script>

<template>
  <div class="home-page">
    <!-- 页头：主页标题 + 返回实例列表（主页自己的页面级控件，不占卡片槽位） -->
    <div v-if="!empty" class="page-head">
      <h2 class="page-title">{{ showBlocks ? t("home.blocks") : t("home.entry") }}</h2>
      <button v-if="showBlocks" class="page-back" @click="showBlocks = false">
        <span class="page-back-arrow">‹</span>
        {{ t("blocks.back") }}
      </button>
      <button v-else class="page-back" @click="emit('back')">
        <span class="page-back-arrow">‹</span>
        {{ t("home.backToList") }}
      </button>
    </div>

    <!-- 无实例：空实例引导块（融合空状态设计） -->
    <div v-if="empty" class="empty-block">
      <div class="empty-block-icon">
        <svg viewBox="0 0 24 24" width="44" height="44" fill="none" stroke="#fff" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 3 4.5 7.5v9L12 21l7.5-4.5v-9L12 3z" />
          <path d="M4.5 7.5 12 12l7.5-4.5" />
          <path d="M12 12v9" />
        </svg>
        <span class="empty-block-badge">＋</span>
      </div>
      <h2 class="empty-block-title">{{ t("empty.title") }}</h2>
      <p class="empty-block-desc">{{ t("empty.desc") }}</p>
      <div class="empty-block-actions">
        <button class="empty-block-btn primary" @click="emit('add-instance')">
          ＋ {{ t("list.add") }}
        </button>
        <button class="empty-block-btn" @click="emit('add-account')">
          {{ t("empty.addAccount") }}
        </button>
        <button class="empty-block-btn" @click="emit('add-java')">
          {{ t("empty.setJava") }}
        </button>
      </div>
    </div>

    <!-- 当前选中实例：仅在选中时显示（未选中时该槽位留空），点一下即可快捷启动 -->
    <div v-if="currentInstance" class="last-card">
      <InstanceIcon :name="currentInstance.name" :uuid="currentInstance.uuid" :size="52" />
      <span class="last-name">{{ currentInstance.name }}</span>
      <button class="last-play" @click="emit('quick-launch')">
        ▶ {{ t("home.lastPlay") }}
      </button>
      <button class="last-open" @click="emit('select', currentInstance)">›</button>
    </div>

    <template v-if="!showBlocks">
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

        <!-- 方块列表 -->
        <button class="entry-card lottery" @click="showBlocks = true">
          <span class="entry-icon">
            <svg viewBox="0 0 24 24" width="26" height="26" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 3 4.5 7.5v9L12 21l7.5-4.5v-9L12 3z" />
              <path d="M4.5 7.5 12 12l7.5-4.5" />
              <path d="M12 12v9" />
              <path d="M19.5 11v6L12 21" />
            </svg>
          </span>
          <span class="entry-text">
            <span class="entry-title">{{ t("home.blocks") }}</span>
            <span class="entry-desc">{{ t("home.blocksDesc") }}</span>
          </span>
          <span class="entry-arrow">›</span>
        </button>
      </div>

      <NewsPanel
        :items="items"
        :loading="loading"
        :page="page"
        :has-more="hasMore"
        @refresh="emit('refresh')"
        @prev="emit('prev')"
        @next="emit('next')"
        @open="(url: string) => emit('open', url)"
      />
    </template>

    <!-- 方块列表视图：搜索 / 分类 / 设为实例图标 -->
    <BlockPanel v-else :current-instance="currentInstance" />
  </div>
</template>

<style scoped>
.home-page {
  display: flex;
  flex-direction: column;
  gap: 18px;
}

/* 页头：主页标题 + 返回实例列表 */
.page-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.page-title {
  font-size: 16px;
  font-weight: 700;
  color: var(--text);
}

.page-back {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 6px 13px 6px 10px;
  border: 1px solid var(--accent-border);
  border-radius: 9px;
  background: var(--bg-card);
  color: var(--accent);
  font-size: 12.5px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.page-back:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.page-back-arrow {
  font-size: 16px;
  line-height: 1;
}

/* 空实例引导块 */
.empty-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 26px 20px 22px;
  border: 1px dashed var(--accent-border);
  border-radius: 16px;
  background: var(--bg-card);
}

.empty-block-icon {
  position: relative;
  width: 96px;
  height: 96px;
  border-radius: 28px;
  background: var(--accent-grad);
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  box-shadow: 0 14px 36px var(--accent-soft);
  margin-bottom: 14px;
}

.empty-block-badge {
  position: absolute;
  right: -8px;
  top: -8px;
  width: 30px;
  height: 30px;
  border-radius: 50%;
  background: var(--bg-card);
  border: 2px solid var(--accent);
  color: var(--accent);
  font-size: 17px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
}

.empty-block-title {
  font-size: 19px;
  font-weight: 700;
}

.empty-block-desc {
  font-size: 13px;
  color: var(--text-dim);
  margin-bottom: 10px;
  text-align: center;
}

.empty-block-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 10px;
  margin-top: 8px;
}

.empty-block-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 9px 20px;
  border: 1px solid var(--accent-border);
  border-radius: 10px;
  background: var(--bg-card);
  color: var(--accent);
  font-size: 13.5px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
}

.empty-block-btn:hover {
  background: var(--accent-soft);
}

.empty-block-btn.primary {
  border: none;
  background: var(--accent);
  color: #fff;
}

.empty-block-btn.primary:hover {
  filter: brightness(1.12);
  background: var(--accent);
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

.last-name {
  flex: 1;
  min-width: 0;
  font-size: 15px;
  font-weight: 700;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
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
</style>
