<script setup lang="ts">
// 主窗口顶部栏：品牌 + 主页切换 + 功能入口 + 主题切换 + 账户选择
import { t } from "../../../lib/i18n";
import AccountSelector from "../../../components/AccountSelector.vue";
import type { Account } from "../../../lib/types";
import type { FeatureId } from "../types";

defineProps<{
  features: Array<{ id: FeatureId; icon: string }>;
  newsActive: boolean;
  theme: "dark" | "light";
  currentAccount: Account | null;
  accounts: Account[];
}>();

const emit = defineEmits<{
  (e: "toggle-news"): void;
  (e: "feature", id: FeatureId): void;
  (e: "toggle-theme"): void;
  (e: "update:account", acc: Account): void;
}>();
</script>

<template>
  <header class="topbar">
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
        <svg v-else-if="f.icon === 'user'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
          <circle cx="12" cy="7" r="4" />
        </svg>
        <svg v-else viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
          <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
        </svg>
      </button>

      <!-- 主题切换 -->
      <button class="topbar-icon-btn" :title="theme === 'dark' ? 'Light' : 'Dark'" @click="emit('toggle-theme')">
        <svg v-if="theme === 'dark'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <circle cx="12" cy="12" r="4.5" />
          <path d="M12 2v2.5M12 19.5V22M4.9 4.9l1.8 1.8M17.3 17.3l1.8 1.8M2 12h2.5M19.5 12H22M4.9 19.1l1.8-1.8M17.3 6.7l1.8-1.8" />
        </svg>
        <svg v-else viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
          <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
        </svg>
      </button>

      <AccountSelector
        :account="currentAccount"
        :accounts="accounts"
        @update:account="emit('update:account', $event)"
      />
    </div>
  </header>
</template>

<style scoped>
.topbar {
  height: 64px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 18px;
  background: var(--bg-side);
  border-bottom: 1px solid var(--border);
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

.topbar-right {
  display: flex;
  align-items: center;
  gap: 6px;
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
