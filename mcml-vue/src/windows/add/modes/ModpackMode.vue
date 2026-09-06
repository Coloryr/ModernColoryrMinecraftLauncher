<script setup lang="ts">
// 整合包模式：CurseForge / Modrinth 在线搜索 + 选版本安装
// 实例名取自整合包元数据（安装后端自动处理），分组沿用窗口顶部的分组输入框
import { computed, ref, watch } from "vue";
import SegmentedTabs from "../../../components/ui/SegmentedTabs.vue";
import { api } from "../../../lib/api";
import { t } from "../../../lib/i18n";
import type { ModpackFile, ModpackItem, VersionInfo } from "../../../lib/types";

const props = defineProps<{
  /** 游戏版本列表（主窗口共享的版本数据，用于过滤搜索结果） */
  versions: VersionInfo[];
}>();

const emit = defineEmits<{
  (e: "install", payload: { source: string; projectId: string; fileId: string; fileName: string }): void;
}>();

const PAGE_SIZE = 20;

const source = ref<"curseforge" | "modrinth">("curseforge");
const query = ref("");
const selVersion = ref("");
const sort = ref("popularity");
const page = ref(0);
const total = ref(0);
const items = ref<ModpackItem[]>([]);
const searching = ref(false);
const searchError = ref("");

/** 当前展开版本列表的整合包项目 ID */
const expandedId = ref("");
const files = ref<ModpackFile[]>([]);
const filesLoading = ref(false);

const SORTS = [
  { value: "popularity", labelKey: "modpack.sortPopularity" },
  { value: "downloads", labelKey: "modpack.sortDownloads" },
  { value: "updated", labelKey: "modpack.sortUpdated" },
  { value: "name", labelKey: "modpack.sortName" },
];

const maxPage = computed(() => Math.max(0, Math.ceil(total.value / PAGE_SIZE) - 1));

async function search() {
  searching.value = true;
  searchError.value = "";
  try {
    const res = await api.searchModpacks(
      source.value,
      query.value.trim() || null,
      selVersion.value || null,
      sort.value,
      page.value,
    );
    items.value = res.items;
    total.value = res.total;
  } catch {
    items.value = [];
    total.value = 0;
    searchError.value = t("modpack.searchFail");
  } finally {
    searching.value = false;
  }
}

function submitSearch() {
  page.value = 0;
  void search();
}

// 来源 / 版本过滤 / 排序变化：回到第一页重新搜索
watch([source, selVersion, sort], submitSearch);

function turnPage(delta: number) {
  const next = page.value + delta;
  if (next < 0 || next > maxPage.value) return;
  page.value = next;
  void search();
}

/** 展开某整合包的版本列表（再次点击收起） */
async function toggleFiles(item: ModpackItem) {
  if (expandedId.value === item.id) {
    expandedId.value = "";
    files.value = [];
    return;
  }
  expandedId.value = item.id;
  files.value = [];
  filesLoading.value = true;
  try {
    files.value = await api.getModpackFiles(source.value, item.id, selVersion.value || null);
  } catch {
    files.value = [];
  } finally {
    filesLoading.value = false;
  }
}

function install(item: ModpackItem, file: ModpackFile) {
  emit("install", {
    source: source.value,
    projectId: item.id,
    fileId: file.id,
    fileName: file.name || item.name,
  });
}

function formatDate(date: string): string {
  return date ? date.slice(0, 10) : "";
}

function formatSize(size: number): string {
  if (!size) return t("modpack.unknownSize");
  if (size >= 1024 * 1024) return `${(size / 1024 / 1024).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(size / 1024))} KB`;
}
</script>

