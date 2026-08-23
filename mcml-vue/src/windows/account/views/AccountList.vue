<script setup lang="ts">
// 账户列表视图：头像 / 名字 / 类型 / UUID + 操作
import { t } from "../../../lib/i18n";
import { avatarImage } from "../../../lib/accountImages";
import AccountActions from "../../../components/AccountActions.vue";
import type { Account } from "../../../lib/types";

const props = defineProps<{
  accounts: Account[];
  currentUuid: string;
  typeLabel: (acc: Account) => string;
  tokenLabel: (acc: Account) => string;
  seedOf: (uuid: string) => number;
}>();

const emit = defineEmits<{
  (e: "switch", acc: Account): void;
  (e: "refresh", acc: Account): void;
  (e: "relogin", acc: Account): void;
  (e: "delete", acc: Account): void;
}>();

function isCurrent(acc: Account): boolean {
  return acc.uuid === props.currentUuid;
}
</script>

<template>
  <div class="acc-list">
    <div
      v-for="acc in accounts"
      :key="acc.uuid"
      class="acc-row"
      :class="{ current: isCurrent(acc) }"
      @dblclick="emit('switch', acc)"
    >
      <img :src="avatarImage(seedOf(acc.uuid), acc.skin)" class="row-avatar" alt="" />
      <div class="row-meta">
        <span class="row-name">{{ acc.name }}</span>
        <span class="row-sub">{{ typeLabel(acc) }} · {{ acc.uuid }}</span>
      </div>
      <span v-if="isCurrent(acc)" class="current-tag">{{ t("account.current") }}</span>
      <span class="token-tag" :class="acc.tokenStatus">{{ tokenLabel(acc) }}</span>
      <AccountActions
        @refresh="emit('refresh', acc)"
        @relogin="emit('relogin', acc)"
        @delete="emit('delete', acc)"
      />
    </div>
    <div v-if="accounts.length === 0" class="empty-tip">{{ t("account.searchEmpty") }}</div>
  </div>
</template>

<style scoped>
.acc-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.acc-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  cursor: default;
}

.acc-row.current {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}

.row-avatar {
  width: 40px;
  height: 40px;
  border-radius: 6px;
  image-rendering: pixelated;
  flex-shrink: 0;
}

.row-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.row-name {
  font-size: 14px;
  font-weight: 700;
}

.row-sub {
  font-size: 11.5px;
  color: var(--text-dim);
  font-family: "Cascadia Code", Consolas, monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.current-tag {
  font-size: 10.5px;
  padding: 2px 8px;
  border-radius: 20px;
  background: var(--accent);
  color: #fff;
  white-space: nowrap;
  flex-shrink: 0;
}

.token-tag {
  font-size: 11px;
  padding: 2px 9px;
  border-radius: 20px;
  white-space: nowrap;
  flex-shrink: 0;
}

.token-tag.valid {
  background: rgba(62, 207, 142, 0.14);
  color: var(--green);
}

.token-tag.expired {
  background: rgba(255, 95, 86, 0.14);
  color: var(--red);
}
</style>
