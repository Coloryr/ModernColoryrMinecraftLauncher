<script setup lang="ts">
// 添加资源窗口：模组 / 材质包 / 光影包 / 存档 / 数据包 在线搜索 + 下载
//
// 从主界面「添加资源」按钮打开，目标实例取 gui_config.json 里主窗口
// 当前选中的实例（与资源管理窗口一致）。数据包下载前必须先选存档。
// 下载走全局下载器（命令立即返回，进度在下载窗口里看）。
import { computed, onDeactivated, onMounted, onUnmounted, ref, watch } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import AsyncImage from "../../components/ui/AsyncImage.vue";
import ResourceDownloadBar from "../../components/ResourceDownloadBar.vue";
import { api } from "../../lib/api";
import { t, tErr } from "../../lib/i18n";
import { showToast } from "../../lib/toast";
import { loadGuiConfig } from "../../lib/guiConfig";
import { useResourceStatus } from "../../lib/resourceTasks";
import type { FileListItemDto, ProjectItemDto, ResourceSaveDto, ResourceTaskDto } from "../../lib/bindings";

defineEmits<{ (e: "close"): void }>();

/** 下载任务状态（本窗口是"负责显示"的窗口） */
const { status: resourceStatus, init: initResourceStatus } = useResourceStatus(true);

/** 后端列表每页 20 个项目 */
const PAGE_SIZE = 20;
/** 版本列表每页条数（与 CurseForge 后端分页一致） */
const FILE_PAGE_SIZE = 50;

/** 资源类型（取值是后端 FileType 线串，显示标签走 i18n） */
const TYPES = [
  { value: "mod", label: t("addResource.mods") },
  { value: "resourcepack", label: t("addResource.resourcepacks") },
  { value: "shaderpack", label: t("addResource.shaders") },
  { value: "save", label: t("addResource.saves") },
  { value: "dataPacks", label: t("addResource.datapacks") },
];

const SOURCE_LABELS: Record<string, string> = {
  curseforge: "modpack.curseforge",
  modrinth: "modpack.modrinth",
};

/** 模组可按加载器过滤（normal = 全部） */
const LOADERS = ["normal", "forge", "fabric", "quilt", "neoforge"];

/** 目标实例（uuid + 显示名），空 = 未选择 */
const instanceUuid = ref("");
const instanceName = ref("");

const type = ref("mod");
const sources = ref<Array<{ value: string; label: string }>>([]);
const source = ref("");

const sorts = ref<string[]>([]);
const sort = ref("");
const versions = ref<string[]>([]);
const version = ref("");
/** 分类，"" = 全部。键 = 传给后端的分类值，值 = 显示名 */
const categories = ref<Array<{ value: string; label: string }>>([]);
const category = ref("");
/** 加载器（仅模组；normal = 全部） */
const loader = ref("normal");
const filter = ref("");

// 离开本窗口时清掉搜索词（来源 / 类型 / 版本等**筛选**留着：那是浏览上下文，
// 搜索词只是"找某样东西"的一次性输入）。
// 单窗口模式下这一页被 KeepAlive 缓存，切走不销毁，不清的话再打开时输入框里
// 还压着上次搜的词，用户早忘了自己搜过什么，只会觉得界面不对劲。
onDeactivated(() => {
  filter.value = "";
});

const items = ref<ProjectItemDto[]>([]);
const total = ref(0);
const page = ref(0);
const searching = ref(false);
const error = ref("");

/** 版本选择弹窗：当前项目 + 版本列表 */
const versionItem = ref<ProjectItemDto | null>(null);
const files = ref<FileListItemDto[]>([]);
const allFiles = ref<FileListItemDto[]>([]);
const filesLoading = ref(false);
const filePage = ref(0);
const fileTotal = ref(0);
const fileVersion = ref("");

/** 存档选择弹窗（数据包下载前）：待装的文件 + 存档列表 */
const savePickFile = ref<FileListItemDto | null>(null);
const saves = ref<ResourceSaveDto[]>([]);
const savesLoading = ref(false);
const pickedSave = ref("");

const maxPage = computed(() => Math.max(0, Math.ceil(total.value / PAGE_SIZE) - 1));

const isModrinth = computed(() => source.value === "modrinth");

