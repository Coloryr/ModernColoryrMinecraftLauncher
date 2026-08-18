<script setup lang="ts">
// 子窗口通用框架：标题 + 内容区
// “返回主页面”按钮只在单窗口模式（应用内切换）显示；
// 多窗口模式下子窗口是独立真实窗口/标签页，用系统原生按钮关闭。
import { computed } from "vue";
import { t } from "../../lib/i18n";
import { isTauri, multiWindow } from "../../windows/windowManager";

defineProps<{ title: string }>();

const emit = defineEmits<{ (e: "close"): void }>();

/** 单窗口模式（仅浏览器存在）才显示返回按钮 */
const showBack = computed(() => !isTauri() && !multiWindow.value);
</script>

<template>
  <div class="window-frame">
    <header class="frame-head">
      <button v-if="showBack" class="back-btn" @click="emit('close')">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <path d="m15 18-6-6 6-6" />
        </svg>
        {{ t("winCommon.back") }}
      </button>
      <h1>{{ title }}</h1>
      <span class="spacer"></span>
    </header>
    <div class="frame-body">
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
  height: 60px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 0 18px;
  background: var(--bg-side);
  border-bottom: 1px solid var(--border);
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
</style>
