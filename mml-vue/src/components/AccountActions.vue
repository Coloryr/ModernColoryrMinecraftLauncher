<script setup lang="ts">
// 账户操作：刷新 Token / 重新登录 / 编辑（离线）/ 删除（单色 SVG 图标按钮）
// 按钮显隐由后端给的能力标志决定（哪些账户类型有什么操作是业务规则，不由前端硬编码）
import { t } from "../lib/i18n";

defineProps<{
  /** 是否有可刷新的令牌（AccountStoreDto.canRefresh） */
  canRefresh?: boolean;
  /** 是否支持重新登录（AccountStoreDto.canRelogin） */
  canRelogin?: boolean;
  /** 是否可编辑（AccountStoreDto.canEdit） */
  canEdit?: boolean;
}>();

defineEmits<{
  (e: "refresh"): void;
  (e: "relogin"): void;
  (e: "edit"): void;
  (e: "delete"): void;
}>();
</script>

<template>
  <div class="acc-actions">
    <button v-if="canEdit" class="icon-btn" :title="t('account.editOffline')" @click="$emit('edit')">
      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M17 3a2.8 2.8 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5z" />
      </svg>
    </button>
    <button v-if="canRefresh" class="icon-btn" :title="t('account.refresh')" @click="$emit('refresh')">
      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M21 12a9 9 0 1 1-2.64-6.36M21 3v6h-6" />
      </svg>
    </button>
    <button v-if="canRelogin" class="icon-btn" :title="t('account.relogin')" @click="$emit('relogin')">
      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4" />
        <path d="M10 17l5-5-5-5M15 12H3" />
      </svg>
    </button>
    <button class="icon-btn danger" :title="t('account.delete')" @click="$emit('delete')">
      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
        <path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6" />
      </svg>
    </button>
  </div>
</template>

<style scoped>
.acc-actions {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.icon-btn {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  flex-shrink: 0;
}

.icon-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

.icon-btn.danger:hover {
  border-color: var(--red);
  color: var(--red);
  background: rgba(255, 95, 86, 0.1);
}
</style>