/** 当前页显示的版本列表：Modrinth 一次拿全部本地切片，CurseForge 后端分页 */
const pageFiles = computed(() =>
  isModrinth.value
    ? allFiles.value.slice(filePage.value * FILE_PAGE_SIZE, (filePage.value + 1) * FILE_PAGE_SIZE)
    : files.value,
);
const fileMaxPage = computed(() => Math.max(1, Math.ceil(fileTotal.value / FILE_PAGE_SIZE)));

/** 存档下载只有 CurseForge 有（Modrinth 没有 world 类型） */
const sourceOptions = computed(() =>
  type.value === "save"
    ? sources.value.filter((s) => s.value === "curseforge")
    : sources.value,
);

const isSaveType = computed(() => type.value === "save");

/** 拉取下载源列表并选中第一个 */
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
  source.value = sourceOptions.value[0]?.value ?? sources.value[0].value;
}

/** 切换类型 / 下载源：清空搜索词与结果，重新拉排序 / 版本 / 分类，然后搜第一页 */
async function loadSource() {
  items.value = [];
  total.value = 0;
  page.value = 0;
  error.value = "";
  filter.value = "";
  version.value = "";
  category.value = "";
  loader.value = "normal";
  sort.value = "";
  sorts.value = [];
  versions.value = [];
  categories.value = [];

  searching.value = true;
  try {
    const [sortList, versionList, categoryMap] = await Promise.all([
      api.getResourceSorts(source.value),
      api.getResourceVersions(source.value),
      api.getResourceCategories(source.value, type.value),
    ]);

    sorts.value = sortList;
    sort.value = sortList[0] ?? "";
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

/** 搜索代次：新的一发开始后，旧一发的返回全部丢弃 */
let searchSeq = 0;

async function search() {
  if (!instanceUuid.value) {
    error.value = t("addResource.noInstance");
    return;
  }
  const mine = ++searchSeq;
  searching.value = true;
  try {
    const res = await api.searchResources(
      instanceUuid.value,
      source.value,
      type.value,
      page.value,
      sort.value,
      category.value || null,
      filter.value.trim() || null,
      version.value || null,
      type.value === "mod" && LOADERS.includes(loader.value) ? loader.value : null,
    );
    if (mine !== searchSeq) return;
    items.value = res.items;
    total.value = res.count;
    error.value = "";
  } catch (e) {
    if (mine !== searchSeq) return;
    items.value = [];
    total.value = 0;
    error.value = tErr(e);
  } finally {
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

/** 版本列表代次：换筛选 / 换页时丢弃旧一发的返回 */
let fileSeq = 0;

async function loadFiles(pid: string, pageNo: number) {
  if (!instanceUuid.value) return;
  const mine = ++fileSeq;
  filesLoading.value = true;
  try {
    const res = await api.getResourceFiles(
      instanceUuid.value,
      source.value,
      pid,
      type.value,
      pageNo,
      fileVersion.value || null,
      null,
    );
    if (mine !== fileSeq) return;
    fileTotal.value = res.count || res.list.length;
    if (isModrinth.value) {
      allFiles.value = res.list;
    } else {
      files.value = res.list;
    }
  } catch (e) {
    if (mine === fileSeq) showToast(tErr(e));
  } finally {
    if (mine === fileSeq) filesLoading.value = false;
  }
}

/** 点击项目：打开版本选择弹窗（筛选沿用列表页选中的游戏版本） */
function openVersions(item: ProjectItemDto) {
  versionItem.value = item;
  files.value = [];
  allFiles.value = [];
  filePage.value = 0;
  fileTotal.value = 0;
  fileVersion.value = version.value;
  loadFiles(item.source.pid, 0);
}

function closeVersions() {
  fileSeq++;
  versionItem.value = null;
  files.value = [];
  allFiles.value = [];
}

function onFileVersionChange() {
  filePage.value = 0;
  if (versionItem.value) {
    loadFiles(versionItem.value.source.pid, 0);
  }
}

function turnFilePage(delta: number) {
  const next = filePage.value + delta;
  if (next < 0 || next >= fileMaxPage.value) return;
  filePage.value = next;
  if (isModrinth.value || !versionItem.value) return;
  loadFiles(versionItem.value.source.pid, next);
}

/** 下载资源：数据包先选存档，其余直接加入下载 */
async function download(file: FileListItemDto) {
  if (!versionItem.value || !instanceUuid.value) return;
  const pid = versionItem.value.source.pid;

  if (type.value === "dataPacks") {
    savePickFile.value = file;
    pickedSave.value = "";
    savesLoading.value = true;
    saves.value = [];
    try {
      saves.value = await api.getResourceSaves(instanceUuid.value);
    } catch (e) {
      showToast(tErr(e));
    } finally {
      savesLoading.value = false;
    }
    return;
  }

  await doDownload(pid, file);
}

/** 存档选择确认：真正发起数据包下载 */
async function confirmSave() {
  if (!savePickFile.value || !versionItem.value || !pickedSave.value) return;
  const pid = versionItem.value.source.pid;
  const file = savePickFile.value;
  savePickFile.value = null;
  await doDownload(pid, file, pickedSave.value);
}

async function doDownload(pid: string, file: FileListItemDto, world: string | null = null) {
  try {
    await api.downloadResource(
      instanceUuid.value,
      source.value,
      pid,
      file.source.fid,
      type.value,
      world,
    );
    showToast(t("addResource.downloadOk", { name: file.name }));
    closeVersions();
  } catch (e) {
    showToast(tErr(e));
  }
}

function formatDate(date: string): string {
  return date ? date.slice(0, 10) : "";
}

/**
 * 本次会话里见过的「下载完成」标记（只增不减）
 *
 * 任务表的终态条目由后端延时移除，但文件确实已经装进实例了——
 * 「已下载」角标不能跟着任务一起消失。只往集合里加，不做任何复位。
 */
const donePids = ref(new Set<string>());
const doneFileKeys = ref(new Set<string>());

watch(
  resourceStatus,
  (status) => {
    const tasks = status?.tasks ?? [];
    if (!tasks.some((task) => task.done)) return;
    const pids = new Set(donePids.value);
    const files = new Set(doneFileKeys.value);
    for (const task of tasks) {
      if (!task.done) continue;
      pids.add(task.pid);
      files.add(`${task.pid}|${task.fid}`);
    }
    // 只在真新增时换引用：进度事件很频繁，无谓的赋值会带着模板一起重渲染
    if (pids.size !== donePids.value.size) donePids.value = pids;
    if (files.size !== doneFileKeys.value.size) doneFileKeys.value = files;
  },
  { immediate: true },
);

/**
 * 进行中任务派生的角标状态
 *
 * 由 `resourceStatus` 直接**派生**，不再往列表项里写回 `download` / `downloadNow`：
 * 写回是单向的（只置 true 不复位），任务失败或结束后角标会一直挂着。
 * 「已下载」取实例侧来源（`item.download` / `file.isDownload`）与上面那张累计表的「或」。
 */
const taskState = computed(() => {
  const running = (task: ResourceTaskDto) => !task.done && !task.failed;
  const tasks = resourceStatus.value?.tasks ?? [];
  return {
    runningProjects: new Set(tasks.filter(running).map((task) => task.pid)),
    runningFiles: new Set(tasks.filter(running).map((task) => `${task.pid}|${task.fid}`)),
  };
});

function fileKey(file: FileListItemDto): string {
  return `${file.source.pid}|${file.source.fid}`;
}

/** 项目级「已下载」（实例侧已装 + 本次会话装完） */
function projectInstalled(item: ProjectItemDto): boolean {
  return item.download || donePids.value.has(item.source.pid);
}

/** 项目级「下载中」：只看任务表——DTO 里那份是取列表时的快照，用它角标会粘住 */
function projectRunning(item: ProjectItemDto): boolean {
  return taskState.value.runningProjects.has(item.source.pid);
}

/** 文件级「已下载」 */
function fileInstalled(file: FileListItemDto): boolean {
  return file.isDownload || doneFileKeys.value.has(fileKey(file));
}

/** 文件级「下载中」 */
function fileRunning(file: FileListItemDto): boolean {
  return taskState.value.runningFiles.has(fileKey(file));
}

function formatSize(size: number): string {
  if (!size) return t("modpack.unknownSize");
  if (size >= 1024 * 1024) return `${(size / 1024 / 1024).toFixed(1)} MB`;
  return `${Math.max(1, Math.round(size / 1024))} KB`;
}

/** 类型 / 下载源变化：换源重载（存档类型强制 CurseForge） */
watch([source, type], () => {
  if (source.value) loadSource();
});

watch(type, () => {
  // Modrinth 没有存档类型，切到存档时把源换到 CurseForge
  if (isSaveType.value && source.value === "modrinth") {
    source.value = "curseforge";
  }
});

onMounted(async () => {
  const unlisten = await initResourceStatus();
  onUnmounted(unlisten);

  try {
    const cfg = await loadGuiConfig();
    instanceUuid.value = cfg?.mainWindow.selectedInstance ?? "";
  } catch {
    instanceUuid.value = "";
  }

  if (instanceUuid.value) {
    try {
      const list = await api.getInstances();
      instanceName.value = list.find((i) => i.uuid === instanceUuid.value)?.name ?? "";
    } catch {
      instanceName.value = "";
    }
  }

  await loadSources();
});
</script>

<template>
  <WindowFrame :title="t('winTitle.addResource')" @close="$emit('close')">
    <div class="res-mode">
      <!-- 下载任务进度条（多任务，完成后短暂停留再自动消失） -->
      <ResourceDownloadBar v-if="resourceStatus?.tasks.length" :status="resourceStatus" />

      <!-- 资源类型 + 下载源 -->
      <div class="res-top">
        <div class="res-filters">
          <SegmentedTabs v-model="type" :options="TYPES" :disabled="searching" />
          <!-- 存档只有 CurseForge 有：锁定源，不显示页签 -->
          <SegmentedTabs v-if="!isSaveType" v-model="source" :options="sourceOptions" :disabled="searching" />
          <div v-else class="res-source-lock">
            <span class="res-source-name">{{ t("modpack.curseforge") }}</span>
            <span class="res-source-hint">{{ t("addResource.saveSourceLimit") }}</span>
          </div>
        </div>

        <!-- 过滤条件 + 搜索：加载期间整条锁死，避免半途改条件与在途请求打架 -->
        <div class="res-filters">
          <select v-model="version" class="field-select sel-filter" :disabled="searching" @change="submitSearch">
            <option value="">{{ t("modpack.allVersions") }}</option>
            <option v-for="v in versions" :key="v" :value="v">{{ v }}</option>
          </select>
          <select v-model="sort" class="field-select sel-filter" :disabled="searching" @change="submitSearch">
            <option v-for="s in sorts" :key="s" :value="s">{{ t(`modpack.sort.${s}`) }}</option>
          </select>
          <select
            v-if="categories.length"
            v-model="category"
            class="field-select sel-filter"
            :disabled="searching"
            @change="submitSearch"
          >
            <option value="">{{ t("modpack.allCategories") }}</option>
            <option v-for="c in categories" :key="c.value" :value="c.value">{{ c.label }}</option>
          </select>
          <select v-if="type === 'mod'" v-model="loader" class="field-select sel-filter" :disabled="searching" @change="submitSearch">
            <option v-for="l in LOADERS" :key="l" :value="l">
              {{ l === "normal" ? t("addResource.loader.normal") : l }}
            </option>
          </select>
        </div>

        <div class="res-search">
          <input
            v-model="filter"
            class="field-input search-input"
            :placeholder="t('addResource.searchHint')"
            spellcheck="false"
            :disabled="searching"
            @keydown.enter="submitSearch"
          />
          <button class="search-btn" :disabled="searching" @click="submitSearch">
            <span v-if="searching" class="btn-spinner"></span>
            {{ t("modpack.search") }}
          </button>
        </div>
      </div>

      <!-- 目标实例 -->
      <div class="res-instance">
        <span>{{ t("addResource.instance") }}</span>
        <strong v-if="instanceUuid">{{ instanceName || instanceUuid }}</strong>
        <span v-else class="res-no-instance">{{ t("addResource.noInstance") }}</span>
      </div>

      <!-- 列表加载指示：翻页 / 换筛选只在结果区上方走一条细进度，不再全窗口遮罩 -->
      <div class="list-bar" :class="{ on: searching }" aria-hidden="true"><span /></div>

      <!-- 结果列表（加载状态内联在本区，旧内容保留并压暗） -->
      <div class="res-list" :class="{ 'is-loading-dim': searching }">
        <!-- 首次加载（还没有内容）：骨架行 -->
        <template v-if="searching && items.length === 0">
          <div v-for="n in 6" :key="n" class="sk-row">
            <div class="sk sk-icon"></div>
            <div class="sk-lines">
              <div class="sk sk-line w60"></div>
              <div class="sk sk-line w90"></div>
              <div class="sk sk-line w40"></div>
            </div>
          </div>
        </template>
        <div v-else-if="error" class="empty-tip">{{ error }}</div>
        <div v-else-if="items.length === 0" class="empty-tip">{{ t("addResource.empty") }}</div>
        <template v-else>
          <div v-for="item in items" :key="item.source.pid" class="pack-item">
            <div class="pack-main" @click="openVersions(item)">
              <AsyncImage v-if="item.image" class="pack-icon" :src="item.image" alt="" />
              <div v-else class="pack-icon pack-icon-fallback">{{ item.name.slice(0, 1).toUpperCase() }}</div>
              <div class="pack-info">
                <div class="pack-name">
                  <span>{{ item.name }}</span>
                  <span v-if="item.authors.length" class="pack-author">
                    {{ item.authors.map((a) => a.name).join(", ") }}
                  </span>
                  <span v-if="projectInstalled(item)" class="pack-badge">{{ t("modpack.installed") }}</span>
                  <span v-else-if="projectRunning(item)" class="pack-badge busy">{{ t("modpack.downloading") }}</span>
                </div>
                <div class="pack-desc">{{ item.summary }}</div>
                <div v-if="item.tag.length" class="pack-tags">
                  <span v-for="tag in item.tag" :key="tag.name" class="pack-tag">{{ tag.name }}</span>
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
      <div v-if="maxPage > 0" class="res-page">
        <button class="page-btn" :disabled="page === 0 || searching" @click="turnPage(-1)">
          {{ t("modpack.prevPage") }}
        </button>
        <span class="page-num">{{ page + 1 }} / {{ maxPage + 1 }} · {{ t("modpack.totalItems", { n: total }) }}</span>
        <button class="page-btn" :disabled="page >= maxPage || searching" @click="turnPage(1)">
          {{ t("modpack.nextPage") }}
        </button>
      </div>

      <!-- 版本选择弹窗 -->
      <BaseModal v-if="versionItem" :width="620" below-titlebar @close="closeVersions">
        <div class="ver-head">
          <AsyncImage v-if="versionItem.image" class="ver-icon" :src="versionItem.image" alt="" />
          <div v-else class="ver-icon ver-icon-fallback">
            {{ versionItem.name.slice(0, 1).toUpperCase() }}
          </div>
          <div class="ver-title">
            <div class="ver-name">{{ versionItem.name }}</div>
            <div v-if="versionItem.summary" class="ver-summary">{{ versionItem.summary }}</div>
          </div>
        </div>
        <div class="ver-tools">
          <select v-model="fileVersion" class="field-select sel-file-version" :disabled="filesLoading" @change="onFileVersionChange">
            <option value="">{{ t("modpack.allVersions") }}</option>
            <option v-for="v in versions" :key="v" :value="v">{{ v }}</option>
          </select>
          <div v-if="fileMaxPage > 1" class="file-page">
            <button class="page-btn" :disabled="filePage === 0 || filesLoading" @click="turnFilePage(-1)">
              {{ t("modpack.prevPage") }}
            </button>
            <span class="file-page-num">{{ filePage + 1 }} / {{ fileMaxPage }} · {{ t("modpack.totalItems", { n: fileTotal }) }}</span>
            <button class="page-btn" :disabled="filePage >= fileMaxPage - 1 || filesLoading" @click="turnFilePage(1)">
              {{ t("modpack.nextPage") }}
            </button>
          </div>
        </div>
        <div class="ver-files" :class="{ 'is-loading-dim': filesLoading }">
          <!-- 首次加载用骨架；翻页时保留旧内容并压暗（结果到了才整体替换） -->
          <template v-if="filesLoading && pageFiles.length === 0">
            <div v-for="n in 4" :key="n" class="sk-row compact">
              <div class="sk-lines">
                <div class="sk sk-line w70"></div>
                <div class="sk sk-line w45"></div>
              </div>
            </div>
          </template>
          <div v-else-if="pageFiles.length === 0" class="empty-tip">{{ t("modpack.noVersions") }}</div>
          <template v-else>
            <div v-for="file in pageFiles" :key="file.source.fid" class="file-row">
              <div class="file-info">
                <span class="file-name">
                  {{ file.name }}
                  <span v-if="fileInstalled(file)" class="pack-badge">{{ t("modpack.installed") }}</span>
                  <span v-else-if="fileRunning(file)" class="pack-badge busy">{{ t("modpack.downloading") }}</span>
                </span>
                <span class="file-meta">
                  {{ t("modpack.fileMeta", {
                    n: file.download.toLocaleString(),
                    date: formatDate(file.time),
                    size: formatSize(file.size),
                  }) }}
                </span>
              </div>
              <button class="install-btn" @click="download(file)">
                {{ t("addResource.download") }}
              </button>
            </div>
          </template>
        </div>
      </BaseModal>

      <!-- 存档选择弹窗（数据包下载前） -->
      <BaseModal v-if="savePickFile" :width="420" below-titlebar @close="savePickFile = null">
        <div class="save-head">{{ t("addResource.chooseSave") }}</div>
        <div class="save-hint">{{ t("addResource.chooseSaveHint") }}</div>
        <div class="save-list">
          <div v-if="savesLoading" class="empty-tip">{{ t("modpack.loading") }}</div>
          <div v-else-if="saves.length === 0" class="empty-tip">{{ t("resource.empty") }}</div>
          <label
            v-for="save in saves"
            :key="save.dir"
            class="save-row"
            :class="{ on: pickedSave === save.dir }"
          >
            <input v-model="pickedSave" type="radio" name="save-pick" :value="save.dir" />
            <span class="save-name">{{ save.name }}</span>
          </label>
        </div>
        <div class="save-actions">
          <button class="search-btn" :disabled="!pickedSave" @click="confirmSave">
            {{ t("modpack.install") }}
          </button>
        </div>
      </BaseModal>
    </div>
  </WindowFrame>
</template>

<style scoped>
.res-mode {
  display: flex;
  flex-direction: column;
  gap: 10px;
  height: 100%;
}

/* 下载进度条也是这一列里的卡片，同样回缩 8px（与下面筛选卡、资源卡对齐） */
.res-mode > :deep(.res-bar) {
  margin: 0 8px;
}

/* 左右各 8px 与下面列表里那圈内边距对齐 —— 列表要让出这 8px 给卡片悬停阴影 */
.res-top {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin: 0 8px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 10px;
}

.res-filters {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

/* 类型一栏（5 项）可能比窗口宽：允许换行 */
.res-filters > :deep(.seg-tabs) {
  flex-wrap: wrap;
}

.sel-filter {
  flex: 1 1 120px;
  min-width: 120px;
  /* 纵向内边距别改小：下拉的高度由 --field-h（42）兜底，比"行高 + 内边距"多出来的
     那几px在 <select> 里会全沉到底部，文字看着偏上（不变量见 forms.css） */
  padding: 10px;
  font-size: 13px;
}

/* field-select 的自定义箭头留位 */
select.sel-filter,
select.sel-file-version {
  padding-right: 26px;
}

/* 存档类型：源锁定为 CurseForge 的占位块（与页签同高） */
.res-source-lock {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  font-size: 13px;
}

.res-source-name {
  color: var(--text);
  font-weight: 600;
}

.res-source-hint {
  color: var(--text-dim);
  font-size: 12px;
}

.res-search {
  display: flex;
  gap: 8px;
}

.search-input {
  flex: 1;
  min-width: 0;
}

.search-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  padding: 0 18px;
  border: none;
  border-radius: 10px;
  background: var(--accent-grad);
  color: #fff;
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
}

.search-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

/* 目标实例一行（同样回缩 8px，与上下两张卡对齐） */
.res-instance {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0 8px;
  padding: 8px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  font-size: 13px;
  color: var(--text-dim);
}

.res-instance strong {
  color: var(--text);
}

.res-no-instance {
  color: var(--red);
}

.res-list {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  /* 左右与下方留 8px：滚动容器会把自己的溢出裁掉，不留余量卡片悬停阴影会被齐边切掉 */
  padding: 0 8px 8px;
}

.empty-tip {
  margin: 24px 0;
  color: var(--text-dim);
  text-align: center;
  font-size: 13px;
}

.pack-item {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
  transition: box-shadow 0.15s ease;
}

/* 悬停：主题阴影（与主页卡片 .entry-card 同一套 token，也与整合包窗口一致）。
   刻意不换颜色：底色换成 --bg-hover 在浅色主题下白卡会发灰，
   描边染成强调色则跟着主题色走（红 / 粉主题下很跳）。
   也不上浮：列表容器是滚动容器，会把自己的溢出裁掉，上浮会把卡片顶边切掉 */
.pack-item:hover {
  box-shadow: var(--shadow-md);
}

.pack-main {
  display: flex;
  gap: 12px;
  padding: 10px 12px;
  cursor: pointer;
}

.pack-icon {
  width: 52px;
  height: 52px;
  border-radius: 10px;
  object-fit: cover;
  flex-shrink: 0;
}

.pack-icon-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-hover);
  color: var(--text-dim);
  font-size: 22px;
  font-weight: 600;
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
  font-size: 14px;
  font-weight: 600;
  color: var(--text);
}

.pack-author {
  font-size: 12px;
  font-weight: 400;
  color: var(--text-dim);
}

.pack-badge {
  padding: 1px 8px;
  border-radius: 999px;
  background: var(--green);
  color: #fff;
  font-size: 11px;
  font-weight: 400;
}

.pack-badge.busy {
  background: var(--red);
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
  gap: 6px;
  flex-wrap: wrap;
}

.pack-tag {
  padding: 1px 8px;
  border-radius: 999px;
  background: var(--bg-hover);
  color: var(--text-dim);
  font-size: 11px;
}

.pack-meta {
  font-size: 11px;
  color: var(--text-dim);
}

.res-page {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding-bottom: 2px;
}

.page-btn {
  height: 28px;
  padding: 0 14px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-card);
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  white-space: nowrap;
}

.page-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.page-num {
  font-size: 13px;
  color: var(--text-dim);
  /* 末尾带「共 N 个」，窄了会折行 */
  white-space: nowrap;
}

/* 搜索按钮里的转圈（按钮是渐变底，圈用白色） */
.btn-spinner {
  width: 13px;
  height: 13px;
  border: 2px solid rgb(255 255 255 / 35%);
  border-top-color: #fff;
  border-radius: 50%;
  animation: res-spin 0.8s linear infinite;
  flex-shrink: 0;
}

@keyframes res-spin {
  to {
    transform: rotate(360deg);
  }
}

/* 结果区上方的细进度条与骨架行：样式在 styles/skeleton.css（跨窗口共用） */

/* 版本选择弹窗 */
.ver-head {
  display: flex;
  gap: 12px;
  align-items: center;
  margin-bottom: 12px;
}

.ver-icon {
  width: 48px;
  height: 48px;
  border-radius: 10px;
  object-fit: cover;
  flex-shrink: 0;
}

.ver-icon-fallback {
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--bg-hover);
  color: var(--text-dim);
  font-size: 20px;
  font-weight: 600;
}

