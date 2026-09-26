<script setup lang="ts">
// 主窗口顶部栏：同时充当自绘标题栏（整条可拖动，两端放窗口按钮）
// 布局：品牌 + 主页切换 + 功能入口 + 主题切换 + 账户选择
import { t } from "../../../lib/i18n";
import AccountSelector from "../../../components/AccountSelector.vue";
import WindowControls from "../../../components/ui/WindowControls.vue";
import { onTitleBarPointerDown, titleBarStyle } from "../../../lib/titlebar";
import type { AccountStoreDto } from "../../../lib/bindings";
import type { FeatureId } from "../types";

defineProps<{
  features: Array<{ id: FeatureId; icon: string }>;
  newsActive: boolean;
  currentAccount: AccountStoreDto | null;
  accounts: AccountStoreDto[];
}>();

const emit = defineEmits<{
  (e: "toggle-news"): void;
  (e: "feature", id: FeatureId): void;
  (e: "update:account", acc: AccountStoreDto): void;
}>();
</script>

<template>
  <header class="topbar" :class="titleBarStyle" @pointerdown="onTitleBarPointerDown">
    <!-- macos 样式：红黄绿在左端 -->
    <WindowControls v-if="titleBarStyle === 'macos'" :style="titleBarStyle" />

    <div class="brand">
      <div class="brand-logo">MC</div>
      <div class="brand-text">
        <div class="brand-name">{{ t("app.name") }}</div>
        <div class="brand-sub">{{ t("app.sub") }}</div>
      </div>
    </div>

    <div class="topbar-right">
      <!-- 启动器主页（切换按钮） -->
      <button
        class="topbar-icon-btn"
        :class="{ pressed: newsActive }"
        :title="t('home.entry')"
        @click="emit('toggle-news')"
      >
        <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="m3 10 9-7 9 7v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V10z" />
          <path d="M9 22V12h6v10" />
        </svg>
      </button>

      <!-- 功能入口 -->
      <button
        v-for="f in features"
        :key="f.id"
        class="topbar-icon-btn"
        :title="t('features.' + f.id)"
        @click="emit('feature', f.id)"
      >
        <svg v-if="f.icon === 'gear'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8">
          <circle cx="12" cy="12" r="3.2" />
          <path d="M19.4 15a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1.03 1.56V21a2 2 0 1 1-4 0v-.09a1.7 1.7 0 0 0-1.03-1.56 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.56-1.03H3a2 2 0 1 1 0-4h.09a1.7 1.7 0 0 0 1.56-1.03 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34h.01a1.7 1.7 0 0 0 1.03-1.56V3a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 1.03 1.56 1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87v.01a1.7 1.7 0 0 0 1.56 1.03H21a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.56 1.03z" />
        </svg>
        <svg v-else-if="f.icon === 'chart'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <path d="M18 20V10M12 20V4M6 20v-6" />
        </svg>
        <svg v-else-if="f.icon === 'star'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="m12 2.6 2.9 5.9 6.5.95-4.7 4.58 1.1 6.47L12 17.44 6.2 20.5l1.1-6.47-4.7-4.58 6.5-.95z" />
        </svg>
        <svg v-else viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
          <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
        </svg>
      </button>

      <AccountSelector
        :account="currentAccount"
        :accounts="accounts"
        @update:account="emit('update:account', $event)"
      />
    </div>

    <!-- windows 样式：最小化 / 最大化 / 关闭在右端 -->
    <WindowControls v-if="titleBarStyle === 'windows'" :style="titleBarStyle" />
  </header>
</template>

<style scoped>
/* 高度即标题栏高度：顶栏是 .main-window 的第一个 flex 子节点，改高度会让
   顶栏以下的所有内容（含侧栏）整体下移 */
.topbar {
  height: 64px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 0 18px;
  background: var(--bg-side);
  border-bottom: 1px solid var(--border);
}

/* windows 样式的窗口按钮是小方块、不贴边，右侧留一点呼吸空间；
   macos 样式的圆点留在左内边距之后（与系统一致的 ~18px 留白） */
.topbar.windows {
  padding-right: 10px;
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-logo {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  background: linear-gradient(135deg, #4f8cff, #7c5cff);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 800;
  font-size: 16px;
  color: #fff;
  box-shadow: 0 3px 10px rgba(79, 140, 255, 0.35);
}

.brand-text {
  display: flex;
  flex-direction: column;
  line-height: 1.2;
}

.brand-name {
  font-size: 15px;
  font-weight: 700;
}

.brand-sub {
  font-size: 11px;
  color: var(--text-dim);
}

/* 右侧集群自己顶到最右：窗口按钮插在头（macos）或尾（windows）时，
   都不会被布局挤到中间 */
.topbar-right {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
}

.topbar-icon-btn {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.topbar-icon-btn:hover {
  background: var(--bg-card);
  border-color: var(--border);
  color: var(--text);
}

.topbar-icon-btn.pressed {
  background: var(--accent-soft);
  border-color: var(--accent-border);
  color: var(--accent);
}
</style>
