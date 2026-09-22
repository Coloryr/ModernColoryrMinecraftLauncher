<script setup lang="ts">
// 整合包安装进度条（多任务）：总进度条 + 任务数，点击展开每个任务的
// 详情（阶段 / 主子进度 / 取消按钮），终态任务带状态标签，可一键清除。
// 下载整合包窗口常显；主窗口仅在整合包窗口关闭时显示（由调用方 v-if 控制）。
import { computed, ref } from "vue";
import CollapsePanel from "./ui/CollapsePanel.vue";
import { api } from "../lib/api";
import { t, tErr } from "../lib/i18n";
import { showToast } from "../lib/toast";
import type { ModPackStatusDto, ModPackTaskDto } from "../lib/bindings";

const props = defineProps<{
  status: ModPackStatusDto;
}>();

const open = ref(false);

/** 进行中的任务（终态之外） */
const running = computed(() =>
  props.status.tasks.filter((task) => isRunning(task)),
);

/** 已结束的任务（完成 / 失败 / 取消） */
const finished = computed(() =>
  props.status.tasks.filter((task) => !isRunning(task)),
);

/** 总进度 = 各任务主进度的均值 */
const percent = computed(() => {
  if (!running.value.length) {
    return 100;
  }
  const sum = running.value.reduce(
    (acc, task) => acc + (task.total ? (task.now / task.total) * 100 : 0),
    0,
  );
  return sum / running.value.length;
});

function isRunning(task: ModPackTaskDto): boolean {
  return !task.done && !task.failed && !task.cancelled;
}

async function cancel(task: ModPackTaskDto) {
  try {
    await api.cancelModpackInstall(task.pid, task.fid);
  } catch (e) {
    showToast(t("modpack.bar.cancelFail", { msg: tErr(e) }));
  }
}

async function clearDone() {
  try {
    await api.clearModpackDone();
  } catch (e) {
    showToast(tErr(e));
  }
}
</script>

<template>
  <div class="mp-bar">
    <!-- 总览：标题（任务数）+ 清除 + 展开开关，点击整行切换 -->
    <div class="mp-bar-head" @click="open = !open">
      <span class="mp-bar-title">{{ t("modpack.bar.running", { count: running.length }) }}</span>
      <button
        v-if="finished.length"
        class="mp-bar-clear"
        @click.stop="clearDone"
      >
        {{ t("modpack.bar.clearDone") }}
      </button>
      <span class="mp-bar-chevron" :class="{ up: open }">▾</span>
    </div>
    <div class="progress-track">
      <div class="progress-fill" :style="{ width: percent + '%' }" />
    </div>

    <!-- 展开的任务详情 -->
    <CollapsePanel :open="open">
      <div class="mp-task-list">
        <div v-for="task in status.tasks" :key="task.uuid" class="mp-task">
          <div class="mp-task-head">
            <span class="mp-task-name">{{ task.name }}</span>
            <span v-if="task.done" class="mp-tag done">{{ t("add.packState.done") }}</span>
            <span
              v-else-if="task.failed"
              class="mp-tag failed"
              :title="task.error || ''"
            >{{ t("modpack.bar.failed") }}</span>
            <span v-else-if="task.cancelled" class="mp-tag cancelled">{{ t("modpack.bar.cancelled") }}</span>
            <button v-else class="mp-cancel" @click="cancel(task)">{{ t("modpack.bar.cancel") }}</button>
          </div>
          <div v-if="isRunning(task)" class="mp-task-state">
            {{ t(`add.packState.${task.state}`) }}
            <span v-if="task.total">{{ task.now }} / {{ task.total }}</span>
          </div>
          <div v-else-if="task.failed && task.error" class="mp-task-error">
            {{ tErr(task.error) }}
          </div>
          <div class="progress-track sub">
            <div
              class="progress-fill"
              :class="{ dim: !isRunning(task) && !task.done }"
              :style="{ width: task.total ? (task.now / task.total) * 100 + '%' : isRunning(task) ? '0%' : '100%' }"
            />
          </div>
          <template v-if="isRunning(task) && (task.subText || task.subTotal)">
            <div class="mp-task-sub">{{ task.subText || "" }}</div>
            <div class="progress-track sub">
              <div
                class="progress-fill"
                :style="{ width: task.subTotal ? (task.subNow / task.subTotal) * 100 + '%' : '0%' }"
              />
            </div>
          </template>
        </div>
      </div>
    </CollapsePanel>
  </div>
</template>

<style scoped>
.mp-bar {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 12px;
  border-radius: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
}

.mp-bar-head {
  display: flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
  user-select: none;
}

.mp-bar-title {
  flex: 1;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
}

.mp-bar-clear {
  font-size: 12px;
  color: var(--text-dim);
  background: none;
  border: none;
  cursor: pointer;
  padding: 2px 6px;
  border-radius: 6px;
}

.mp-bar-clear:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.mp-bar-chevron {
  font-size: 12px;
  color: var(--text-dim);
  transition: transform 0.22s ease;
}

.mp-bar-chevron.up {
  transform: rotate(180deg);
}

.mp-task-list {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-top: 4px;
}

.mp-task {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.mp-task-head {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mp-task-name {
  flex: 1;
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  word-break: break-all;
}

.mp-task-state {
  font-size: 12px;
  color: var(--text-dim);
}

.mp-task-state span {
  margin-left: 6px;
}

.mp-task-sub {
  font-size: 12px;
  color: var(--text-dim);
  word-break: break-all;
}

.mp-task-error {
  font-size: 12px;
  color: var(--red);
  word-break: break-all;
}

.mp-tag {
  font-size: 11.5px;
  padding: 1px 8px;
  border-radius: 999px;
  flex-shrink: 0;
}

.mp-tag.done {
  color: var(--green);
  background: color-mix(in srgb, var(--green) 14%, transparent);
}

.mp-tag.failed {
  color: var(--red);
  background: color-mix(in srgb, var(--red) 14%, transparent);
}

.mp-tag.cancelled {
  color: var(--text-dim);
  background: var(--bg-hover);
}

.mp-cancel {
  font-size: 12px;
  color: var(--red);
  background: none;
  border: 1px solid color-mix(in srgb, var(--red) 40%, transparent);
  border-radius: 6px;
  padding: 1px 8px;
  cursor: pointer;
  flex-shrink: 0;
}

.mp-cancel:hover {
  background: color-mix(in srgb, var(--red) 12%, transparent);
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

.progress-fill.dim {
  background: var(--text-dim);
  opacity: 0.5;
}

.progress-track.sub {
  height: 6px;
}
</style>
