<script setup lang="ts">
// 统一按钮：primary（主操作）/ accent（强调）/ plain（次要）/ danger（危险）/ ghost（图标类）
withDefaults(
  defineProps<{
    variant?: "primary" | "accent" | "plain" | "danger" | "ghost";
    size?: "sm" | "md" | "lg";
    disabled?: boolean;
    block?: boolean;
  }>(),
  { variant: "plain", size: "md", disabled: false, block: false },
);
</script>

<template>
  <button
    class="ui-btn"
    :class="[`v-${variant}`, `s-${size}`, { block }]"
    :disabled="disabled"
  >
    <slot />
  </button>
</template>

<style scoped>
.ui-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  border: none;
  border-radius: 10px;
  font-family: inherit;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.ui-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
  filter: none;
}

/* 尺寸 */
.s-sm {
  font-size: 12.5px;
  padding: 7px 14px;
  border-radius: 8px;
}

.s-md {
  font-size: 14px;
  padding: 11px 20px;
}

.s-lg {
  font-size: 15px;
  padding: 13px 30px;
}

/* 变体 */
.v-primary {
  background: var(--accent-grad);
  color: #fff;
  box-shadow: 0 4px 14px rgba(79, 140, 255, 0.22);
}

.v-primary:hover {
  filter: brightness(1.1);
}

.v-primary:active {
  transform: scale(0.98);
}

.v-plain {
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-dim);
}

.v-plain:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.v-accent {
  border: 1px solid var(--accent-border);
  background: var(--accent-soft);
  color: var(--accent);
}

.v-accent:hover {
  background: var(--accent);
  color: #fff;
}

.v-danger {
  background: rgba(255, 95, 86, 0.14);
  color: var(--red);
  border: 1px solid rgba(255, 95, 86, 0.4);
}

.v-danger:hover {
  background: rgba(255, 95, 86, 0.24);
}

.v-ghost {
  background: transparent;
  color: var(--text-dim);
  border: 1px solid transparent;
}

.v-ghost:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.block {
  width: 100%;
}
</style>
