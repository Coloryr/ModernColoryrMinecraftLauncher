<script setup lang="ts">
// 下载管理窗口：对接 mcml_downloader
// - 状态快照来自 download_get_status（任务 + 线程 + 总体速度），按固定间隔轮询
// - 任务 / 文件状态事件触发立即刷新（进度本身靠轮询，避免高频重绘）
// - 每个任务可暂停 / 继续 / 停止
import { computed, onMounted, onUnmounted, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import { api, onCloseBlocked, onDownloadItem, onDownloadTask } from "../../lib/api";
import { t } from "../../lib/i18n";
import type { DownloadStatus, DownloadTaskInfo, DownloadThreadInfo } from "../../lib/types";

const emit = defineEmits<{ (e: "close"): void }>();

/** 轮询间隔（毫秒） */
const REFRESH_MS = 700;

const status = ref<DownloadStatus>({ tasks: [], threads: [], speed: 0, paused: false });
const loading = ref(true);

/** 停止下载确认弹窗 */
const confirmStop = ref(false);
/** 关闭窗口确认弹窗（有下载任务时） */
const confirmClose = ref(false);

let timer: number | null = null;
let unsubs: Array<() => void> = [];

const tasks = computed(() => [...status.value.tasks].sort((a, b) => a.id - b.id));
/** 线程列表：已完成的线程不占位 */
const threads = computed(() => status.value.threads.filter((x) => x.state !== "done"));

// ================= 进度计算 =================

/** 单任务进度：优先按字节，元信息未知时退回文件数 */
function taskProgress(task: DownloadTaskInfo): number {
  if (task.allBytes > 0) {
    return Math.min(100, (task.nowBytes / task.allBytes) * 100);
  }
  if (task.total > 0) {
    return Math.min(100, ((task.completed + task.failed) / task.total) * 100);
  }
  return 0;
}

/** 总体进度：按字节加权，未知时按文件数 */
const overall = computed(() => {
  const list = tasks.value;
  const all = list.reduce((n, x) => n + x.allBytes, 0);
  if (all > 0) {
    const now = list.reduce((n, x) => n + x.nowBytes, 0);
    return Math.min(100, (now / all) * 100);
  }
  const total = list.reduce((n, x) => n + x.total, 0);
  const done = list.reduce((n, x) => n + x.completed + x.failed, 0);
  return total > 0 ? Math.min(100, (done / total) * 100) : 0;
});

/** 总体字节：已下载 / 总大小 */
const overallBytes = computed(() => ({
  now: tasks.value.reduce((n, x) => n + x.nowBytes, 0),
  all: tasks.value.reduce((n, x) => n + x.allBytes, 0),
}));

/** 文件计数汇总 */
const overallFiles = computed(() => ({
  done: tasks.value.reduce((n, x) => n + x.completed, 0),
  total: tasks.value.reduce((n, x) => n + x.total, 0),
}));

// ================= 任务状态（标签文案 + 配色） =================

type TaskState = "running" | "paused" | "failed" | "done";

function taskState(task: DownloadTaskInfo): TaskState {
  if (task.paused) return "paused";
  if (task.total > 0 && task.completed + task.failed >= task.total) {
    return task.failed > 0 ? "failed" : "done";
  }
  return task.failed > 0 ? "failed" : "running";
}

// ================= 格式化 =================

function formatBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let index = 0;
  while (value >= 1024 && index < units.length - 1) {
    value /= 1024;
    index += 1;
  }
  const text = index === 0 || value >= 100 ? value.toFixed(0) : value.toFixed(1);
  return `${text} ${units[index]}`;
}

/** 速度：无速度时显示占位符 */
function formatSpeed(speed: number): string {
  return speed > 0 ? `${formatBytes(speed)}/s` : "—";
}

/** 已进行时间：不足 1 小时为 mm:ss，否则 h:mm:ss */
function formatDuration(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const hour = Math.floor(total / 3600);
  const minute = Math.floor((total % 3600) / 60);
  const second = total % 60;
  const pad = (n: number) => String(n).padStart(2, "0");
  return hour > 0 ? `${hour}:${pad(minute)}:${pad(second)}` : `${pad(minute)}:${pad(second)}`;
}

/** 大小文本：总大小未知时只显示已下载 */
function sizeText(now: number, all: number): string {
  return all > 0 ? `${formatBytes(now)} / ${formatBytes(all)}` : formatBytes(now);
}

function stateLabel(state: string): string {
  return t(`winDownload.state.${state}`);
}

// ================= 数据刷新 =================

