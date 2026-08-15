<script setup lang="ts">
// 实例下拉选择（带实例图标），用于列表模式选中实例
import { computed, ref } from "vue";
import { t } from "../lib/i18n";
import type { InstanceInfo } from "../lib/types";
import InstanceIcon from "./InstanceIcon.vue";

const props = defineProps<{
  instances: InstanceInfo[];
  modelValue: string | null;
}>();

const emit = defineEmits<{ (e: "update:modelValue", uuid: string): void }>();

const open = ref(false);

const selected = computed(() =>
  props.instances.find((i) => i.uuid === props.modelValue) ?? null,
);

function pick(inst: InstanceInfo) {
  emit("update:modelValue", inst.uuid);
  open.value = false;
}
</script>

<template>
  <div class="instance-select">
    <button class="select-btn" @click="open = !open">
      <InstanceIcon
        v-if="selected"
        :name="selected.name"
        :uuid="selected.uuid"
        :size="30"
      />
      <span class="placeholder" v-else>—</span>
      <span class="select-text">
        <span v-if="selected" class="sel-name">{{ selected.name }}</span>
        <span v-else class="sel-placeholder">{{ t("launch.selectInstance") }}</span>
        <span v-if="selected" class="sel-sub">
          {{ selected.version }}
          <template v-if="selected.loader !== '原版'">
            <span class="loader-text">{{ selected.loader }}</span>
          </template>
        </span>
      </span>
      <svg
        class="chevron"
        :class="{ flip: open }"
        viewBox="0 0 24 24"
        width="14"
        height="14"
        fill="none"
        stroke="currentColor"
        stroke-width="2"
      >
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>

    <div v-if="open" class="select-menu">
      <button
        v-for="inst in instances"
        :key="inst.uuid"
        class="option"
        :class="{ active: inst.uuid === modelValue }"
        @click="pick(inst)"
      >
        <InstanceIcon :name="inst.name" :uuid="inst.uuid" :size="30" />
        <span class="option-text">
          <span class="option-name">{{ inst.name }}</span>
          <span class="option-sub">
            {{ inst.version }}
            <template v-if="inst.loader !== '原版'">
              <span class="loader-text">{{ inst.loader }}</span>
            </template>
          </span>
        </span>
        <span v-if="inst.running" class="run-dot" :title="t('launch.running')"></span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.instance-select {
  position: relative;
  width: 100%;
  max-width: 460px;
}

.select-btn {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text);
  cursor: pointer;
  transition: all 0.15s;
  font-family: inherit;
}

.select-btn:hover {
  border-color: var(--accent);
  background: var(--bg-hover);
}

.placeholder {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  background: var(--bg-hover);
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-dim);
  flex-shrink: 0;
}

.select-text {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  min-width: 0;
  line-height: 1.3;
  text-align: left;
}

.sel-name {
  font-size: 14px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  max-width: 100%;
}

.sel-placeholder {
  font-size: 13.5px;
  color: var(--text-dim);
}

.sel-sub {
  font-size: 11.5px;
  color: var(--text-dim);
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.loader-text {
  font-size: 10px;
  padding: 0 6px;
  border-radius: 8px;
  background: var(--accent-soft);
  color: var(--accent);
  line-height: 1.6;
}

.chevron {
  color: var(--text-dim);
  transition: transform 0.15s;
  flex-shrink: 0;
}

.chevron.flip {
  transform: rotate(180deg);
}

.select-menu {
  position: absolute;
  left: 0;
  right: 0;
  top: calc(100% + 6px);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: var(--shadow-lg);
  padding: 6px;
  max-height: 280px;
  overflow-y: auto;
  z-index: 300;
}

.option {
  width: 100%;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border: none;
  border-radius: 9px;
  background: transparent;
  color: var(--text);
  cursor: pointer;
  font-family: inherit;
  text-align: left;
}

.option:hover {
  background: var(--bg-hover);
}

.option.active {
  background: var(--accent-soft);
  outline: 1px solid var(--accent-border);
}

.option-text {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-width: 0;
  line-height: 1.3;
}

.option-name {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.option-sub {
  font-size: 11px;
  color: var(--text-dim);
  display: inline-flex;
  align-items: center;
  gap: 4px;
}

.option-sub .loader-text {
  font-size: 9.5px;
}

.run-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--green);
  box-shadow: 0 0 5px var(--green);
  flex-shrink: 0;
}
</style>
