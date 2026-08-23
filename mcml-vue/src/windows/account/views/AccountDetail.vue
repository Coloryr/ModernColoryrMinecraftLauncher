<script setup lang="ts">
// 账户详情视图：全账户表格，操作按钮在左侧
import { t } from "../../../lib/i18n";
import AccountActions from "../../../components/AccountActions.vue";
import type { Account } from "../../../lib/types";

const props = defineProps<{
  accounts: Account[];
  currentUuid: string;
  typeLabel: (acc: Account) => string;
  tokenLabel: (acc: Account) => string;
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
  <div class="acc-detail">
    <table class="detail-table wide">
      <thead>
        <tr>
          <th class="col-actions">{{ t("account.actions") }}</th>
          <th>{{ t("account.name") }}</th>
          <th>{{ t("account.uuid") }}</th>
          <th>{{ t("account.type") }}</th>
          <th>{{ t("account.lastLogin") }}</th>
          <th>{{ t("account.tokenStatus") }}</th>
          <th>{{ t("account.capeName") }}</th>
        </tr>
      </thead>
      <tbody>
        <tr
          v-for="acc in accounts"
          :key="acc.uuid"
          :class="{ current: isCurrent(acc) }"
          @dblclick="emit('switch', acc)"
        >
          <td class="col-actions">
            <AccountActions
              @refresh="emit('refresh', acc)"
              @relogin="emit('relogin', acc)"
              @delete="emit('delete', acc)"
            />
          </td>
          <td class="cell-name">
            {{ acc.name }}
            <span v-if="isCurrent(acc)" class="current-tag">{{ t("account.current") }}</span>
          </td>
          <td class="mono">{{ acc.uuid }}</td>
          <td>{{ typeLabel(acc) }}</td>
          <td>{{ acc.lastLogin }}</td>
          <td>
            <span class="token-tag" :class="acc.tokenStatus">{{ tokenLabel(acc) }}</span>
          </td>
          <td>{{ acc.name }}_cape</td>
        </tr>
      </tbody>
    </table>
    <div v-if="accounts.length === 0" class="empty-tip">{{ t("account.searchEmpty") }}</div>
  </div>
</template>

<style scoped>
.acc-detail {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.detail-table.wide {
  border-collapse: collapse;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
  width: 100%;
}

.detail-table th,
.detail-table td {
  padding: 10px 14px;
  font-size: 12.5px;
  border-bottom: 1px solid var(--border);
  text-align: left;
  white-space: nowrap;
}

.detail-table thead th {
  color: var(--text-dim);
  font-weight: 600;
  background: var(--bg-side);
}

.detail-table tbody tr:last-child th,
.detail-table tbody tr:last-child td {
  border-bottom: none;
}

.detail-table tbody tr.current td {
  background: var(--accent-soft);
}

.col-actions {
  width: 108px;
}

.cell-name {
  font-weight: 700;
}

.current-tag {
  font-size: 10.5px;
  padding: 2px 8px;
  border-radius: 20px;
  background: var(--accent);
  color: #fff;
  white-space: nowrap;
  flex-shrink: 0;
  margin-left: 6px;
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

.mono {
  font-family: "Cascadia Code", Consolas, monospace;
}
</style>
