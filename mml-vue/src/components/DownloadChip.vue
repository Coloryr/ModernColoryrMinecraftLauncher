<script setup lang="ts">
// 单窗口模式：有下载任务时右下角的浮出入口（点击打开"下载管理"弹窗）
//
// 为什么需要它：下载窗口原本只能从方块列表的渲染流程打开，一旦离开那个页面就再也找不到入口；
// 单窗口模式下下载又是常驻后台的（关掉弹窗任务照跑），所以给一个"有任务才出现"的常驻入口，
// 顺带兼作进度指示（单窗口下用户可能停在任意页面，别处看不到下载进度）。
//
// 多窗口模式不需要它：那时下载管理是独立窗口，任务在跑时窗口自己开着。
import { computed, onMounted, onUnmounted, ref } from "vue";
import { t } from "../lib/i18n";
import { api, onDownloadItem, onDownloadTask } from "../lib/api";
import { openDownloadPopup } from "../windows/windowManager";
import type { DownloadStatusDto } from "../lib/bindings";

const status = ref<DownloadStatusDto>({ tasks: [], threads: [], speed: 0, paused: false });

/** 拉一次任务快照（失败保留上次数据：入口闪一下比不更新更糟） */
async function refresh() {
  try {
    status.value = await api.getDownloadStatus();
  } catch {
    /* 忽略：后端不可用时保留上次显示 */
  }
}

const count = computed(() => status.value.tasks.length);

/** 总体进度：与下载窗口同一口径（优先按字节，元信息未知时退回文件数） */
const percent = computed(() => {
  const list = status.value.tasks;
  const all = list.reduce((n, x) => n + x.allBytes, 0);
  if (all > 0) {
    const now = list.reduce((n, x) => n + x.nowBytes, 0);
    return Math.min(100, (now / all) * 100);
  }
  const total = list.reduce((n, x) => n + x.total, 0);
  const done = list.reduce((n, x) => n + x.completed + x.failed, 0);
  return total > 0 ? Math.min(100, (done / total) * 100) : 0;
});

const label = computed(() => t("winDownload.taskCount", { n: count.value }));

let unsubs: Array<() => void> = [];

onMounted(async () => {
  await refresh();
  // 任务 / 文件状态变化都会广播，跟着刷新即可（进度本身也是事件驱动的）
  unsubs.push(await onDownloadTask(() => void refresh()));
  unsubs.push(await onDownloadItem(() => void refresh()));
});

onUnmounted(() => {
  unsubs.forEach((fn) => fn());
  unsubs = [];
});
</script>

<template>
  <!-- 没有任务时整个入口不存在（任务全部结束后后端会清表，它自己就消失了） -->
  <button
    v-if="count > 0"
    class="dl-chip"
    v-tip="t('features.download')"
    @click="openDownloadPopup()"
  >
    <span class="dl-row">
      <span class="dl-icon">
        <svg
          viewBox="0 0 24 24"
          width="15"
          height="15"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M12 3v12" />
          <path d="m7 12 5 5 5-5" />
          <path d="M4 21h16" />
        </svg>
      </span>
      <span class="dl-text">{{ label }}</span>
      <span class="dl-pct">{{ Math.round(percent) }}%</span>
    </span>
    <span class="dl-track">
      <span class="dl-fill" :style="{ width: percent + '%' }" />
    </span>
  </button>
</template>

<style scoped>
.dl-chip {
  position: fixed;
  right: 18px;
  bottom: 18px;
  /* 在页面内容之上、弹窗遮罩（100）之下：弹窗开着时它被遮住是合理的 */
  z-index: 60;
  display: flex;
  flex-direction: column;
  gap: 7px;
  min-width: 158px;
  padding: 9px 12px 10px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
  color: var(--text);
  font-family: inherit;
  cursor: pointer;
  box-shadow: var(--shadow-md);
  transition: border-color 0.15s ease, transform 0.15s ease;
}

.dl-chip:hover {
  border-color: var(--accent);
  transform: translateY(-2px);
}

.dl-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.dl-icon {
  display: flex;
  align-items: center;
  color: var(--accent);
  flex-shrink: 0;
}

.dl-text {
  flex: 1;
  min-width: 0;
  font-size: 12.5px;
  text-align: left;
  white-space: nowrap;
}

.dl-pct {
  font-size: 12px;
  color: var(--text-dim);
  flex-shrink: 0;
}

.dl-track {
  height: 3px;
  border-radius: 2px;
  background: var(--bg-hover);
  overflow: hidden;
}

.dl-fill {
  display: block;
  height: 100%;
  border-radius: 2px;
  background: var(--accent-grad);
  transition: width 0.2s ease;
}
</style>