async function refresh() {
  try {
    status.value = await api.getDownloadStatus();
  } catch {
    /* 纯浏览器模式：无 IPC，保持空状态 */
  } finally {
    loading.value = false;
  }
}

/** 是否已全部暂停（后端全局状态） */
const allPaused = computed(() => status.value.paused);
/** 是否有任务可操作（决定全局按钮是否可用） */
const hasTasks = computed(() => tasks.value.length > 0);

async function pauseAll() {
  if (usingFake) {
    fakePaused = true;
    status.value = fakeStatus();
    return;
  }
  await api.pauseAllDownloads();
  await refresh();
}

async function resumeAll() {
  if (usingFake) {
    fakePaused = false;
    fakeStopped = false;
    status.value = fakeStatus();
    return;
  }
  await api.resumeAllDownloads();
  await refresh();
}

async function stopAll() {
  if (usingFake) {
    fakeStopped = true;
    fakePaused = false;
    status.value = fakeStatus();
    return;
  }
  await api.stopAllDownloads();
  await refresh();
}

/** 点停止：先二次确认 */
function requestStop() {
  confirmStop.value = true;
}

/** 确认停止所有下载 */
async function confirmStopAll() {
  confirmStop.value = false;
  await stopAll();
}

/** 点关闭：有下载任务时先确认（无任务直接关） */
function requestClose() {
  if (hasTasks.value) {
    confirmClose.value = true;
    return;
  }
  emit("close");
}

/** 确认停止所有下载并关闭窗口（停止后任务已清空，Rust 侧不再拦截关闭） */
async function confirmCloseAll() {
  confirmClose.value = false;
  await stopAll();
  emit("close");
}

onMounted(async () => {
  await refresh();

  unsubs.push(await onDownloadTask(() => void refresh()));
  unsubs.push(await onDownloadItem(() => void refresh()));
  // 原生标题栏 X / 关闭命令被 Rust 拒绝时，弹确认框
  unsubs.push(
    await onCloseBlocked(() => {
      confirmClose.value = true;
    }),
  );

  timer = window.setInterval(refresh, REFRESH_MS);
});

onUnmounted(() => {
  if (timer !== null) {
    window.clearInterval(timer);
  }
  unsubs.forEach((fn) => fn());
  unsubs = [];
});

// ================= TEMP-假数据（看完删除） =================
// 说明：真实下载任务为空时用一组会随时间推进的假任务演示界面
// （进度会走、速度会变、时间是活的；暂停 / 继续 / 停止按钮在假数据下操作本地演示状态）
let fakeTick = 0;
let fakePaused = false;
let fakeStopped = false;

