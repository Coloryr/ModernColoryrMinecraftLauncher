<script setup lang="ts">
// 资源下载进度条：每个任务一条下载进度（0–100%），终态带标签。
// 完成后后端保留一段时间再自动移除，这里只做展示。
// 添加资源窗口常显；主窗口仅在添加资源窗口关闭时显示（由调用方 v-if 控制）。
import { computed } from "vue";
import type { ResourceStatusDto, ResourceTaskDto } from "../lib/bindings";
import { t } from "../lib/i18n";

const props = defineProps<{
  status: ResourceStatusDto;
}>();

/** 总进度 = 各任务进度的均值（没有任务时 100） */
const percent = computed(() => {
  const running = props.status.tasks.filter((task) => !task.done && !task.failed);
  if (!running.length) return 100;
  return running.reduce((acc, task) => acc + task.progress, 0) / running.length;
});

function stateTag(task: ResourceTaskDto): string | null {
  if (task.done) return t("addResource.bar.done");
  if (task.failed) return t("addResource.bar.failed");
  return null;
}
</script>

<template>
  <div class="res-bar">
    <div class="res-bar-head">
      <span class="res-bar-title">{{ t("addResource.bar.running", { count: status.tasks.length }) }}</span>
    </div>
    <div class="progress-track">
      <div class="progress-fill" :style="{ width: percent + '%' }" />
    </div>

    <!-- 每个任务的下载进度 -->
    <div v-if="status.tasks.length > 1" class="res-task-list">
      <div v-for="task in status.tasks" :key="task.pid + task.fid" class="res-task">
        <div class="res-task-head">
          <span class="res-task-name">{{ task.name }}</span>
          <span v-if="stateTag(task)" class="res-tag" :class="{ done: task.done, failed: task.failed }">
            {{ stateTag(task) }}
          </span>
          <span v-else class="res-task-percent">{{ task.progress.toFixed(0) }}%</span>
        </div>
        <div class="progress-track sub">
          <div class="progress-fill" :style="{ width: task.progress + '%' }" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.res-bar {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 12px;
  border-radius: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.res-bar-head {
  display: flex;
  align-items: center;
  gap: 8px;
}

.res-bar-title {
  flex: 1;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.res-task-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding-top: 2px;
}

.res-task {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.res-task-head {
  display: flex;
  align-items: center;
  gap: 8px;
}

.res-task-name {
  flex: 1;
  font-size: 12px;
  color: var(--text);
  word-break: break-all;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.res-task-percent {
  font-size: 11px;
  color: var(--text-dim);
  flex-shrink: 0;
}

.res-tag {
  font-size: 11px;
  padding: 1px 8px;
  border-radius: 999px;
  flex-shrink: 0;
}

.res-tag.done {
  color: var(--green);
  background: color-mix(in srgb, var(--green) 14%, transparent);
}

.res-tag.failed {
  color: var(--red);
  background: color-mix(in srgb, var(--red) 14%, transparent);
}

.progress-track {
  height: 8px;
  border-radius: 4px;
  background: var(--bg-hover);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 4px;
  background: var(--accent-grad);
  transition: width 0.3s;
}

.progress-track.sub {
  height: 6px;
}
</style>
