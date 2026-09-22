<script setup lang="ts">
// 整合包模式：CurseForge / Modrinth 在线搜索 + 选版本安装
// 实例名取自整合包元数据（安装后端自动处理），分组沿用窗口顶部的分组输入框
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { marked } from "marked";
import DOMPurify from "dompurify";
import BaseModal from "../../components/ui/BaseModal.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import AsyncImage from "../../components/ui/AsyncImage.vue";
import { api } from "../../lib/api";
import { t, tErr } from "../../lib/i18n";
import { showToast } from "../../lib/toast";
import type { FileListItemDto, ModPackStatusDto, ProjectDetailDto, ProjectItemDto } from "../../lib/bindings";

const props = defineProps<{
  /** 安装到的分组（空 = 默认分组） */
  group: string;
  /** 已有分组候选（datalist 下拉用） */
  groups: string[];
  /** 安装任务状态（同步列表的「已安装 / 安装中」角标） */
  status: ModPackStatusDto | null;
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

/** 分组组合框：点击输入框即展开已有分组（与添加实例窗口一致） */
const groupOpen = ref(false);
const groupQuery = computed(() =>
  props.groups.filter((g) => g.toLowerCase().includes(props.group.trim().toLowerCase())),
);

function onGroupInput(e: Event) {
  emit("update:group", (e.target as HTMLInputElement).value);
  groupOpen.value = true;
}

function pickGroup(name: string) {
  emit("update:group", name);
  groupOpen.value = false;
}

const items = ref<ProjectItemDto[]>([]);
const total = ref(0);
const page = ref(0);
const searching = ref(false);
const error = ref("");

/** 详情页：当前项目 + 详情数据 + 版本列表 */
const detailItem = ref<ProjectItemDto | null>(null);
const detail = ref<ProjectDetailDto | null>(null);
const detailLoading = ref(false);
const detailError = ref("");
/** CurseForge：后端翻页，当前页结果；Modrinth：一次拿全部，本地切片 */
const files = ref<FileListItemDto[]>([]);
const allFiles = ref<FileListItemDto[]>([]);
const filesLoading = ref(false);
/** 版本列表页码（0 起）与总数 */
const filePage = ref(0);
const fileTotal = ref(0);
/** 最新版本（详情页「下载」按钮用，page 0 的第一条） */
const latestFile = ref<FileListItemDto | null>(null);
/** 详情页版本列表的游戏版本筛选（"" = 全部），打开详情时沿用列表页的筛选 */
const fileVersion = ref("");

/** 版本列表每页条数（与 CurseForge 后端分页一致） */
const FILE_PAGE_SIZE = 50;

const isModrinth = computed(() => source.value === "modrinth");

/** 当前页显示的版本列表 */
const pageFiles = computed(() =>
  isModrinth.value
    ? allFiles.value.slice(filePage.value * FILE_PAGE_SIZE, (filePage.value + 1) * FILE_PAGE_SIZE)
    : files.value,
);

const fileMaxPage = computed(() => Math.max(1, Math.ceil(fileTotal.value / FILE_PAGE_SIZE)));

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

/** 切换下载源：清空搜索词与结果，重新拉排序 / 版本 / 分类，然后搜第一页 */
async function loadSource() {
  items.value = [];
  total.value = 0;
  page.value = 0;
  error.value = "";
  detailItem.value = null;
  detail.value = null;
  files.value = [];
  allFiles.value = [];
  latestFile.value = null;
  fileVersion.value = "";
  filter.value = "";
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

/** 详情代次：连续开关弹窗时丢弃旧一发的返回 */
let detailSeq = 0;

/** 版本列表代次：换筛选 / 换页时丢弃旧一发的返回 */
let fileSeq = 0;

/** 拉取版本列表（fileVersion 过滤 + 指定页）；Modrinth 一次拿全部本地切片，CurseForge 后端分页 */
async function loadFiles(pid: string, page: number) {
  const mine = ++fileSeq;
  filesLoading.value = true;
  try {
    const res = await api.getModpackFiles(source.value, pid, page, fileVersion.value || null);
    if (mine !== fileSeq) return;
    fileTotal.value = res.count || res.list.length;
    // Modrinth 一次返回全部版本，本地切片翻页；CurseForge 走后端分页
    if (isModrinth.value) {
      allFiles.value = res.list;
    } else {
      files.value = res.list;
    }
    if (page === 0) {
      latestFile.value = res.list[0] ?? null;
    }
  } finally {
    if (mine === fileSeq) filesLoading.value = false;
  }
}

/** 双击项目：打开详情页，正文/截图与版本列表并行加载 */
async function openDetail(item: ProjectItemDto) {
  const mine = ++detailSeq;
  const pid = item.source.pid;

  detailItem.value = item;
  detail.value = null;
  detailError.value = "";
  files.value = [];
  allFiles.value = [];
  latestFile.value = null;
  filePage.value = 0;
  fileTotal.value = 0;
  // 版本列表的筛选沿用列表页选中的游戏版本
  fileVersion.value = version.value;
  detailLoading.value = true;

  loadFiles(pid, 0);

  try {
    const res = await api.getModpackDetail(source.value, pid);
    if (mine !== detailSeq) return;
    detail.value = res;
  } catch (e) {
    if (mine !== detailSeq) return;
    detailError.value = tErr(e);
  } finally {
    if (mine === detailSeq) detailLoading.value = false;
  }
}

/** 版本列表筛选变化：回到第一页重新拉取 */
function onFileVersionChange() {
  filePage.value = 0;
  if (detailItem.value) {
    loadFiles(detailItem.value.source.pid, 0);
  }
}

/** 版本列表翻页：Modrinth 本地切片，CurseForge 向后端取页 */
function turnFilePage(delta: number) {
  const next = filePage.value + delta;
  if (next < 0 || next >= fileMaxPage.value) return;
  filePage.value = next;
  if (isModrinth.value || !detailItem.value) return;

  loadFiles(detailItem.value.source.pid, next);
}

/** 下载最新版本（page 0 的第一条） */
function downloadLatest() {
  if (detailItem.value && latestFile.value) {
    install(detailItem.value, latestFile.value);
  }
}

/** 版本列表锚点：头部「版本列表」按钮滚动到这里 */
const versionsRef = ref<HTMLElement | null>(null);

function scrollToVersions() {
  versionsRef.value?.scrollIntoView({ behavior: "smooth", block: "start" });
}

/** 收藏 / 取消收藏（列表星标与详情页星标共用） */
async function toggleStar(item: ProjectItemDto) {
  const star = !item.isStar;
  try {
    await api.collectStar(
      item.source.source,
      item.source.fileType,
      item.source.pid,
      item.name,
      item.image,
      item.url,
      star,
    );
    item.isStar = star;
  } catch (e) {
    showToast(tErr(e));
  }
}

function closeDetail() {
  detailSeq++;
  detailItem.value = null;
  detail.value = null;
  files.value = [];
  allFiles.value = [];
  latestFile.value = null;
  detailError.value = "";
}

function install(item: ProjectItemDto, file: FileListItemDto) {
  emit("install", {
    source: item.source.source,
    projectId: item.source.pid,
    fileId: file.source.fid,
    fileName: item.name,
  });
  closeDetail();
}

/** Modrinth 正文（markdown）渲染成 HTML；CurseForge 没有 body，显示简介 */
const bodyHtml = computed(() => {
  const body = detail.value?.body;
  if (!body) return "";
  return DOMPurify.sanitize(marked.parse(body, { async: false }));
});

/** Modrinth 分类图标：icon 字段是内联 svg 源码（stroke=currentColor），消毒后内联渲染 */
function svgIcon(svg: string): string {
  return DOMPurify.sanitize(svg);
}

/** 截图放大预览（当前预览的图片地址，空 = 关闭） */
const preview = ref("");

/** 预览图缩放倍率（滚轮调节，1 = 适配大小） */
const previewScale = ref(1);

function openPreview(url: string) {
  preview.value = url;
  previewScale.value = 1;
}

function closePreview() {
  preview.value = "";
  previewScale.value = 1;
}

/** 滚轮缩放预览图（向上放大、向下缩小，1~8 倍） */
function onPreviewWheel(e: WheelEvent) {
  const factor = e.deltaY < 0 ? 1.1 : 1 / 1.1;
  previewScale.value = Math.min(8, Math.max(1, previewScale.value * factor));
}

/** Esc：先关截图预览，再关详情页 */
function onDetailKey(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  if (preview.value) {
    closePreview();
    return;
  }
  closeDetail();
}

watch(detailItem, (val) => {
  if (val) window.addEventListener("keydown", onDetailKey);
  else window.removeEventListener("keydown", onDetailKey);
});

onUnmounted(() => window.removeEventListener("keydown", onDetailKey));

function formatDate(date: string): string {
  return date ? date.slice(0, 10) : "";
}

/** 安装状态变化时同步列表角标：项目级「已安装」按 pid，版本级按 pid+fid */
watch(
  () => props.status,
  (status) => {
    if (!status) return;
    const isRunning = (t: { done: boolean; failed: boolean; cancelled: boolean }) =>
      !t.done && !t.failed && !t.cancelled;
    const doneFiles = new Set(
      status.tasks.filter((t) => t.done).map((t) => `${t.pid}|${t.fid}`),
    );
    const doneProjects = new Set(status.tasks.filter((t) => t.done).map((t) => t.pid));
    const runningFiles = new Set(
      status.tasks.filter(isRunning).map((t) => `${t.pid}|${t.fid}`),
    );
    const runningProjects = new Set(status.tasks.filter(isRunning).map((t) => t.pid));

    for (const item of items.value) {
      if (doneProjects.has(item.source.pid)) item.download = true;
      if (runningProjects.has(item.source.pid)) item.downloadNow = true;
    }
    for (const file of [...files.value, ...allFiles.value]) {
      const key = `${file.source.pid}|${file.source.fid}`;
      if (doneFiles.has(key)) {
        file.isDownload = true;
        file.downloadNow = false;
      } else if (runningFiles.has(key)) {
        file.downloadNow = true;
      }
    }
  },
);

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
    <!-- 过滤条件 + 搜索：同一块白底卡片 -->
    <div class="modpack-top">
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
      <div class="group-combo sel-group">
        <input
          :value="props.group"
          class="field-input"
          :placeholder="t('modpack.groupPlaceholder')"
          spellcheck="false"
          @focus="groupOpen = true"
          @input="onGroupInput"
          @blur="groupOpen = false"
        />
        <div v-if="groupOpen" class="group-drop">
          <button
            v-for="g in groupQuery"
            :key="g"
            class="group-opt"
            @mousedown.prevent
            @click="pickGroup(g)"
          >
            {{ g }}
          </button>
          <div v-if="!groupQuery.length" class="empty-tip">{{ t("add.groupNone") }}</div>
        </div>
      </div>
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
    </div>

    <!-- 结果列表（搜索中由锁定弹窗提示，这里不再重复显示） -->
    <div class="modpack-list">
      <div v-if="error" class="empty-tip">{{ error }}</div>
      <div v-else-if="!searching && items.length === 0" class="empty-tip">{{ t("modpack.empty") }}</div>
      <template v-else>
        <div v-for="item in items" :key="item.source.pid" class="pack-item">
          <!-- 收藏星标（右上角）：点击收藏 / 取消收藏 -->
          <button
            class="pack-star"
            :class="{ on: item.isStar }"
            :title="item.isStar ? t('modpack.unstar') : t('modpack.star')"
            @click.stop="toggleStar(item)"
          >
            <svg viewBox="0 0 24 24" width="17" height="17" stroke-width="2" stroke-linejoin="round">
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
            </svg>
          </button>
          <div class="pack-main" :title="t('modpack.detailHint')" @dblclick="openDetail(item)">
            <AsyncImage v-if="item.image" class="pack-icon" :src="item.image" alt="" />
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
                  <span v-if="tag.svg" class="tag-svg" v-html="svgIcon(tag.svg)"></span>
                  <img v-else-if="tag.logo" class="pack-tag-icon" :src="tag.logo" loading="lazy" alt="" />
                  {{ tag.name }}
                </span>
              </div>
              <div class="pack-meta">
                {{ t("modpack.downloads", { n: item.downloadCount.toLocaleString() }) }}
                <template v-if="item.date"> · {{ formatDate(item.date) }}</template>
              </div>
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

    <!-- 截图放大预览：滚轮缩放，点任意处或 Esc 关闭 -->
    <Teleport to="body">
      <transition name="shot-fade">
        <div v-if="preview" class="shot-preview" @click="closePreview" @wheel.prevent="onPreviewWheel">
          <img :src="preview" alt="" :style="{ transform: `scale(${previewScale})` }" />
        </div>
      </transition>
    </Teleport>

    <!-- 项目详情（双击列表项打开）：整页覆盖 + 进出场动画，有正文只显示正文，没有才显示简介 -->
    <Teleport to="body">
      <transition name="detail-page">
        <div v-if="detailItem" class="detail-page">
          <!-- 头部 + 内容合在一块面板里：头部固定、内容滚动，滚动条贴面板右缘 -->
          <div class="detail-panel">
          <div class="detail-head">
            <button class="detail-back" :title="t('modpack.back')" @click="closeDetail">
              <svg viewBox="0 0 24 24" width="18" height="18" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <line x1="19" y1="12" x2="5" y2="12" />
                <polyline points="12 19 5 12 12 5" />
              </svg>
            </button>
            <AsyncImage v-if="detailItem.image" class="detail-icon" :src="detailItem.image" alt="" />
            <div v-else class="detail-icon detail-icon-fallback">
              {{ detailItem.name.slice(0, 1).toUpperCase() }}
            </div>
            <div class="detail-title">
              <div class="detail-name">
                {{ detailItem.name }}
                <span v-if="detailItem.download" class="pack-badge">{{ t("modpack.installed") }}</span>
                <span v-else-if="detailItem.downloadNow" class="pack-badge busy">{{ t("modpack.downloading") }}</span>
              </div>
              <div class="detail-sub">
                <span v-if="detailItem.authors.length">{{ detailItem.authors.map((a) => a.name).join(", ") }}</span>
                <span> · {{ t("modpack.downloads", { n: detailItem.downloadCount.toLocaleString() }) }}</span>
                <span v-if="detailItem.date"> · {{ formatDate(detailItem.date) }}</span>
              </div>
              <div v-if="detail?.tag.length || detailItem.tag.length" class="pack-tags detail-tags">
                <span
                  v-for="tag in detail?.tag.length ? detail.tag : detailItem.tag"
                  :key="tag.name"
                  class="pack-tag"
                >
                  <span v-if="tag.svg" class="tag-svg" v-html="svgIcon(tag.svg)"></span>
                  <img v-else-if="tag.logo" class="pack-tag-icon" :src="tag.logo" loading="lazy" alt="" />
                  {{ tag.name }}
                </span>
              </div>
            </div>
            <button class="detail-jump" @click="scrollToVersions">
              {{ t("modpack.versions") }}
            </button>
            <button
              class="detail-download"
              :disabled="filesLoading || !latestFile"
              @click="downloadLatest"
            >
              {{ t("modpack.download") }}
            </button>
            <button
              class="detail-star"
              :class="{ on: detailItem.isStar }"
              :title="detailItem.isStar ? t('modpack.unstar') : t('modpack.star')"
              @click="toggleStar(detailItem)"
            >
              <svg viewBox="0 0 24 24" width="20" height="20" stroke-width="2" stroke-linejoin="round">
                <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
              </svg>
            </button>
            <button
              v-if="detailItem.url"
              class="detail-link"
              :title="t('modpack.openPage')"
              @click="api.openUrl(detailItem.url)"
            >
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                <polyline points="15 3 21 3 21 9" />
                <line x1="10" y1="14" x2="21" y2="3" />
              </svg>
            </button>
          </div>

          <div class="detail-scroll">
            <div v-if="detailLoading" class="empty-tip">{{ t("modpack.loading") }}</div>
            <div v-else-if="detailError" class="empty-tip">{{ detailError }}</div>
            <!-- 简介 / 截图 / 版本列表合并为一块大卡片，分区间距隔开，避免多框割裂 -->
            <div class="detail-card">
              <div v-if="detailLoading" class="empty-tip">{{ t("modpack.loading") }}</div>
              <div v-else-if="detailError" class="empty-tip">{{ detailError }}</div>
              <template v-else-if="detail">
                <!-- Modrinth：正文；CurseForge：简介。标题都用「项目简介」 -->
                <div class="detail-section">{{ t("modpack.summary") }}</div>
                <article v-if="bodyHtml" class="detail-md" v-html="bodyHtml"></article>
                <div v-else class="detail-summary">{{ detail.summary }}</div>

                <template v-if="detail.screenshots.length">
                  <div class="detail-section detail-block">{{ t("modpack.screenshots") }}</div>
                  <div class="detail-shots">
                    <AsyncImage
                      v-for="shot in detail.screenshots"
                      :key="shot.logo"
                      class="detail-shot"
                      :src="shot.logo"
                      :title="shot.name || shot.description"
                      @click="openPreview(shot.logo)"
                    />
                  </div>
                </template>
              </template>

              <div ref="versionsRef" class="detail-block versions-anchor">
                <div class="detail-section detail-versions-head">
                  <span>{{ t("modpack.versions") }}</span>
                  <div class="version-tools">
                    <select v-model="fileVersion" class="field-input sel-file-version" @change="onFileVersionChange">
                      <option value="">{{ t("modpack.allVersions") }}</option>
                      <option v-for="v in versions" :key="v" :value="v">{{ v }}</option>
                    </select>
                    <div v-if="fileMaxPage > 1" class="file-page">
                      <button class="page-btn" :disabled="filePage === 0 || filesLoading" @click="turnFilePage(-1)">
                        {{ t("modpack.prevPage") }}
                      </button>
                      <span class="file-page-num">{{ filePage + 1 }} / {{ fileMaxPage }}</span>
                      <button class="page-btn" :disabled="filePage >= fileMaxPage - 1 || filesLoading" @click="turnFilePage(1)">
                        {{ t("modpack.nextPage") }}
                      </button>
                    </div>
                  </div>
                </div>
                <div class="detail-files">
                  <div v-if="filesLoading" class="empty-tip">{{ t("modpack.loadingVersions") }}</div>
                  <div v-else-if="pageFiles.length === 0" class="empty-tip">{{ t("modpack.noVersions") }}</div>
                  <div v-for="file in pageFiles" :key="file.source.fid" class="file-row">
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
                    <button class="install-btn" @click="install(detailItem, file)">
                      {{ t("modpack.install") }}
                    </button>
                  </div>
                </div>
              </div>
            </div>
          </div>
          </div>
        </div>
      </transition>
    </Teleport>
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

/* 分组是组合框（外层 div 包输入框 + 下拉），内边距和字号落到输入框上 */
.sel-group {
  padding: 0;
}

.sel-group .field-input {
  padding: 8px 10px;
  font-size: 13px;
}

/* 分组组合框：点击输入框展开候选分组（与添加实例窗口一致） */
.group-combo {
  position: relative;
}

.group-drop {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
  right: 0;
  z-index: 20;
  max-height: 180px;
  overflow-y: auto;
  padding: 4px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
}

.group-opt {
  display: block;
  width: 100%;
  padding: 8px 10px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
}

.group-opt:hover {
  background: var(--bg-hover);
}

/* 顶部一块白底卡片：筛选行 + 搜索行都框在里面 */
.modpack-top {
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 10px;
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
  transition: filter 0.12s ease, opacity 0.12s ease;
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

/* 截图放大预览 */
.shot-preview {
  position: fixed;
  /* 不盖住标题栏 */
  inset: var(--titlebar-h) 0 0 0;
  z-index: 200;
  background: rgb(0 0 0 / 85%);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: zoom-out;
  /* 放大后溢出部分裁掉 */
  overflow: hidden;
}

.shot-preview img {
  max-width: 92%;
  max-height: 92%;
  object-fit: contain;
  box-shadow: var(--shadow-lg);
}

.shot-fade-enter-active,
.shot-fade-leave-active {
  transition: opacity 0.15s ease;
}

.shot-fade-enter-from,
.shot-fade-leave-to {
  opacity: 0;
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
  position: relative;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-side);
  overflow: hidden;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}

.pack-item:hover {
  border-color: var(--accent-border);
  box-shadow: 0 4px 14px rgb(0 0 0 / 12%);
}

/* 收藏星标（右上角）：悬停卡片时出现，已收藏则常显；可点击收藏 / 取消收藏。
   星色固定用黄色（收藏的通用印象），不跟主题走 */
.pack-star {
  position: absolute;
  top: 5px;
  right: 5px;
  z-index: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 28px;
  height: 28px;
  border: none;
  border-radius: 8px;
  background: transparent;
  padding: 0;
  color: var(--border);
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.12s ease, color 0.12s ease;
}

.pack-star:hover {
  color: #f6b500;
}

.pack-item:hover .pack-star {
  opacity: 1;
}

.pack-star.on {
  opacity: 1;
}

.pack-star svg {
  fill: none;
  stroke: currentColor;
}

.pack-star.on {
  color: #f6b500;
}

.pack-star.on svg {
  fill: currentColor;
}

.pack-main {
  width: 100%;
  display: flex;
  align-items: center;
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
  width: 90px;
  height: 90px;
  border-radius: 14px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--bg-card);
}

.pack-icon-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 22px;
  color: var(--accent);
  background: var(--accent-soft);
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
  /* 单行：标题超长省略，作者占位但不超过一半，不会把标题顶成两行 */
  flex-wrap: nowrap;
  white-space: nowrap;
}