function fakeStatus(): DownloadStatus {
  if (fakeStopped) {
    return { tasks: [], threads: [], speed: 0, paused: false };
  }

  if (!fakePaused) {
    fakeTick += 1;
  }
  const MB = 1024 * 1024;
  const wave = (base: number, amp: number, phase: number) =>
    Math.max(0, Math.round(base + Math.sin(fakeTick / 3 + phase) * amp));

  const task1Now = Math.min(861 * MB, 361 * MB + fakeTick * 4 * MB);
  const thread0Speed = fakePaused ? 0 : wave(1.8 * MB, 0.6 * MB, 0);
  const thread1Speed = fakePaused ? 0 : wave(0.9 * MB, 0.3 * MB, 1.2);
  const thread2Speed = fakePaused || fakeTick % 4 !== 0 ? 0 : wave(0.5 * MB, 0.2 * MB, 2.4);

  const threads: DownloadThreadInfo[] = [
    {
      thread: 0,
      name: "assets/objects/1a/1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d7e8f9a0b",
      state: "download",
      progress: Math.min(100, 10 + fakeTick * 1.5),
      nowBytes: 12 * MB,
      allBytes: 34 * MB,
      speed: thread0Speed,
    },
    {
      thread: 1,
      name: "libraries/net/fabricmc/fabric-loader/0.16.9/fabric-loader-0.16.9.jar",
      state: "download",
      progress: Math.min(100, 40 + fakeTick * 2.2),
      nowBytes: 2 * MB,
      allBytes: 5 * MB,
      speed: thread1Speed,
    },
    {
      thread: 2,
      name: "assets/indexes/19.json",
      state: "getinfo",
      progress: 0,
      nowBytes: 0,
      allBytes: 0,
      speed: thread2Speed,
    },
    {
      thread: 3,
      name: "versions/1.21.4/1.21.4.jar",
      state: "error",
      progress: 63.2,
      nowBytes: 15 * MB,
      allBytes: 24 * MB,
      speed: 0,
    },
    {
      thread: 4,
      name: "resourcepacks/Faithful-64x.zip",
      state: "pause",
      progress: 28.4,
      nowBytes: 18 * MB,
      allBytes: 64 * MB,
      speed: 0,
    },
    {
      thread: 5,
      name: "mods/sodium-fabric-0.6.5.jar",
      state: "action",
      progress: 100,
      nowBytes: 1 * MB,
      allBytes: 1 * MB,
      speed: 0,
    },
    {
      thread: 6,
      name: "logs/latest.log",
      state: "wait",
      progress: 0,
      nowBytes: 0,
      allBytes: 0,
      speed: 0,
    },
  ];

  // 暂停时线程状态同步显示为已暂停（与后端行为一致）
  const finalThreads = fakePaused
    ? threads.map((x) =>
        x.state === "download" || x.state === "getinfo"
          ? { ...x, state: "pause", speed: 0 }
          : x,
      )
    : threads;

  return {
    speed: thread0Speed + thread1Speed + thread2Speed,
    paused: fakePaused,
    tasks: [
      {
        id: 1,
        total: 356,
        completed: Math.min(356, 44 + Math.floor(fakeTick / 3)),
        failed: 0,
        allBytes: 861 * MB,
        nowBytes: task1Now,
        elapsedMs: fakeTick * REFRESH_MS + 42_000,
        paused: fakePaused,
      },
      {
        id: 2,
        total: 128,
        completed: 82,
        failed: 2,
        allBytes: 132 * MB,
        nowBytes: 84 * MB,
        elapsedMs: 254_000,
        paused: true,
      },
      {
        id: 3,
        total: 6,
        completed: 6,
        failed: 0,
        allBytes: 24 * MB,
        nowBytes: 24 * MB,
        elapsedMs: 31_000,
        paused: false,
      },
    ],
    threads: finalThreads,
  };
}
</script>

