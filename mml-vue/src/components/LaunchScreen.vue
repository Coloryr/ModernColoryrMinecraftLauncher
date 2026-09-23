<script setup lang="ts">
import { computed, ref, watch, nextTick } from "vue";
import { t } from "../lib/i18n";
import type { InstanceInfo, LogLine } from "../lib/bindings";
import InstanceIcon from "./InstanceIcon.vue";

const props = defineProps<{
  instance: InstanceInfo | null;
  statusText: string;
  logs: LogLine[];
  running: boolean;
}>();

const emit = defineEmits<{
  (e: "stop"): void;
}>();

const consoleEl = ref<HTMLElement | null>(null);

watch(
  () => props.logs,
  () => {
    nextTick(() => {
      if (consoleEl.value) {
        consoleEl.value.scrollTop = consoleEl.value.scrollHeight;
      }
    });
  },
);

// ================= 筛选器（线程 / 级别 / 分类） =================

const ALL = "";
const threadFilter = ref(ALL);
const levelFilter = ref(ALL);
const categoryFilter = ref(ALL);

/** 去空去重排序 */
function uniq(values: string[]): string[] {
  return [...new Set(values.filter(Boolean))].sort();
}

const threadOptions = computed(() => uniq(props.logs.map((l) => l.thread)));
const categoryOptions = computed(() => uniq(props.logs.map((l) => l.category)));

/** 级别按严重度排序 */
const levelOptions = computed(() => {
  const set = new Set(props.logs.map((l) => l.level).filter(Boolean));
  return ["Error", "Warn", "Info", "Debug"].filter((l) => set.delete(l)).concat([...set]);
});

const filteredLogs = computed(() =>
  props.logs.filter(
    (l) =>
      (threadFilter.value === ALL || l.thread === threadFilter.value) &&
      (levelFilter.value === ALL || l.level === levelFilter.value) &&
      (categoryFilter.value === ALL || l.category === categoryFilter.value),
  ),
);

watch(
  () => props.logs.length === 0,
  (empty) => {
    if (empty) {
      threadFilter.value = ALL;
      levelFilter.value = ALL;
      categoryFilter.value = ALL;
    }
  },
);
</script>

<template>
  <div class="launch-screen">
    <div class="launch-card">
      <InstanceIcon
        :name="instance?.name ?? 'M'"
        :uuid="instance?.uuid ?? '0'"
        :size="88"
      />
      <h2>{{ instance?.name ?? "启动中" }}</h2>
      <div class="status-row">
        <span class="status-dot" :class="{ running: running }"></span>
        <span class="status-text">{{ statusText }}</span>
      </div>

      <div class="filter-bar">
        <select v-model="threadFilter" class="filter-select" :title="t('launch.filterThread')">
          <option value="">{{ t("launch.filterThread") }} · {{ t("launch.filterAll") }}</option>
          <option v-for="th in threadOptions" :key="th" :value="th">{{ th }}</option>
        </select>
        <select v-model="levelFilter" class="filter-select" :title="t('launch.filterLevel')">
          <option value="">{{ t("launch.filterLevel") }} · {{ t("launch.filterAll") }}</option>
          <option v-for="lv in levelOptions" :key="lv" :value="lv">{{ lv }}</option>
        </select>
        <select v-model="categoryFilter" class="filter-select" :title="t('launch.filterCategory')">
          <option value="">{{ t("launch.filterCategory") }} · {{ t("launch.filterAll") }}</option>
          <option v-for="c in categoryOptions" :key="c" :value="c">{{ c }}</option>
        </select>
      </div>

      <div ref="consoleEl" class="console">
        <div
          v-for="(line, i) in filteredLogs"
          :key="i"
          class="log-line"
          :class="{ 'log-error': line.level === 'Error', 'log-warn': line.level === 'Warn' }"
        >
          {{ line.text }}
        </div>
        <div v-if="filteredLogs.length === 0" class="log-empty">{{ t("launch.waitLog") }}</div>
      </div>

      <button class="btn-stop" @click="emit('stop')">{{ t("launch.stop") }}</button>
    </div>
  </div>
</template>

<style scoped>
.launch-screen {
  position: fixed;
  inset: 0;
  background: rgba(10, 12, 16, 0.92);
  backdrop-filter: blur(6px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 300;
}

.launch-card {
  width: 640px;
  max-width: 92vw;
  max-height: 86vh;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 18px;
  padding: 28px 26px 22px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.55);
}

.launch-card h2 {
  font-size: 20px;
}

.status-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--yellow);
}

.status-dot.running {
  background: var(--green);
  box-shadow: 0 0 8px var(--green);
  animation: pulse 1.4s ease-in-out infinite;
}

.status-text {
  font-size: 13px;
  color: var(--text-dim);
}

.console {
  width: 100%;
  flex: 1;
  min-height: 220px;
  max-height: 46vh;
  background: #0d0f12;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 14px;
  overflow-y: auto;
  font-family: "Cascadia Code", Consolas, "Courier New", monospace;
  font-size: 12.5px;
  line-height: 1.65;
  user-select: text;
  text-align: left;
}

.filter-bar {
  width: 100%;
  display: flex;
  gap: 8px;
}

.filter-select {
  flex: 1;
  min-width: 0;
  padding: 6px 8px;
  font-size: 12px;
  color: var(--text);
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 8px;
  cursor: pointer;
}

.log-line {
  white-space: pre-wrap;
  word-break: break-all;
  color: #c8d0da;
}

.log-line.log-error {
  color: #ff7b72;
}

.log-line.log-warn {
  color: #e3b341;
}

.log-empty {
  color: #4d5560;
  text-align: center;
  margin-top: 80px;
}

.btn-stop {
  border: none;
  background: rgba(255, 95, 86, 0.14);
  color: var(--red);
  font-size: 14px;
  font-weight: 600;
  padding: 11px 34px;
  border-radius: 10px;
  cursor: pointer;
  border: 1px solid rgba(255, 95, 86, 0.4);
  transition: all 0.15s;
}

.btn-stop:hover {
  background: rgba(255, 95, 86, 0.24);
}

@keyframes pulse {
  0%,
  100% {
    opacity: 1;
  }
  50% {
    opacity: 0.4;
  }
}
</style>
