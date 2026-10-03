<script setup lang="ts">
// 子窗口通用框架：标题（兼作自绘标题栏）+ 内容区
// 头部整条可拖动，两端按平台样式放窗口按钮；「返回主页面」按钮只在单窗口模式下显示。
import { computed } from "vue";
import { t } from "../../lib/i18n";
import { multiWindow } from "../../windows/windowManager";
import WindowControls from "./WindowControls.vue";
import ModpackTitleIndicator from "../ModpackTitleIndicator.vue";
import DownloadTitleIndicator from "../DownloadTitleIndicator.vue";
import { onTitleBarPointerDown, titleBarStyle, useWindowTitle } from "../../lib/titlebar";

const props = defineProps<{
  title: string;
  /** 内容区不整页滚动（overflow hidden + flex 列），滚动交给视图内部的容器，
   *  详情表格这类"工具栏固定、表格自己滚"的布局用 */
  bodyFill?: boolean;
  /** 窗口内子页面的返回键文案（如"返回"）；不传则用单窗口模式那枚「返回上一页」。
   *  按钮本身只画箭头，这段文案作为悬停提示（v-tip）与无障碍名称存在；
   *  两种用法共用同一个 `.back-btn`，外观完全一致 */
  back?: string;
  /** 是否在标题栏显示整合包安装进度指示（默认显示）。
   *  整合包窗口整页就是进度，给它关掉，免得同一件事在标题栏里再出现一次 */
  hideModpackIndicator?: boolean;
  /** 是否显示下载进度指示（默认显示）。下载窗口整页就是下载管理，同样关掉 */
  hideDownloadIndicator?: boolean;
}>();

const emit = defineEmits<{ (e: "close"): void; (e: "back"): void }>();

/** 单窗口模式（浏览器 / Tauri 都一样）才显示返回按钮：
 *  这时功能页都在同一个窗口里切换，"返回"＝回上一层（见 windowManager 的返回栈）；
 *  多窗口模式下每个功能是独立窗口，关掉它自然露出下面那个，不需要返回键 */
const showBack = computed(() => !multiWindow.value);

// 原生窗口标题（任务栏 / Alt+Tab）跟随自绘标题栏文案。
// 必须走 useWindowTitle：它**每次激活都重设**，而不是只在挂载时设一次 ——
// 单窗口模式下本页被 KeepAlive 缓存，切走再回来不重新挂载，只设一次的话
// 从别的页面切回来时，任务栏上还挂着上一个页面的标题
useWindowTitle(() => props.title);
</script>

<template>
  <div class="window-frame">
    <header class="frame-head" :class="titleBarStyle" @pointerdown="onTitleBarPointerDown">
      <!-- macos 样式：红黄绿在左端 -->
      <WindowControls v-if="titleBarStyle === 'macos'" :style="titleBarStyle" />

      <!-- 窗口内子页面的返回键（如整合包详情）：与下面的单窗口返回按钮共用一套外观。
           按钮只画箭头，文案改由悬停提示给出（见 .back-btn 的说明） -->
      <button
        v-if="props.back"
        class="back-btn"
        v-tip="props.back"
        :aria-label="props.back"
        @click="emit('back')"
      >
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
          <path d="m15 18-6-6 6-6" />
        </svg>
      </button>
      <!-- 单窗口模式才显示返回按钮；子页面自带返回键时让位 -->
      <button
        v-else-if="showBack"
        class="back-btn"
        v-tip="t('winCommon.back')"
        :aria-label="t('winCommon.back')"
        @click="emit('close')"
      >
        <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
          <path d="m15 18-6-6 6-6" />
        </svg>
      </button>
      <h1>{{ title }}</h1>
      <span class="spacer"></span>
      <!-- 标题栏右侧扩展区（如查询进度指示） -->
      <slot name="head-right" />
      <!-- 进度指示器：跟着**当时显示的那条标题栏**走。
           多窗口模式下各功能是独立窗口，自然挂各自的；单窗口模式下功能页也各有自己的
           标题栏（主页面才是 MainTopbar），所以这里照样得挂 —— 之前只在 multiWindow 时挂，
           结果从"下载整合包"返回"添加实例"后两个指示器都不见了。
           没有任务时各自整块不存在 -->
      <ModpackTitleIndicator v-if="!hideModpackIndicator" />
      <DownloadTitleIndicator v-if="!hideDownloadIndicator" />

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

/* 幽灵图标按钮：只有箭头，无边框、无底色，悬停才浮出一层淡底并提亮箭头——
   与右侧窗口按钮"平时透明、悬停才出底"同一套逻辑。
   文案（"返回"）不再画在按钮上，改由模板里的 v-tip 悬停提示给出。
   两个用法（单窗口的返回 / 窗口内子页面的返回）共用这一套外观 */
.back-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  padding: 0;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  transition: background 0.15s, color 0.15s;
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