<template>
  <WindowFrame :title="t('features.download')" @close="requestClose">
    <div class="download-body">
      <!-- 总览：任务总量 / 文件数 / 大小 / 总体速度 + 总体进度条 -->
      <div class="overview">
        <div class="overview-head">
          <span class="ov-item">{{ t("winDownload.taskCount", { n: tasks.length }) }}</span>
          <span class="ov-item">
            {{ t("winDownload.files", { done: overallFiles.done, total: overallFiles.total }) }}
          </span>
          <span v-if="overallBytes.all > 0" class="ov-item">
            {{ sizeText(overallBytes.now, overallBytes.all) }}
          </span>
          <span class="ov-speed">
            <span class="ov-speed-label">{{ t("winDownload.totalSpeed") }}</span>
            <span class="ov-speed-value" :class="{ idle: status.speed <= 0 }">
              {{ formatSpeed(status.speed) }}
            </span>
          </span>
          <!-- 全局控制：暂停 / 继续 / 停止（作用于所有下载任务） -->
          <span class="ov-actions">
            <button
              v-if="!allPaused"
              class="mini-btn"
              :disabled="!hasTasks"
              @click="pauseAll"
            >
              {{ t("winDownload.pause") }}
            </button>
            <button
              v-else
              class="mini-btn primary"
              :disabled="!hasTasks"
              @click="resumeAll"
            >
              {{ t("winDownload.resume") }}
            </button>
            <button class="mini-btn danger" :disabled="!hasTasks" @click="requestStop">
              {{ t("winDownload.stop") }}
            </button>
          </span>
        </div>
        <div class="ov-progress">
          <span class="progress-track ov-track">
            <span class="progress-fill" :style="{ width: overall + '%' }" />
          </span>
          <span class="ov-percent">{{ overall.toFixed(1) }}%</span>
        </div>
      </div>

      <!-- 下载任务 -->
      <div class="section">
        <div class="section-head">
          <span class="section-title">{{ t("winDownload.tasks") }}</span>
        </div>
        <div v-if="loading" class="empty">{{ t("winDownload.loading") }}</div>
        <div v-else-if="tasks.length === 0" class="empty">{{ t("winDownload.empty") }}</div>
        <div v-else class="task-list">
          <div v-for="task in tasks" :key="task.id" class="task-item">
            <div class="task-head">
              <span class="task-id">#{{ task.id }}</span>
              <span class="state-tag" :class="'ts-' + taskState(task)">
                {{ t(`winDownload.taskState.${taskState(task)}`) }}
              </span>
              <span class="task-meta">
                {{ t("winDownload.files", { done: task.completed, total: task.total }) }}
              </span>
              <span class="task-meta right">
                <template v-if="task.failed > 0">{{ t("winDownload.failed", { n: task.failed }) }}</template>
              </span>
              <span class="task-meta right">{{ sizeText(task.nowBytes, task.allBytes) }}</span>
              <span class="task-meta right">
                {{ t("winDownload.elapsed", { time: formatDuration(task.elapsedMs) }) }}
              </span>
            </div>
            <div class="task-foot">
              <span class="progress-track">
                <span
                  class="progress-fill"
                  :class="{ failed: task.failed > 0, paused: task.paused }"
                  :style="{ width: taskProgress(task) + '%' }"
                />
              </span>
              <span class="task-percent">{{ taskProgress(task).toFixed(1) }}%</span>
            </div>
          </div>
        </div>
      </div>

      <!-- 下载线程：当前文件进度 + 速度 -->
      <div class="section">
        <div class="section-head">
          <span class="section-title">{{ t("winDownload.threads") }}</span>
        </div>
        <div v-if="threads.length === 0" class="empty">{{ t("winDownload.idle") }}</div>
        <div v-else class="thread-list">
          <div v-for="item in threads" :key="item.thread" class="thread-item">
            <span class="thread-id">#{{ item.thread + 1 }}</span>
            <span class="thread-name" :title="item.name">{{ item.name }}</span>
            <span class="thread-progress">
              <span class="progress-track">
                <span class="progress-fill" :style="{ width: item.progress + '%' }" />
              </span>
              <span class="thread-percent">{{ item.progress.toFixed(1) }}%</span>
            </span>
            <span class="thread-size">{{ sizeText(item.nowBytes, item.allBytes) }}</span>
            <span class="thread-speed" :class="{ idle: item.speed <= 0 }">
              {{ formatSpeed(item.speed) }}
            </span>
            <span class="state-tag" :class="'state-' + item.state">{{ stateLabel(item.state) }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 停止下载确认 -->
    <BaseModal v-if="confirmStop" :title="t('winDownload.stopTitle')" @close="confirmStop = false">
      <p class="delete-tip">{{ t("winDownload.stopConfirm") }}</p>
      <div class="modal-actions">
        <BaseButton @click="confirmStop = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" @click="confirmStopAll">
          {{ t("winDownload.stop") }}
        </BaseButton>
      </div>
    </BaseModal>

    <!-- 关闭窗口确认（有下载任务时） -->
    <BaseModal v-if="confirmClose" :title="t('winDownload.closeTitle')" @close="confirmClose = false">
      <p class="delete-tip">{{ t("winDownload.closeConfirm") }}</p>
      <div class="modal-actions">
        <BaseButton @click="confirmClose = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" @click="confirmCloseAll">
          {{ t("winDownload.stopAndClose") }}
        </BaseButton>
      </div>
    </BaseModal>
  </WindowFrame>
</template>

<style scoped>
.download-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* ---------- 总览 ---------- */
.overview {
  display: flex;
  flex-direction: column;
  gap: 7px;
  padding: 10px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
}

.overview-head {
  display: flex;
  align-items: center;
  gap: 14px;
  font-size: 12.5px;
  color: var(--text-dim);
}

.ov-item:first-child {
  color: var(--text);
  font-weight: 600;
}

.ov-speed {
  margin-left: auto;
  display: flex;
  align-items: center;
  gap: 6px;
}

/* 全局控制按钮（暂停 / 继续 / 停止） */
.ov-actions {
  display: flex;
  gap: 6px;
}

.ov-speed-label {
  font-size: 11.5px;
}

.ov-speed-value {
  font-size: 13px;
  font-weight: 700;
  color: var(--green, #4caf7d);
  font-variant-numeric: tabular-nums;
}

.ov-speed-value.idle {
  color: var(--text-dim);
  font-weight: 500;
}

/* 总体进度：进度条 + 右侧百分比 */
.ov-progress {
  display: flex;
  align-items: center;
  gap: 8px;
}

.ov-percent {
  flex-shrink: 0;
  width: 48px;
  text-align: right;
  font-size: 11.5px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.ov-track {
  height: 8px;
}

/* ---------- 分区 ---------- */
.section {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.section-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.section-title {
  font-size: 12.5px;
  font-weight: 700;
  color: var(--text-dim);
}

.empty {
  padding: 16px;
  background: var(--bg-card);
  border: 1px dashed var(--border);
  border-radius: 10px;
  text-align: center;
  font-size: 12.5px;
  color: var(--text-dim);
}

/* ---------- 任务 ---------- */
.task-list,
.thread-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.task-item {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 8px 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
}

/* 任务行：数字/时间列贴右，多余宽度只留在「文件」列（表格常规做法） */
.task-head {
  display: grid;
  grid-template-columns: 26px 86px minmax(0, 1fr) 54px 122px 100px;
  gap: 8px;
  align-items: center;
  font-size: 12px;
}

.task-id {
  font-weight: 700;
  color: var(--accent);
  font-variant-numeric: tabular-nums;
}

.task-meta {
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  font-variant-numeric: tabular-nums;
}

.task-meta.right {
  text-align: right;
}

.mini-btn {
  padding: 3px 10px;
  border: 1px solid var(--border);
  border-radius: 7px;
  background: transparent;
  color: var(--text);
  font-size: 11.5px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
}

.mini-btn:disabled {
  opacity: 0.45;
  cursor: not-allowed;
  pointer-events: none;
}

.mini-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.mini-btn.primary {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

.mini-btn.danger:hover {
  border-color: var(--red);
  color: var(--red);
}

.task-foot {
  display: flex;
  align-items: center;
  gap: 8px;
}

.task-percent {
  flex-shrink: 0;
  width: 46px;
  text-align: right;
  font-size: 11.5px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

/* ---------- 进度条 ---------- */
.progress-track {
  flex: 1;
  display: block;
  height: 6px;
  border-radius: 3px;
  background: var(--bg-hover);
  overflow: hidden;
}

.progress-fill {
  display: block;
  height: 100%;
  border-radius: 3px;
  background: var(--accent-grad);
  transition: width 0.3s;
}

.progress-fill.failed {
  background: var(--red);
}

.progress-fill.paused {
  background: var(--yellow, #f5b944);
}

/* ---------- 线程 ---------- */
/* 线程行：固定列宽（信息列紧邻），状态列放得下最长的标签（"获取信息" / "Fetching Info"） */
.thread-item {
  display: grid;
  grid-template-columns: 26px minmax(0, 1fr) 116px 108px 62px 88px;
  gap: 8px;
  align-items: center;
  padding: 6px 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 9px;
  font-size: 12px;
}

.thread-id {
  font-weight: 600;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.thread-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.thread-progress {
  display: flex;
  align-items: center;
  gap: 7px;
}

.thread-percent {
  flex-shrink: 0;
  width: 40px;
  text-align: right;
  font-size: 11.5px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.thread-size {
  text-align: right;
  font-size: 11.5px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.thread-speed {
  text-align: right;
  font-size: 11.5px;
  font-weight: 600;
  color: var(--green, #4caf7d);
  font-variant-numeric: tabular-nums;
}

.thread-speed.idle {
  color: var(--text-dim);
  font-weight: 500;
}

/* ---------- 状态标签（多色） ---------- */
.state-tag {
  padding: 1px 8px;
  border-radius: 999px;
  font-size: 11px;
  white-space: nowrap;
  text-align: center;
  overflow: hidden;
  text-overflow: ellipsis;
  background: var(--bg-hover);
  color: var(--text-dim);
}

/* 任务状态 */
.ts-running {
  color: #6aa8ff;
  background: rgba(63, 140, 255, 0.16);
}

.ts-paused {
  color: var(--yellow, #f5b944);
  background: rgba(245, 185, 68, 0.16);
}

.ts-failed {
  color: #ff8080;
  background: rgba(255, 90, 90, 0.16);
}

.ts-done {
  color: #5ecf99;
  background: rgba(76, 175, 125, 0.16);
}

/* 线程状态 */
.state-download {
  color: #6aa8ff;
  background: rgba(63, 140, 255, 0.16);
}

.state-getinfo {
  color: #4fd1e0;
  background: rgba(6, 182, 212, 0.16);
}

.state-init {
  color: #b98cff;
  background: rgba(168, 85, 247, 0.16);
}

.state-action {
  color: #ffab5c;
  background: rgba(255, 150, 56, 0.16);
}

.state-pause {
  color: var(--yellow, #f5b944);
  background: rgba(245, 185, 68, 0.16);
}

.state-wait {
  color: var(--text-dim);
  background: var(--bg-hover);
}

.state-done {
  color: #5ecf99;
  background: rgba(76, 175, 125, 0.16);
}

.state-error {
  color: #ff8080;
  background: rgba(255, 90, 90, 0.16);
}
</style>
