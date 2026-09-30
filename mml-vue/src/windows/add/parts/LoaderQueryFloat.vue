<script setup lang="ts">
// 添加实例 · 窗口正上方的浮动进度提示
// 两种形态：有总数（支持列表按加载器步进）走确定进度条，无总数（拉取加载器版本）走不确定滚动条。
// Teleport 到 body 并 pointer-events:none，不挡窗口操作。
import { computed } from "vue";
import { t } from "../../../lib/i18n";

const props = defineProps<{
  /** 是否显示 */
  visible: boolean;
  /** 任务类型：query = 查询支持的加载器；versions = 拉取加载器版本 */
  kind: "query" | "versions";
  step: number;
  total: number;
}>();

const label = computed(() =>
  props.kind === "query" ? t("add.loaderQuerying") : t("add.loaderVerLoading"),
);
/** 没有步数可报时用不确定动画（进度条宽度也没有意义） */
const determinate = computed(() => props.kind === "query" && props.total > 0);
const percent = computed(() => (props.total ? (props.step / props.total) * 100 : 0));
</script>

<template>
  <Teleport to="body">
    <Transition name="load-pop">
      <div v-if="visible" class="load-float" role="status" aria-live="polite">
        <span class="load-float-spinner"></span>
        <span class="load-float-label">{{ label }}</span>
        <div class="load-float-bar">
          <div
            v-if="determinate"
            class="load-float-fill"
            :style="{ width: percent + '%' }"
          ></div>
          <div v-else class="load-float-indet"></div>
        </div>
        <span v-if="determinate" class="load-float-text">{{ step }} / {{ total }}</span>
      </div>
    </Transition>
  </Teleport>
</template>

<style scoped>
.load-float {
  position: fixed;
  top: 14px;
  left: 0;
  right: 0;
  margin: 0 auto;
  width: fit-content;
  z-index: 500;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  background: var(--bg-card);
  border: 1px solid var(--accent-border);
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  font-size: 12.5px;
  color: var(--text);
  pointer-events: none;
}

.load-float-spinner {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  animation: load-float-spin 0.8s linear infinite;
  flex-shrink: 0;
}

@keyframes load-float-spin {
  to {
    transform: rotate(360deg);
  }
}

.load-float-label {
  color: var(--text-dim);
  white-space: nowrap;
}

.load-float-bar {
  width: 90px;
  height: 6px;
  border-radius: 3px;
  background: var(--border);
  overflow: hidden;
}

.load-float-fill {
  height: 100%;
  border-radius: 3px;
  background: var(--accent);
  transition: width 0.2s;
}

/* 不确定进度：小色块来回滚动 */
.load-float-indet {
  height: 100%;
  width: 40%;
  border-radius: 3px;
  background: var(--accent);
  animation: load-float-slide 1.1s ease-in-out infinite;
}

@keyframes load-float-slide {
  from {
    transform: translateX(-100%);
  }
  to {
    transform: translateX(300%);
  }
}

.load-float-text {
  color: var(--text-dim);
  min-width: 30px;
  text-align: right;
}

.load-pop-enter-active,
.load-pop-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}

.load-pop-enter-from,
.load-pop-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}
</style>
