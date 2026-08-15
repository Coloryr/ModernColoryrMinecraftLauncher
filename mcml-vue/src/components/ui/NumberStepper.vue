<script setup lang="ts">
// 数字输入：[- 输入框 +]，可手动输入数字，也可用左右按钮
import { ref } from "vue";

const props = withDefaults(
  defineProps<{
    modelValue: number;
    min?: number;
    max?: number;
    step?: number;
  }>(),
  { min: 0, max: 99999, step: 1 },
);

const emit = defineEmits<{ (e: "update:modelValue", v: number): void }>();

const editing = ref(false);
const draft = ref(String(props.modelValue));

function clamp(v: number) {
  return Math.min(props.max, Math.max(props.min, v));
}

function commit(v: number) {
  emit("update:modelValue", clamp(v));
}

function dec() {
  commit(props.modelValue - props.step);
}

function inc() {
  commit(props.modelValue + props.step);
}

function onFocus() {
  editing.value = true;
  draft.value = String(props.modelValue);
}

function onInput(e: Event) {
  draft.value = (e.target as HTMLInputElement).value;
}

function onCommit() {
  editing.value = false;
  const n = Number(draft.value);
  if (!Number.isNaN(n)) commit(n);
}
</script>

<template>
  <div class="stepper">
    <button class="step-btn" @click="dec">−</button>
    <input
      class="step-input"
      type="number"
      :min="min"
      :max="max"
      :step="step"
      :value="editing ? draft : modelValue"
      @focus="onFocus"
      @input="onInput"
      @change="onCommit"
      @keyup.enter="($event.target as HTMLInputElement).blur()"
      @blur="onCommit"
    />
    <button class="step-btn" @click="inc">＋</button>
  </div>
</template>

<style scoped>
.stepper {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.step-btn {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text);
  font-size: 15px;
  line-height: 1;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  font-family: inherit;
  padding: 0;
  flex-shrink: 0;
}

.step-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

.step-btn:active {
  transform: scale(0.95);
}

.step-input {
  width: 64px;
  height: 28px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text);
  font-size: 13px;
  text-align: center;
  outline: none;
  font-variant-numeric: tabular-nums;
  font-family: inherit;
  -moz-appearance: textfield;
}

.step-input::-webkit-outer-spin-button,
.step-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

.step-input:focus {
  border-color: var(--accent);
}
</style>
