<script setup lang="ts">
// 子窗口通用框架：标题（兼作自绘标题栏）+ 内容区
// 头部整条可拖动，两端按平台样式放窗口按钮；「返回主页面」按钮只在单窗口模式下显示。
import { computed, watch } from "vue";
import { t } from "../../lib/i18n";
import { isTauri, multiWindow } from "../../windows/windowManager";
import { commands } from "../../lib/bindings";
import WindowControls from "./WindowControls.vue";
import { onTitleBarPointerDown, titleBarStyle } from "../../lib/titlebar";

const props = defineProps<{
  title: string;
  /** 内容区不整页滚动（overflow hidden + flex 列），滚动交给视图内部的容器，
   *  详情表格这类"工具栏固定、表格自己滚"的布局用 */
  bodyFill?: boolean;
}>();

const emit = defineEmits<{ (e: "close"): void }>();

/** 单窗口模式（仅浏览器存在）才显示返回按钮 */
const showBack = computed(() => !isTauri() && !multiWindow.value);

// 原生窗口标题（任务栏 / Alt+Tab）跟随自绘标题栏文案，语言切换时同步更新
watch(
  () => props.title,
  (title) => {
    if (!isTauri()) return;
    commands.windows.setTitle(title).catch(() => {});
  },
  { immediate: true },
);
</script>

<template>
  <div class="window-frame">
    <header class="frame-head" :class="titleBarStyle" @pointerdown="onTitleBarPointerDown">
      <!-- macos 样式：红黄绿在左端 -->
      <WindowControls v-if="titleBarStyle === 'macos'" :style="titleBarStyle" />

      <button v-if="showBack" class="back-btn" @click="emit('close')">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <path d="m15 18-6-6 6-6" />
        </svg>
        {{ t("winCommon.back") }}
      </button>
      <h1>{{ title }}</h1>
      <span class="spacer"></span>
      <!-- 标题栏右侧扩展区（如查询进度指示） -->
      <slot name="head-right" />

      <!-- windows 样式：最小化 / 最大化 / 关闭在右端 -->
      <WindowControls v-if="titleBarStyle === 'windows'" :style="titleBarStyle" />
    </header>
    <div class="frame-body" :class="{ fill: props.bodyFill }">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.window-frame {
  display: flex;
  flex-direction: column;
  height: 100vh;
  background: var(--bg);
}

.frame-head {
  height: var(--titlebar-h);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 0 18px;
  background: var(--bg-side);
  border-bottom: 1px solid var(--border);
}

/* windows 样式的窗口按钮是小方块、不贴边，右侧留一点呼吸空间；
   macos 样式的圆点留在左内边距之后 */
.frame-head.windows {
  padding-right: 10px;
}

.back-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 8px 14px;
  border-radius: 9px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
}

.back-btn:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.frame-head h1 {
  font-size: 16px;
  font-weight: 700;
}

.spacer {
  flex: 1;
}

.frame-body {
  flex: 1;
  overflow-y: auto;
  padding: 22px 26px;
}

.frame-body.fill {
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
</style>
