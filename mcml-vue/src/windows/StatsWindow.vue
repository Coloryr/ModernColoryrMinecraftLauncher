<script setup lang="ts">
import { onMounted, ref } from "vue";
import WindowFrame from "../components/ui/WindowFrame.vue";
import InstanceIcon from "../components/InstanceIcon.vue";
import { api } from "../lib/api";
import { t } from "../lib/i18n";
import type { InstanceInfo } from "../lib/types";

const instances = ref<InstanceInfo[]>([]);

// 模拟统计数据
const stats = new Map<string, { count: number; hours: number; last: string }>([
  ["11111111-1111-4111-8111-111111111111", { count: 23, hours: 18.5, last: "2026-04-12 21:30" }],
  ["22222222-2222-4222-8222-222222222222", { count: 56, hours: 41.2, last: "2026-04-15 19:05" }],
  ["33333333-3333-4333-8333-333333333333", { count: 9, hours: 6.8, last: "2026-03-28 22:11" }],
  ["44444444-4444-4444-8444-444444444444", { count: 31, hours: 27.4, last: "2026-04-10 20:44" }],
]);

const totalLaunch = 119;
const totalHours = 93.9;

function statOf(uuid: string) {
  return stats.get(uuid) ?? { count: 0, hours: 0, last: t("detail.none") };
}

onMounted(async () => {
  try {
    instances.value = await api.getInstances();
  } catch {
    instances.value = [];
  }
});
</script>

<template>
  <WindowFrame :title="t('features.stats')" @close="$emit('close')">
    <div class="summary">
      <div class="summary-card">
        <span class="summary-num">{{ instances.length }}</span>
        <span class="summary-label">{{ t("winStats.instances") }}</span>
      </div>
      <div class="summary-card">
        <span class="summary-num">{{ totalLaunch }}</span>
        <span class="summary-label">{{ t("winStats.totalLaunch") }}</span>
      </div>
      <div class="summary-card">
        <span class="summary-num">{{ totalHours }}h</span>
        <span class="summary-label">{{ t("winStats.totalPlay") }}</span>
      </div>
    </div>

    <table class="stats-table">
      <thead>
        <tr>
          <th>{{ t("winStats.colInstance") }}</th>
          <th>{{ t("winStats.colCount") }}</th>
          <th>{{ t("winStats.colHours") }}</th>
          <th>{{ t("winStats.colLast") }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="inst in instances" :key="inst.uuid">
          <td class="inst-cell">
            <InstanceIcon :name="inst.name" :uuid="inst.uuid" :size="30" />
            <span>{{ inst.name }}</span>
          </td>
          <td>{{ statOf(inst.uuid).count }}</td>
          <td>{{ t("winStats.hours", { h: statOf(inst.uuid).hours }) }}</td>
          <td>{{ statOf(inst.uuid).last }}</td>
        </tr>
        <tr v-if="instances.length === 0">
          <td colspan="4" class="empty-cell">{{ t("winStats.noData") }}</td>
        </tr>
      </tbody>
    </table>

    <p class="foot-note">{{ t("winStats.foot") }}</p>
  </WindowFrame>
</template>

<style scoped>
.summary {
  display: flex;
  gap: 14px;
  margin-bottom: 18px;
}

.summary-card {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
}

.summary-num {
  font-size: 22px;
  font-weight: 800;
  color: var(--accent);
}

.summary-label {
  font-size: 12px;
  color: var(--text-dim);
}

.stats-table {
  width: 100%;
  border-collapse: collapse;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}

.stats-table th {
  text-align: left;
  font-size: 12px;
  color: var(--text-dim);
  font-weight: 600;
  padding: 12px 16px;
  background: var(--bg-side);
  border-bottom: 1px solid var(--border);
}

.stats-table td {
  padding: 11px 16px;
  font-size: 13px;
  border-bottom: 1px solid var(--border);
}

.stats-table tr:last-child td {
  border-bottom: none;
}

.inst-cell {
  display: flex;
  align-items: center;
  gap: 10px;
  font-weight: 600;
}

.empty-cell {
  text-align: center;
  color: var(--text-dim);
  padding: 24px;
}

.foot-note {
  font-size: 11.5px;
  color: var(--text-dim);
  margin-top: 10px;
}
</style>