.pack-name > span:first-child {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pack-author {
  flex-shrink: 0;
  max-width: 45%;
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

/* 固定两行高，保证每张卡片高度一致 */
.pack-desc {
  font-size: 12px;
  line-height: 18px;
  height: 36px;
  color: var(--text-dim);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

/* 固定单行高，超出省略（没有标签也占位，卡片高度统一） */
.pack-tags {
  display: flex;
  flex-wrap: nowrap;
  gap: 5px;
  height: 22px;
  overflow: hidden;
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
  height: 16px;
  line-height: 16px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 项目详情：整页覆盖（自绘标题栏以下）+ 进出场动画。
   一整块白色面板：头部固定在上、内容滚动在下，滚动条贴面板右缘，
   和列表页的结果列表卡片（滚动条在卡内）同一套观感 */
.detail-page {
  position: fixed;
  inset: var(--titlebar-h) 0 0 0;
  z-index: 100;
  background: var(--bg);
  display: flex;
  flex-direction: column;
  padding: 12px;
}

.detail-panel {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}

.detail-page-enter-active,
.detail-page-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}

.detail-page-enter-from,
.detail-page-leave-to {
  opacity: 0;
  transform: translateY(14px);
}

.detail-back {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 10px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  flex-shrink: 0;
}

.detail-back:hover {
  background: var(--bg-hover);
  color: var(--text);
}

/* 面板头部：固定在面板顶部，用分隔线与内容区分开 */
.detail-head {
  display: flex;
  align-items: center;
  gap: 14px;
  flex-shrink: 0;
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}

.detail-icon {
  width: 64px;
  height: 64px;
  border-radius: 12px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--bg-side);
}

.detail-icon-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 24px;
  color: var(--accent);
  background: var(--accent-soft);
}

.detail-title {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.detail-name {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 18px;
  font-weight: 700;
  color: var(--text);
}

.detail-sub {
  font-size: 12.5px;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* 详情头部的标签可换行，不受列表卡片的单行约束 */
.detail-tags {
  flex-wrap: wrap;
  height: auto;
  overflow: visible;
}

/* Modrinth 分类图标：icon 字段是内联 svg 源码，stroke=currentColor 继承这里设置的文字色 */
.tag-svg {
  display: inline-flex;
  width: 14px;
  height: 14px;
  flex-shrink: 0;
  color: var(--text-dim);
}

.tag-svg :deep(svg) {
  width: 100%;
  height: 100%;
}

/* 详情页收藏星标：与外链按钮同规格，收藏后实心黄色（与列表星标一致） */
.detail-star {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border: none;
  border-radius: 9px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  flex-shrink: 0;
  transition: color 0.12s ease, background 0.12s ease;
}

.detail-star svg {
  fill: none;
  stroke: currentColor;
}

.detail-star.on {
  color: #f6b500;
}

.detail-star.on svg {
  fill: currentColor;
}

.detail-star:hover {
  background: var(--bg-hover);
  color: #f6b500;
}

.detail-link {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 34px;
  height: 34px;
  border: none;
  border-radius: 9px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  flex-shrink: 0;
}

.detail-link:hover {
  background: var(--bg-hover);
  color: var(--accent);
}

.detail-scroll {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  padding: 12px 14px 16px;
}

/* 内容区容器（无边框，面板本身就是卡片） */
.detail-card {
  padding: 0;
}

/* 卡片内分区的上间距（截图 / 版本列表） */
.detail-block {
  margin-top: 16px;
}

/* 「版本列表」跳转按钮的滚动落点 */
.versions-anchor {
  scroll-margin-top: 8px;
}

.detail-summary {
  font-size: 13.5px;
  color: var(--text);
  line-height: 1.65;
}

.detail-section {
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 8px;
}

/* 版本列表标题行：标题 + 版本筛选 + 翻页控件 */
.detail-versions-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.version-tools {
  display: flex;
  align-items: center;
  gap: 8px;
}

/* 版本筛选下拉：比列表页的筛选项小一号 */
.sel-file-version {
  max-width: 150px;
  padding: 5px 8px;
  font-size: 12px;
}

.file-page {
  display: flex;
  align-items: center;
  gap: 8px;
}

.file-page-num {
  font-size: 12px;
  color: var(--text-dim);
  min-width: 48px;
  text-align: center;
}

/* 头部「版本列表」跳转按钮：点击滚动到版本区 */
.detail-jump {
  padding: 9px 14px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: transparent;
  color: var(--text-dim);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  flex-shrink: 0;
  white-space: nowrap;
  transition: color 0.12s ease, border-color 0.12s ease;
}

.detail-jump:hover {
  color: var(--accent);
  border-color: var(--accent-border);
}

.detail-download {
  padding: 9px 22px;
  border: none;
  border-radius: 10px;
  background: var(--accent-grad);
  color: #fff;
  font-size: 13px;
  font-family: inherit;
  font-weight: 600;
  cursor: pointer;
  flex-shrink: 0;
  white-space: nowrap;
}

.detail-download:hover:enabled {
  filter: brightness(1.1);
}

.detail-download:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.detail-shots {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 10px;
}

.detail-shot {
  width: 100%;
  aspect-ratio: 16 / 9;
  object-fit: cover;
  border-radius: 10px;
  background: var(--bg-side);
  cursor: zoom-in;
  transition: filter 0.15s ease;
}

.detail-shot:hover {
  filter: brightness(1.08);
}

/* 版本列表：无外框，行间用分隔线 */
.detail-files {
  display: flex;
  flex-direction: column;
}

/* Modrinth 正文 markdown 排版（v-html 注入，需要 :deep 穿透） */
.detail-md {
  font-size: 13.5px;
  line-height: 1.65;
  color: var(--text);
  word-break: break-word;
}

.detail-md :deep(h1),
.detail-md :deep(h2),
.detail-md :deep(h3),
.detail-md :deep(h4) {
  margin: 16px 0 8px;
  font-weight: 700;
  line-height: 1.3;
}

.detail-md :deep(h1):first-child,
.detail-md :deep(h2):first-child,
.detail-md :deep(h3):first-child {
  margin-top: 0;
}

.detail-md :deep(h1) {
  font-size: 18px;
}

.detail-md :deep(h2) {
  font-size: 16px;
}

.detail-md :deep(h3) {
  font-size: 14.5px;
}

.detail-md :deep(h4) {
  font-size: 13.5px;
}

.detail-md :deep(p) {
  margin: 8px 0;
}

.detail-md :deep(a) {
  color: var(--accent);
}

.detail-md :deep(ul),
.detail-md :deep(ol) {
  margin: 8px 0;
  padding-left: 22px;
}

.detail-md :deep(code) {
  background: var(--bg-side);
  border-radius: 4px;
  padding: 1px 5px;
  font-size: 12.5px;
}

.detail-md :deep(pre) {
  background: var(--bg-side);
  border-radius: 8px;
  padding: 10px 12px;
  overflow-x: auto;
}

.detail-md :deep(pre code) {
  background: none;
  padding: 0;
}

.detail-md :deep(img) {
  max-width: 100%;
  border-radius: 8px;
}

.detail-md :deep(blockquote) {
  margin: 8px 0;
  padding: 2px 12px;
  border-left: 3px solid var(--border);
  color: var(--text-dim);
}

.detail-md :deep(table) {
  border-collapse: collapse;
  margin: 8px 0;
}

.detail-md :deep(th),
.detail-md :deep(td) {
  border: 1px solid var(--border);
  padding: 4px 10px;
}

.detail-md :deep(hr) {
  border: none;
  border-top: 1px solid var(--border);
  margin: 12px 0;
}

.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border-radius: 8px;
  transition: background 0.12s ease;
}

.file-row + .file-row {
  border-top: 1px solid var(--border);
}

.file-row:hover {
  background: var(--bg-side);
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
  font-size: 13px;
  font-weight: 600;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-meta {
  font-size: 11.5px;
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
  transition: color 0.12s ease, border-color 0.12s ease, opacity 0.12s ease;
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
