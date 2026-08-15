<script setup lang="ts">
import { ref, watch, nextTick } from "vue";
import { t } from "../lib/i18n";
import type { InstanceInfo } from "../lib/types";
import InstanceIcon from "./InstanceIcon.vue";

const props = defineProps<{
  instance: InstanceInfo | null;
  statusText: string;
  logs: string[];
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

      <div ref="consoleEl" class="console">
        <div v-for="(line, i) in logs" :key="i" class="log-line">{{ line }}</div>
        <div v-if="logs.length === 0" class="log-empty">{{ t("launch.waitLog") }}</div>
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

.log-line {
  white-space: pre-wrap;
  word-break: break-all;
  color: #c8d0da;
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
