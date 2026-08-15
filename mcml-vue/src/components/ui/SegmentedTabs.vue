<script setup lang="ts">
// 统一分段切换控件（v-model: string），选项可带图标
import { computed } from "vue";

interface SegOption {
  value: string;
  label: string;
  icon?: string;
}

const props = defineProps<{
  options: SegOption[];
  modelValue: string;
}>();

const emit = defineEmits<{ (e: "update:modelValue", value: string): void }>();

const current = computed(() => props.modelValue);

// 内置图标（线条风格）
const ICONS: Record<string, string> = {
  folder:
    '<path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z"/>',
  grid: '<rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/>',
  list: '<path d="M8 6h13M8 12h13M8 18h13M3 6h.01M3 12h.01M3 18h.01"/>',
};
</script>

<template>
  <div class="seg-tabs">
    <button
      v-for="opt in options"
      :key="opt.value"
      class="seg-btn"
      :class="{ active: current === opt.value }"
      :title="opt.label"
      @click="emit('update:modelValue', opt.value)"
    >
      <svg
        v-if="opt.icon && ICONS[opt.icon]"
        class="seg-icon"
        viewBox="0 0 24 24"
        width="14"
        height="14"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
        stroke-linecap="round"
        v-html="ICONS[opt.icon]"
      />
      <span>{{ opt.label }}</span>
    </button>
  </div>
</template>

<style scoped>
.seg-tabs {
  display: flex;
  gap: 4px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 3px;
}

.seg-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-size: 12.5px;
  padding: 6px 12px;
  border-radius: 7px;
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
  white-space: nowrap;
}

.seg-btn:hover {
  color: var(--text);
}

.seg-btn.active {
  background: var(--accent);
  color: #fff;
  font-weight: 600;
}

.seg-icon {
  flex-shrink: 0;
}
</style>
