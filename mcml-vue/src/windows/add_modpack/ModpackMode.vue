<script setup lang="ts">
// 整合包模式：CurseForge / Modrinth 在线搜索 + 选版本安装
// 实例名取自整合包元数据（安装后端自动处理），分组沿用窗口顶部的分组输入框
import { computed, onMounted, ref, watch } from "vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import { api } from "../../lib/api";
import { t, tErr } from "../../lib/i18n";
import type { FileListItemDto, ProjectItemDto } from "../../lib/bindings";

const props = defineProps<{
  /** 安装到的分组（空 = 默认分组） */
  group: string;
  /** 已有分组候选（datalist 下拉用） */
  groups: string[];
}>();

const emit = defineEmits<{
  (e: "install", payload: { source: string; projectId: string; fileId: string; fileName: string }): void;
  (e: "update:group", value: string): void;
}>();

/** 后端列表每页 20 个项目 */
const PAGE_SIZE = 20;

const SOURCE_LABELS: Record<string, string> = {
  curseforge: "modpack.curseforge",
  modrinth: "modpack.modrinth",
};

/** 下载源（取值是后端线串，显示的标签走 i18n） */
const sources = ref<Array<{ value: string; label: string }>>([]);
const source = ref("");

/** 排序方式（取值就是后端枚举线串，原样回传） */
const sorts = ref<string[]>([]);
const sort = ref("");

/** 游戏版本，"" = 全部 */
const versions = ref<string[]>([]);
const version = ref("");

/** 分类，"" = 全部。键 = 传给后端的分类值，值 = 显示名 */
const categories = ref<Array<{ value: string; label: string }>>([]);
const category = ref("");

/** 搜索文本 */
const filter = ref("");

const items = ref<ProjectItemDto[]>([]);
const total = ref(0);
const page = ref(0);
const searching = ref(false);
const error = ref("");

/** 当前展开版本列表的项目 ID */
const expandedId = ref("");
const files = ref<FileListItemDto[]>([]);
const filesLoading = ref(false);

const maxPage = computed(() => Math.max(0, Math.ceil(total.value / PAGE_SIZE) - 1));

/** 拉取下载源列表并选中第一个；后续筛选数据由 watch(source) 加载 */
async function loadSources() {
  try {
    sources.value = (await api.getModpackSources()).map((value) => ({
      value,
      label: t(SOURCE_LABELS[value] ?? value),
    }));
  } catch (e) {
    error.value = tErr(e);
    return;
  }
  if (sources.value.length === 0) {
    error.value = t("modpack.sourceFail");
    return;
  }
  source.value = sources.value[0].value;
}

/** 切换下载源：清空结果，重新拉排序 / 版本 / 分类，然后搜第一页 */
async function loadSource() {
  items.value = [];
  total.value = 0;
  page.value = 0;
  error.value = "";
  expandedId.value = "";
  files.value = [];
  version.value = "";
  category.value = "";
  sort.value = "";
  sorts.value = [];
  versions.value = [];
  categories.value = [];

  searching.value = true;
  try {
    const [sortList, versionList, categoryMap] = await Promise.all([
      api.getModpackSorts(source.value),
      api.getModpackVersions(source.value),
      api.getModpackCategories(source.value),
    ]);

    sorts.value = sortList;
    sort.value = sortList[0] ?? "";
    // 版本列表里后端已经插了一个空串表示“全部”，这里统一由前端的选项提供
    versions.value = versionList.filter((item) => item !== "");
    categories.value = Object.entries(categoryMap)
      .map(([value, label]) => ({ value, label }))
      .sort((a, b) => a.label.localeCompare(b.label));
  } catch (e) {
    error.value = tErr(e);
    searching.value = false;
    return;
  }

  searching.value = false;

  await search();
}

/** 搜索代次：新的一发开始后，旧一发的返回（含被后端取消的）全部丢弃 */
let searchSeq = 0;