.ver-title {
  flex: 1;
  min-width: 0;
}

.ver-name {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}

.ver-summary {
  font-size: 12px;
  color: var(--text-dim);
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.ver-tools {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-bottom: 10px;
}

/* 版本筛选下拉：比列表页的筛选项小一号。
   高度要跟着一起缩小：--field-h(42) 兜底时多出来的空间在 <select> 里会全沉到底部
   （不变量见 forms.css：高度 = 行高 20 + 上下内边距 6×2 + 边框 1×2 = 34） */
.sel-file-version {
  height: 34px;
  min-height: 0;
  padding: 6px 10px;
  font-size: 13px;
  /* 选中的版本名可能很长：不让它在这条定高下拉里折成两行 */
  white-space: nowrap;
}

.file-page {
  display: flex;
  align-items: center;
  gap: 8px;
  /* 不参与收缩：被压窄的话里面的「1 / 2 · 共 43 个」会折成两行 */
  flex-shrink: 0;
}

.file-page-num {
  font-size: 12px;
  color: var(--text-dim);
  white-space: nowrap;
}

.ver-files {
  max-height: 320px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 10px;
}

.file-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.file-name {
  font-size: 13px;
  color: var(--text);
  display: flex;
  align-items: center;
  gap: 8px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-meta {
  font-size: 11px;
  color: var(--text-dim);
}

.install-btn {
  height: 35px;
  padding: 0 16px;
  border: none;
  border-radius: 8px;
  background: var(--accent-grad);
  color: #fff;
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  flex-shrink: 0;
}

/* 存档选择弹窗 */
.save-head {
  font-size: 15px;
  font-weight: 600;
  color: var(--text);
}

.save-hint {
  margin: 4px 0 12px;
  font-size: 12px;
  color: var(--text-dim);
}

.save-list {
  max-height: 280px;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 12px;
}

.save-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 10px;
  cursor: pointer;
}

.save-row.on {
  border-color: var(--text);
  background: var(--bg-hover);
}

.save-name {
  font-size: 13px;
  color: var(--text);
}

.save-actions {
  display: flex;
  justify-content: flex-end;
}
</style>
