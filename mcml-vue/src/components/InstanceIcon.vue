<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    name: string;
    uuid: string;
    size?: number;
  }>(),
  { size: 48 },
);

// 根据 uuid 稳定取色，保证同一实例图标一致
const gradients = [
  "linear-gradient(135deg, #3f8cff, #5f6cff)",
  "linear-gradient(135deg, #34d399, #22d3ee)",
  "linear-gradient(135deg, #f59e0b, #ef4444)",
  "linear-gradient(135deg, #a855f7, #ec4899)",
  "linear-gradient(135deg, #14b8a6, #3b82f6)",
  "linear-gradient(135deg, #f97316, #f43f5e)",
  "linear-gradient(135deg, #84cc16, #22c55e)",
  "linear-gradient(135deg, #06b6d4, #6366f1)",
];

const palette = computed(() => {
  let h = 0;
  for (const c of props.uuid) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return gradients[h % gradients.length];
});

const char = computed(() => props.name.trim().charAt(0).toUpperCase() || "M");
</script>

<template>
  <div
    class="inst-icon"
    :style="{
      width: size + 'px',
      height: size + 'px',
      background: palette,
      fontSize: Math.round(size * 0.42) + 'px',
      borderRadius: Math.round(size * 0.24) + 'px',
    }"
  >
    <span>{{ char }}</span>
  </div>
</template>

<style scoped>
.inst-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: 700;
  flex-shrink: 0;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
  border: 1px solid rgba(255, 255, 255, 0.14);
  user-select: none;
  overflow: hidden;
  position: relative;
}

/* 简单的高光装饰 */
.inst-icon::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.22), rgba(255, 255, 255, 0) 45%);
}
</style>