<template>
  <div class="modpack-mode">
    <!-- 过滤条件 -->
    <div class="modpack-filters">
      <SegmentedTabs
        v-model="source"
        :options="[
          { value: 'curseforge', label: t('modpack.curseforge') },
          { value: 'modrinth', label: t('modpack.modrinth') },
        ]"
      />
      <select v-model="selVersion" class="field-input sel-version">
        <option value="">{{ t("modpack.allVersions") }}</option>
        <option v-for="v in versions" :key="v.id" :value="v.id">{{ v.id }}</option>
      </select>
      <select v-model="sort" class="field-input sel-sort">
        <option v-for="s in SORTS" :key="s.value" :value="s.value">{{ t(s.labelKey) }}</option>
      </select>
    </div>

    <!-- 搜索 -->
    <div class="modpack-search">
      <input
        v-model="query"
        class="field-input"
        :placeholder="t('modpack.searchHint')"
        spellcheck="false"
        @keydown.enter="submitSearch"
      />
      <button class="search-btn" :disabled="searching" @click="submitSearch">
        {{ t("modpack.search") }}
      </button>
    </div>

    <!-- 结果列表 -->
    <div class="modpack-list">
      <div v-if="searching" class="empty-tip">{{ t("modpack.loading") }}</div>
      <div v-else-if="searchError" class="empty-tip">{{ searchError }}</div>
      <div v-else-if="items.length === 0" class="empty-tip">{{ t("modpack.empty") }}</div>
      <template v-else>
        <div v-for="item in items" :key="item.id" class="pack-item">
          <button class="pack-main" @click="toggleFiles(item)">
            <img v-if="item.icon" class="pack-icon" :src="item.icon" loading="lazy" alt="" />
            <div v-else class="pack-icon pack-icon-fallback">{{ item.name.slice(0, 1).toUpperCase() }}</div>
            <div class="pack-info">
              <div class="pack-name">
                <span>{{ item.name }}</span>
                <span v-if="item.author" class="pack-author">{{ item.author }}</span>
              </div>
              <div class="pack-desc">{{ item.desc }}</div>
              <div class="pack-meta">
                {{ t("modpack.downloads", { n: item.downloads.toLocaleString() }) }}
              </div>
            </div>
          </button>
          <!-- 版本列表（点击展开） -->
          <div v-if="expandedId === item.id" class="pack-files">
            <div v-if="filesLoading" class="empty-tip">{{ t("modpack.loadingVersions") }}</div>
            <div v-else-if="files.length === 0" class="empty-tip">{{ t("modpack.noVersions") }}</div>
            <div v-for="file in files" :key="file.id" class="file-row">
              <div class="file-info">
                <span class="file-name">{{ file.name }}</span>
                <span class="file-meta">{{ file.fileName }} · {{ formatDate(file.date) }} · {{ formatSize(file.size) }}</span>
              </div>
              <button class="install-btn" @click="install(item, file)">
                {{ t("modpack.install") }}
              </button>
            </div>
          </div>
        </div>
      </template>
    </div>

    <!-- 分页 -->
    <div v-if="maxPage > 0" class="modpack-page">
      <button class="page-btn" :disabled="page === 0 || searching" @click="turnPage(-1)">
        {{ t("modpack.prevPage") }}
      </button>
      <span class="page-num">{{ page + 1 }} / {{ maxPage + 1 }}</span>
      <button class="page-btn" :disabled="page >= maxPage || searching" @click="turnPage(1)">
        {{ t("modpack.nextPage") }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.modpack-mode {
  display: flex;
  flex-direction: column;
  gap: 10px;
  height: 100%;
}

.modpack-filters {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.sel-version,
.sel-sort {
  width: auto;
  min-width: 110px;
  padding: 8px 10px;
  font-size: 13px;
}

.modpack-search {
  display: flex;
  gap: 8px;
}

.search-btn {
  padding: 0 18px;
  border: none;
  border-radius: 10px;
  background: var(--accent-grad);
  color: #fff;
  font-size: 13px;
  font-family: inherit;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
}

.search-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.modpack-list {
  flex: 1;
  min-height: 120px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 10px;
}

.empty-tip {
  padding: 18px 12px;
  text-align: center;
  font-size: 12.5px;
  color: var(--text-dim);
}

.pack-item {
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-side);
  overflow: hidden;
}

.pack-main {
  width: 100%;
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 10px 12px;
  border: none;
  background: transparent;
  color: inherit;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
}

.pack-main:hover {
  background: var(--bg-hover);
}

.pack-icon {
  width: 44px;
  height: 44px;
  border-radius: 9px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--bg-card);
}

.pack-icon-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 18px;
  color: var(--accent);
}

.pack-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.pack-name {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text);
}

.pack-author {
  font-size: 11.5px;
  font-weight: 400;
  color: var(--text-dim);
}

.pack-desc {
  font-size: 12px;
  color: var(--text-dim);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.pack-meta {
  font-size: 11.5px;
  color: var(--text-dim);
  opacity: 0.8;
}

.pack-files {
  border-top: 1px solid var(--border);
  display: flex;
  flex-direction: column;
}

.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 14px;
}

.file-row + .file-row {
  border-top: 1px solid var(--border);
}

.file-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.file-name {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-meta {
  font-size: 11px;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.install-btn {
  padding: 6px 14px;
  border: 1px solid var(--accent-border);
  border-radius: 7px;
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 12px;
  font-family: inherit;
  font-weight: 600;
  cursor: pointer;
  flex-shrink: 0;
}

.install-btn:hover {
  background: var(--accent);
  color: #fff;
}

.modpack-page {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
}

.page-btn {
  padding: 6px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
}

.page-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: var(--accent);
}

.page-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.page-num {
  font-size: 12.5px;
  color: var(--text-dim);
  min-width: 60px;
  text-align: center;
}
</style>
