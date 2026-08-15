<script setup lang="ts">
// 子窗口通用框架：返回按钮 + 标题 + 内容区
defineProps<{ title: string }>();

const emit = defineEmits<{ (e: "close"): void }>();
</script>

<template>
  <div class="window-frame">
    <header class="frame-head">
      <button class="back-btn" @click="emit('close')">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
          <path d="m15 18-6-6 6-6" />
        </svg>
        返回主界面
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