/** 搜索期间置 `searching`（弹窗锁住窗口），成败都清掉 */
async function search() {
  const mine = ++searchSeq;
  searching.value = true;
  try {
    const res = await api.searchModpacks(
      source.value,
      page.value,
      sort.value,
      category.value || null,
      filter.value.trim() || null,
      version.value || null,
    );
    if (mine !== searchSeq) return;
    items.value = res.items;
    total.value = res.count;
    error.value = "";
  } catch (e) {
    // 被更新的一发顶掉、或窗口关闭时后端取消，都不算失败，不弹错
    if (mine !== searchSeq || String(e) === "err.cancelled") return;
    items.value = [];
    total.value = 0;
    error.value = tErr(e);
  } finally {
    // 只有最新一发负责收掉锁定弹窗
    if (mine === searchSeq) searching.value = false;
  }
}

function submitSearch() {
  page.value = 0;
  void search();
}

function turnPage(delta: number) {
  const next = page.value + delta;
  if (next < 0 || next > maxPage.value) return;
  page.value = next;
  void search();
}

/** 展开某整合包的版本列表（再次点击收起） */
async function toggleFiles(item: ProjectItemDto) {
  const pid = item.source.pid;
  if (expandedId.value === pid) {
    expandedId.value = "";
    files.value = [];
    return;
  }
  expandedId.value = pid;
  files.value = [];
  filesLoading.value = true;
  try {
    const res = await api.getModpackFiles(source.value, pid, 0, version.value || null);
    files.value = res.list;
  } catch {
    files.value = [];
  } finally {
    filesLoading.value = false;
  }
}

