<script setup lang="ts">
// 实例元信息面板：版本类型+版本 / 加载器+加载器版本 / 整合包平台 / 游戏内语言+日志编码
import { computed, ref } from "vue";
import { t } from "../lib/i18n";
import type { InstanceInfo, VersionInfo } from "../lib/types";

const props = defineProps<{
  instance: InstanceInfo;
  versions: VersionInfo[];
  loaders: string[];
}>();

const emit = defineEmits<{
  (e: "update", patch: Partial<InstanceInfo>): void;
}>();

function change(patch: Partial<InstanceInfo>) {
  emit("update", patch);
}

function loaderLabel(name: string): string {
  if (name === "原版") return t("loader.normal");
  if (name === "自定义") return t("loader.custom");
  return name;
}

// 版本类型 + 版本（同一行）
const versionTypes = computed(() => {
  const set = new Set<string>();
  for (const v of props.versions) set.add(v.versionType);
  return [...set];
});

const typeFilter = ref(props.instance.versionType ?? "release");

const filteredVersions = computed(() =>
  props.versions.filter((v) => v.versionType === typeFilter.value),
);

function onVersionTypeChange(value: string) {
  typeFilter.value = value;
  // 当前版本不在该类型中时，切到该类型第一个版本
  const target = props.versions.find((v) => v.versionType === value);
  if (target && target.id !== props.instance.version) {
    change({ version: target.id, versionType: value });
  } else {
    change({ versionType: value });
  }
}

// 加载器版本（按加载器提供下拉选项）
const LOADER_VERSIONS: Record<string, string[]> = {
  原版: [],
  Forge: ["47.3.0", "47.2.0", "47.1.0", "43.4.0"],
  NeoForge: ["21.1.0", "21.0.0", "20.4.80"],
  Fabric: ["0.16.9", "0.15.11", "0.14.24"],
  Quilt: ["0.27.1", "0.26.0", "0.25.0"],
  OptiFine: ["HD_U_I6", "HD_U_H6", "HD_U_G5"],
  LiteLoader: ["1.12.2-SNAPSHOT", "1.8.9-SNAPSHOT"],
  自定义: ["custom-1.0"],
};

const loaderVersions = computed(() => LOADER_VERSIONS[props.instance.loader] ?? []);

function onLoaderChange(value: string) {
  const list = LOADER_VERSIONS[value] ?? [];
  const first = list[0] ?? null;
  change({ loader: value, loaderVersion: first });
}

// 整合包平台
const platforms = [
  { value: "", label: t("meta.platformNone") },
  { value: "CurseForge", label: "CurseForge" },
  { value: "Modrinth", label: "Modrinth" },
  { value: "McMod", label: "McMod" },
];

function onPlatformChange(value: string) {
  // 选择后直接应用；选“无”时清空 ID
  change({
    modpackType: value || undefined,
    pid: value ? props.instance.pid : undefined,
    fid: value ? props.instance.fid : undefined,
  });
}

// 语言 + 日志编码（同一行）
const langs = [
  { value: "zh_cn", label: "简体中文" },
  { value: "zh_tw", label: "繁體中文" },
  { value: "en_us", label: "English" },
  { value: "ja_jp", label: "日本語" },
  { value: "de_de", label: "Deutsch" },
  { value: "fr_fr", label: "Français" },
];
</script>

<template>
  <div class="meta-panel">
    <!-- 版本类型 + 版本 -->
    <span class="meta-label">{{ t("meta.versionType") }}</span>
    <select
      class="field-select"
      :value="typeFilter"
      @change="onVersionTypeChange(($event.target as HTMLSelectElement).value)"
    >
      <option v-for="vt in versionTypes" :key="vt" :value="vt">
        {{ vt === "release" ? t("version.release") : vt === "snapshot" ? t("version.snapshot") : t("version.other") }}
      </option>
    </select>
    <span class="meta-label small">{{ t("meta.version") }}</span>
    <select
      class="field-select"
      :value="instance.version"
      @change="change({ version: ($event.target as HTMLSelectElement).value })"
    >
      <option v-for="v in filteredVersions" :key="v.id" :value="v.id">{{ v.id }}</option>
    </select>

    <!-- 加载器 + 加载器版本 -->
    <span class="meta-label">{{ t("meta.loader") }}</span>
    <select
      class="field-select"
      :value="instance.loader"
      @change="onLoaderChange(($event.target as HTMLSelectElement).value)"
    >
      <option v-for="l in loaders" :key="l" :value="l">{{ loaderLabel(l) }}</option>
    </select>
    <span class="meta-label small">{{ t("meta.loaderVersion") }}</span>
    <select
      class="field-select"
      :value="instance.loaderVersion ?? ''"
      @change="change({ loaderVersion: ($event.target as HTMLSelectElement).value || null })"
    >
      <option v-for="lv in loaderVersions" :key="lv" :value="lv">{{ lv }}</option>
    </select>

    <!-- 整合包平台（选中后下方输入 ID） -->
    <span class="meta-label">{{ t("meta.modpack") }}</span>
    <select
      class="field-select modpack-select"
      :value="instance.modpackType ?? ''"
      @change="onPlatformChange(($event.target as HTMLSelectElement).value)"
    >
      <option v-for="p in platforms" :key="p.value" :value="p.value">{{ p.label }}</option>
    </select>

    <!-- 整合包 ID（选择平台后显示） -->
    <template v-if="instance.modpackType">
      <span class="meta-label">{{ t("meta.pid") }}</span>
      <input
        class="field-input"
        :value="instance.pid ?? ''"
        placeholder="000000"
        spellcheck="false"
        @input="change({ pid: ($event.target as HTMLInputElement).value })"
      />
      <span class="meta-label small">{{ t("meta.fid") }}</span>
      <input
        class="field-input"
        :value="instance.fid ?? ''"
        placeholder="000000"
        spellcheck="false"
        @input="change({ fid: ($event.target as HTMLInputElement).value })"
      />
    </template>

    <!-- 游戏内语言 + 日志编码（同一行） -->
    <span class="meta-label">{{ t("meta.lang") }}</span>
    <select
      class="field-select"
      :value="instance.lang ?? 'zh_cn'"
      @change="change({ lang: ($event.target as HTMLSelectElement).value })"
    >
      <option v-for="l in langs" :key="l.value" :value="l.value">{{ l.label }}</option>
    </select>
    <span class="meta-label small">{{ t("meta.logEncoding") }}</span>
    <select
      class="field-select"
      :value="instance.logEncoding ?? 'utf8'"
      @change="change({ logEncoding: ($event.target as HTMLSelectElement).value })"
    >
      <option value="utf8">{{ t("meta.utf8") }}</option>
      <option value="gbk">{{ t("meta.gbk") }}</option>
    </select>
  </div>
</template>

<style scoped>
/* 4 列网格：标签 | 下拉 | 标签 | 下拉，保证 6 个下拉框左对齐 */
.meta-panel {
  display: grid;
  grid-template-columns: 84px 1fr 64px 1fr;
  gap: 10px;
  align-items: center;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 14px 16px;
}

.meta-label {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-dim);
  white-space: nowrap;
}

.meta-label.small {
  text-align: right;
}

.modpack-select {
  grid-column: 2 / 5;
}

.modpack-id {
  font-size: 12px;
  color: var(--text-dim);
  text-align: left;
}
</style>
