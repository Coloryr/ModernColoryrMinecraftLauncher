<script setup lang="ts">
// 下载管理窗口：对接 mcml_downloader
// - 任务列表来自 download_get_tasks 快照 + download-task 事件（add / remove / update）
// - 各下载线程当前文件来自 download-item 事件（后端已按线程去重）
import { computed, onMounted, onUnmounted, reactive, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import { api, onDownloadItem, onDownloadTask } from "../../lib/api";
import { t } from "../../lib/i18n";
import type { DownloadItemEvent } from "../../lib/types";

/** 下载任务（progress 来自事件，其余来自快照） */
interface DownloadTask {
  id: number;
  progress: number;
  total: number;
  completed: number;
  failed: number;
}

/** 下载线程当前文件 */
interface ThreadState {
  thread: number;
  name: string;
  state: string;
}

const tasks = reactive(new Map<number, DownloadTask>());
const threads = reactive(new Map<number, ThreadState>());
const loading = ref(true);

let snapshotTimer: number | null = null;
let unsubs: Array<() => void> = [];

const taskList = computed(() => [...tasks.values()].sort((a, b) => a.id - b.id));
const activeThreads = computed(() => [...threads.values()].filter((s) => s.state !== "done"));

async function refreshSnapshot() {
  try {
    const list = await api.getDownloadTasks();
    for (const snap of list) {
      const cur = tasks.get(snap.id);
      if (cur) {
        cur.total = snap.total;
        cur.completed = snap.completed;
        cur.failed = snap.failed;
      }
    }
  } catch {
    // 纯浏览器模式忽略
  }
}

async function load() {
  try {
    const list = await api.getDownloadTasks();
    tasks.clear();
    for (const snap of list) {
      tasks.set(snap.id, {
        id: snap.id,
        progress: 0,
        total: snap.total,
        completed: snap.completed,
        failed: snap.failed,
      });
    }
  } catch {
    // 纯浏览器模式忽略
  } finally {
    loading.value = false;
  }
}

async function cancel(id: number) {
  await api.cancelDownloadTask(id);
}

function taskStateLabel(state: string): string {
  return t(`winDownload.state.${state}`);
}

onMounted(async () => {
  await load();

  unsubs.push(
    await onDownloadTask((e) => {
      if (e.type === "add") {
        tasks.set(e.id, { id: e.id, progress: 0, total: 0, completed: 0, failed: 0 });
        void refreshSnapshot();
      } else if (e.type === "remove") {
        tasks.delete(e.id);
      } else {
        const cur = tasks.get(e.id);
        if (cur) {
          cur.progress = e.progress;
        }
      }
    }),
  );

  unsubs.push(
    await onDownloadItem((e: DownloadItemEvent) => {
      threads.set(e.thread, e);
    }),
  );

  // 定期刷新任务计数（事件只带进度）
  snapshotTimer = window.setInterval(refreshSnapshot, 1000);
});

onUnmounted(() => {
  if (snapshotTimer !== null) {
    window.clearInterval(snapshotTimer);
  }
  unsubs.forEach((fn) => fn());
  unsubs = [];
});
</script>

<template>
  <WindowFrame :title="t('features.download')" @close="$emit('close')">
    <div class="download-body">
      <!-- 任务列表 -->
      <div class="section">
        <div class="section-title">{{ t("winDownload.tasks") }}</div>
        <div v-if="loading" class="empty">{{ t("winDownload.loading") }}</div>
        <div v-else-if="taskList.length === 0" class="empty">{{ t("winDownload.empty") }}</div>
        <div v-else class="task-list">
          <div v-for="task in taskList" :key="task.id" class="task-item">
            <div class="task-head">
              <span class="task-id">#{{ task.id }}</span>
              <span class="task-counts">
                {{ t("winDownload.counts", { done: task.completed, failed: task.failed, total: task.total || "?" }) }}
              </span>
              <BaseButton variant="danger" size="sm" @click="cancel(task.id)">
                {{ t("winDownload.cancel") }}
              </BaseButton>
            </div>
            <div class="progress-track">
              <div
                class="progress-fill"
                :class="{ failed: task.failed > 0 }"
                :style="{ width: Math.min(100, Math.max(0, task.progress)) + '%' }"
              />
            </div>
            <div class="task-progress">{{ task.progress.toFixed(1) }}%</div>
          </div>
        </div>
      </div>

      <!-- 下载线程 -->
      <div class="section">
        <div class="section-title">{{ t("winDownload.threads") }}</div>
        <div v-if="activeThreads.length === 0" class="empty">{{ t("winDownload.idle") }}</div>
        <div v-else class="thread-list">
          <div v-for="item in activeThreads" :key="item.thread" class="thread-item">
            <span class="thread-id">#{{ item.thread + 1 }}</span>
            <span class="thread-name" :title="item.name">{{ item.name }}</span>
            <span class="thread-state" :class="item.state">{{ taskStateLabel(item.state) }}</span>
          </div>
        </div>
      </div>
    </div>
  </WindowFrame>
</template>

<style scoped>
.download-body {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.section {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.section-title {
  font-size: 13px;
  font-weight: 700;
  color: var(--text-dim);
}

.empty {
  padding: 22px 16px;
  background: var(--bg-card);
  border: 1px dashed var(--border);
  border-radius: 12px;
  text-align: center;
  font-size: 13px;
  color: var(--text-dim);
}

.task-list,
.thread-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.task-item {
  padding: 14px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.task-head {
  display: flex;
  align-items: center;
  gap: 12px;
}

.task-id {
  font-weight: 700;
  font-size: 13.5px;
  color: var(--accent);
}

.task-counts {
  flex: 1;
  font-size: 12.5px;
  color: var(--text-dim);
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

.progress-fill.failed {
  background: var(--red);
}

.task-progress {
  font-size: 12px;
  color: var(--text-dim);
  text-align: right;
}

.thread-item {
  padding: 11px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  display: flex;
  align-items: center;
  gap: 12px;
  font-size: 13px;
}

.thread-id {
  font-weight: 600;
  color: var(--text-dim);
  flex-shrink: 0;
}

.thread-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.thread-state {
  flex-shrink: 0;
  font-size: 12px;
  padding: 2px 10px;
  border-radius: 999px;
  background: var(--bg-hover);
  color: var(--text-dim);
}

.thread-state.done {
  color: var(--green, #4caf7d);
}

.thread-state.error {
  color: var(--red);
}
</style>