function install(item: ProjectItemDto, file: FileListItemDto) {
  emit("install", {
    source: item.source.source,
    projectId: item.source.pid,
    fileId: file.source.fid,
    fileName: item.name,
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

onMounted(loadSources);

watch(source, loadSource);
</script>

<template>
  <div class="modpack-mode">
    <!-- 过滤条件 -->
    <div class="modpack-filters">
      <SegmentedTabs v-model="source" :options="sources" />
      <select v-model="version" class="field-input sel-version" @change="submitSearch">
        <option value="">{{ t("modpack.allVersions") }}</option>
        <option v-for="v in versions" :key="v" :value="v">{{ v }}</option>
      </select>
      <select v-model="sort" class="field-input sel-sort" @change="submitSearch">
        <option v-for="s in sorts" :key="s" :value="s">{{ t(`modpack.sort.${s}`) }}</option>
      </select>
      <select v-model="category" class="field-input sel-category" @change="submitSearch">
        <option value="">{{ t("modpack.allCategories") }}</option>
        <option v-for="c in categories" :key="c.value" :value="c.value">{{ c.label }}</option>
      </select>
      <input
        :value="props.group"
        class="field-input sel-group"
        list="modpack-groups"
        :placeholder="t('add.groupPlaceholder')"
        spellcheck="false"
        @input="emit('update:group', ($event.target as HTMLInputElement).value)"
      />
      <datalist id="modpack-groups">
        <option v-for="g in props.groups" :key="g" :value="g" />
      </datalist>
    </div>

    <!-- 搜索 -->
    <div class="modpack-search">
      <input
        v-model="filter"
        class="field-input search-input"
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
      <div v-else-if="error" class="empty-tip">{{ error }}</div>
      <div v-else-if="items.length === 0" class="empty-tip">{{ t("modpack.empty") }}</div>
      <template v-else>
        <div v-for="item in items" :key="item.source.pid" class="pack-item">
          <div class="pack-main" @click="toggleFiles(item)">
            <img v-if="item.image" class="pack-icon" :src="item.image" loading="lazy" alt="" />
            <div v-else class="pack-icon pack-icon-fallback">{{ item.name.slice(0, 1).toUpperCase() }}</div>
            <div class="pack-info">
              <div class="pack-name">
                <span>{{ item.name }}</span>
                <span v-if="item.authors.length" class="pack-author">
                  {{ item.authors.map((a) => a.name).join(", ") }}
                </span>
                <span v-if="item.download" class="pack-badge">{{ t("modpack.installed") }}</span>
                <span v-else-if="item.downloadNow" class="pack-badge busy">{{ t("modpack.downloading") }}</span>
              </div>
              <div class="pack-desc">{{ item.summary }}</div>
              <div v-if="item.tag.length" class="pack-tags">
                <span v-for="tag in item.tag" :key="tag.name" class="pack-tag">
                  <img v-if="tag.logo" class="pack-tag-icon" :src="tag.logo" loading="lazy" alt="" />
                  {{ tag.name }}
                </span>
              </div>
              <div class="pack-meta">
                {{ t("modpack.downloads", { n: item.downloadCount.toLocaleString() }) }}
                <template v-if="item.date"> · {{ formatDate(item.date) }}</template>
              </div>
            </div>
            <button
              v-if="item.url"
              class="pack-link"
              :title="t('modpack.openPage')"
              @click.stop="api.openUrl(item.url)"
            >
              {{ t("modpack.openPage") }}
            </button>
          </div>

          <!-- 版本列表（点击展开） -->
          <div v-if="expandedId === item.source.pid" class="pack-files">
            <div v-if="filesLoading" class="empty-tip">{{ t("modpack.loadingVersions") }}</div>
            <div v-else-if="files.length === 0" class="empty-tip">{{ t("modpack.noVersions") }}</div>
            <div v-for="file in files" :key="file.source.fid" class="file-row">
              <div class="file-info">
                <span class="file-name">
                  {{ file.name }}
                  <span v-if="file.isDownload" class="pack-badge">{{ t("modpack.installed") }}</span>
                  <span v-else-if="file.downloadNow" class="pack-badge busy">{{ t("modpack.downloading") }}</span>
                </span>
                <span class="file-meta">
                  {{ t("modpack.fileMeta", {
                    n: file.download.toLocaleString(),
                    date: formatDate(file.time),
                    size: formatSize(file.size),
                  }) }}
                </span>
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

    <!-- 搜索期间锁定窗口：不置 closable 也不接 close，点击遮罩无效 -->
    <BaseModal v-if="searching" :width="240" :closable="false" below-titlebar>
      <div class="search-lock">
        <span class="search-spinner"></span>
        <span>{{ t("modpack.loading") }}</span>
      </div>
    </BaseModal>
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

/* 四个筛选项平分整行剩余宽度（基宽 110px），窗口太窄时换行 */
.sel-version,
.sel-sort,
.sel-category,
.sel-group {
  flex: 1 1 110px;
  min-width: 110px;
  padding: 8px 10px;
  font-size: 13px;
}

/* 不设 align-items：靠默认 stretch 让按钮（无固定高、无纵向 padding）与输入框等高 */
.modpack-search {
  display: flex;
  gap: 8px;
}

/* 搜索框吃掉剩余宽度（按钮固定宽） */
.search-input {
  flex: 1;
  min-width: 0;
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

/* 搜索期间锁定窗口的弹窗内容 */
.search-lock {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 13px;
  color: var(--text);
}

.search-spinner {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  animation: search-spin 0.8s linear infinite;
  flex-shrink: 0;
}

@keyframes search-spin {
  to {
    transform: rotate(360deg);
  }
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
  align-items: center;
  gap: 8px;
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text);
}

.pack-author {
  font-size: 11.5px;
  font-weight: 400;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.pack-badge {
  padding: 1px 7px;
  border-radius: 6px;
  background: var(--accent-soft);
  color: var(--accent);
  font-size: 10.5px;
  font-weight: 600;
  flex-shrink: 0;
}

.pack-badge.busy {
  background: var(--bg-hover);
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

.pack-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}

.pack-tag {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 1px 7px;
  border: 1px solid var(--border);
  border-radius: 6px;
  font-size: 10.5px;
  color: var(--text-dim);
}

.pack-tag-icon {
  width: 12px;
  height: 12px;
  object-fit: contain;
  border-radius: 3px;
}

.pack-meta {
  font-size: 11.5px;
  color: var(--text-dim);
  opacity: 0.8;
}

.pack-link {
  padding: 4px 10px;
  border: 1px solid var(--border);
  border-radius: 7px;
  background: transparent;
  color: var(--text-dim);
  font-size: 11.5px;
  font-family: inherit;
  cursor: pointer;
  flex-shrink: 0;
  white-space: nowrap;
}

.pack-link:hover {
  color: var(--accent);
  border-color: var(--accent);
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
  display: flex;
  align-items: center;
  gap: 6px;
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
