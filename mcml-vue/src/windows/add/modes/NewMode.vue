<script setup lang="ts">
// 添加实例 · 模式一：从头新建（版本类型 / 版本 / 加载器 / 加载器版本 / 自定义加载器路径）
import { computed, ref } from "vue";
import { t } from "../../../lib/i18n";
import BaseButton from "../../../components/ui/BaseButton.vue";
import type { VersionInfo } from "../../../lib/types";

const props = defineProps<{
  versions: VersionInfo[];
  verType: "release" | "beta";
  newVersion: string;
  loader: string;
  loaderVersion: string;
  loaderPath: string;
}>();

const emit = defineEmits<{
  (e: "update:verType", v: "release" | "beta"): void;
  (e: "update:newVersion", v: string): void;
  (e: "update:loader", v: string): void;
  (e: "update:loaderVersion", v: string): void;
  (e: "update:loaderPath", v: string): void;
}>();

const LOADERS = ["原版", "Forge", "Fabric", "Quilt", "NeoForge", "OptiFine", "LiteLoader", "自定义"];
const LOADER_VERSIONS: Record<string, string[]> = {
  Forge: ["47.3.0", "47.2.0", "43.4.0", "36.1.0", "14.23.5.2860"],
  Fabric: ["0.16.9", "0.16.5", "0.15.11", "0.14.25"],
  NeoForge: ["21.1.0", "21.0.167", "20.4.80-beta"],
  Quilt: ["0.27.1", "0.26.0", "0.25.0"],
  OptiFine: ["HD_U_I6", "HD_U_H6", "HD_U_G5"],
  LiteLoader: ["1.12.2-SNAPSHOT"],
};

const loaderVersions = computed(() => LOADER_VERSIONS[props.loader] ?? []);
const loaderPathInput = ref<HTMLInputElement | null>(null);

function onLoaderChange(e: Event) {
  const v = (e.target as HTMLSelectElement).value;
  emit("update:loader", v);
  emit("update:loaderVersion", LOADER_VERSIONS[v]?.[0] ?? "");
}

function onLoaderPathPick(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (file) emit("update:loaderPath", file.name);
}
</script>

<template>
  <div class="add-row2">
    <div class="add-field">
      <label class="field-label">{{ t("add.versionType") }}</label>
      <select
        :value="verType"
        class="field-select"
        @change="emit('update:verType', ($event.target as HTMLSelectElement).value as 'release' | 'beta')"
      >
        <option value="release">{{ t("add.typeRelease") }}</option>
        <option value="beta">{{ t("add.typeBeta") }}</option>
      </select>
    </div>
    <div class="add-field">
      <label class="field-label">{{ t("add.version") }} <span class="req">*</span></label>
      <select
        :value="newVersion"
        class="field-select"
        @change="emit('update:newVersion', ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="v in versions" :key="v.id" :value="v.id">
          {{ v.id }}（{{ v.versionType }}）
        </option>
      </select>
      <div v-if="versions.length === 0" class="empty-tip">{{ t("add.loading") }}</div>
    </div>
  </div>

  <div class="add-row2">
    <div class="add-field">
      <label class="field-label">{{ t("add.loader") }}</label>
      <select :value="loader" class="field-select" @change="onLoaderChange">
        <option v-for="l in LOADERS" :key="l" :value="l">{{ l }}</option>
      </select>
    </div>
    <div v-if="loaderVersions.length" class="add-field">
      <label class="field-label">{{ t("add.loaderVersion") }}</label>
      <select
        :value="loaderVersion"
        class="field-select"
        @change="emit('update:loaderVersion', ($event.target as HTMLSelectElement).value)"
      >
        <option v-for="lv in loaderVersions" :key="lv" :value="lv">{{ lv }}</option>
      </select>
    </div>
  </div>

  <!-- 自定义加载器：提供路径选择 -->
  <template v-if="loader === '自定义'">
    <label class="field-label">{{ t("add.loaderPath") }}</label>
    <div class="path-row">
      <input
        :value="loaderPath"
        class="field-input"
        :placeholder="t('add.loaderPathPlaceholder')"
        spellcheck="false"
        @input="emit('update:loaderPath', ($event.target as HTMLInputElement).value)"
      />
      <BaseButton size="sm" variant="accent" @click="loaderPathInput?.click()">…</BaseButton>
      <input ref="loaderPathInput" type="file" accept=".jar" class="hidden-input" @change="onLoaderPathPick" />
    </div>
  </template>
</template>

<style scoped>
.add-row2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}

.add-field .field-label {
  margin-top: 6px;
}

.req {
  color: var(--red);
}

.path-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.hidden-input {
  display: none;
}
</style>
