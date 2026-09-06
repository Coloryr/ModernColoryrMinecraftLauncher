<script setup lang="ts">
// 添加实例窗口（独立窗口）
// 模式：从头新建 / 导入压缩包 / 添加文件夹 / 在线实例
import { computed, onMounted, onUnmounted, ref, watch, type Ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import NewMode from "./modes/NewMode.vue";
import ArchiveMode from "./modes/ArchiveMode.vue";
import FolderMode from "./modes/FolderMode.vue";
import OnlineMode from "./modes/OnlineMode.vue";
import { buildTree, collectDirKeys, collectFileKeys, type FileNode } from "../../lib/fileTree";
import { api, onCloseBlocked, onAddLoaderProgress, onAddNameConflict, answerNameConflict } from "../../lib/api";
import { showToast } from "../../lib/toast";
import { t } from "../../lib/i18n";
import { isTauri } from "../windowManager";
import { invoke } from "@tauri-apps/api/core";
import { AddListDir } from "../../lib/invokes";
import type { VersionInfo } from "../../lib/types";

const emit = defineEmits<{ (e: "close"): void }>();

// ================= 模式 =================

type AddMode = "new" | "archive" | "folder" | "online";
const ADD_MODES: Array<{ id: AddMode; labelKey: string; icon: string }> = [
  { id: "new", labelKey: "add.modeNew", icon: "cube" },
  { id: "archive", labelKey: "add.modeArchive", icon: "box" },
  { id: "folder", labelKey: "add.modeFolder", icon: "folder" },
  { id: "online", labelKey: "add.modeOnline", icon: "globe" },
];
const addMode = ref<AddMode>("new");

// ================= 通用字段 =================

const newName = ref("");
const newVersion = ref("");
const addGroup = ref("");
const creating = ref(false);
const addError = ref("");

// ================= 分组（可输入，弹出可选，允许自定义） =================

const groups = ref<string[]>([]);
const groupOpen = ref(false);
const groupQuery = computed(() =>
  groups.value.filter((g) =>
    g.toLowerCase().includes(addGroup.value.trim().toLowerCase()),
  ),
);

function pickGroup(name: string) {
  addGroup.value = name;
  groupOpen.value = false;
}

// ================= 模式一：从头新建（表单见 modes/NewMode.vue） =================

const versions = ref<VersionInfo[]>([]);
/** 版本类型列表（mcml-core 提供独立 ID，显示名走 i18n） */
const versionTypes = ref<string[]>([]);
/** 版本类型（可多选，mcml-core 独立 ID；空 = 不过滤） */
const verTypes = ref<string[]>(["release"]);

const filteredVersions = computed(() => {
  const sel = verTypes.value;
  return versions.value
    .filter((v) => sel.length === 0 || sel.includes(v.versionType))
    .sort((a, b) => compareVersion(b.id, a.id));
});

/** 加载器 ID 列表（选中版本后按支持情况从 mcml-core 查询，显示名走 i18n） */
const loaders = ref<string[]>([]);
const addLoader = ref("normal");
const addLoaderVer = ref("");
const loaderPath = ref("");

/** 压缩包类型 ID 列表（mcml-core 提供独立 ID，显示名走 i18n） */
const packTypes = ref<string[]>([]);

/** 拉取下拉数据源：版本类型 / 压缩包类型（独立 ID） */
async function fetchOptions() {
  try {
    [versionTypes.value, packTypes.value] = await Promise.all([
      api.addGetVersionTypes(),
      api.addGetPackTypes(),
    ]);
  } catch {
    // 核心未加载完时可能失败，留空，需要时可重开窗口
  }
}

/** 版本列表刷新中（刷新按钮转圈并禁用） */
const verLoading = ref(false);

/** 强制刷新版本列表（清后端缓存重新拉取）；不改变已选版本 */
async function refreshVersions() {
  verLoading.value = true;
  try {
    versions.value = await api.refreshVersions();
  } catch {
    // 保留旧列表
  } finally {
    verLoading.value = false;
  }
}

/** 已查询过的版本 → 支持的加载器列表缓存（每个版本只查询一次） */
const supportLoadersCache = new Map<string, string[]>();
/** 支持列表查询中（查询期间加载器下拉禁用） */
const loaderLoading = ref(false);
/** 当前正在查询的版本（防止同一版本并发重复查询） */
let loaderQuerying = "";
/** 加载器版本列表拉取中 */
const loaderVerLoading = ref(false);

/** 支持列表查询进度弹窗（step / total） */
const showLoaderProgress = ref(false);
const loaderProgressStep = ref(0);
const loaderProgressTotal = ref(0);

/** 查询当前版本支持的加载器（无选中版本时清空，加载器下拉禁用） */
async function fetchSupportLoaders() {
  const mc = newVersion.value;
  if (!mc) {
    loaders.value = [];
    return;
  }
  // 该版本查询过：直接用缓存
  const cached = supportLoadersCache.get(mc);
  if (cached) {
    loaders.value = cached;
    ensureLoaderSupported();
    return;
  }
  // 该版本正在查询：不重复发起
  if (loaderQuerying === mc) {
    return;
  }
  // 查询期间清空旧列表（下拉显示"查询中"并禁用），弹出进度窗口
  loaders.value = [];
  loaderQuerying = mc;
  loaderLoading.value = true;
  loaderProgressStep.value = 0;
  showLoaderProgress.value = true;
  try {
    const list = await api.addGetSupportLoaders(mc);
    supportLoadersCache.set(mc, list);
    // 请求期间版本已变化时丢弃过期结果
    if (newVersion.value === mc) {
      loaders.value = list;
      ensureLoaderSupported();
    }
  } catch {
    // 查询失败不缓存：保留原版 + 自定义兜底，可用刷新按钮重试
    if (newVersion.value === mc) {
      loaders.value = [FALLBACK_LOADERS[0], FALLBACK_LOADERS[1]];
    }
  } finally {
    if (loaderQuerying === mc) {
      loaderQuerying = "";
      loaderLoading.value = false;
      showLoaderProgress.value = false;
    }
  }
}

/** 刷新支持的加载器：清除当前版本的缓存后重新查询 */
async function refreshSupportLoaders() {
  const mc = newVersion.value;
  if (!mc || loaderLoading.value) return;
  supportLoadersCache.delete(mc);
  await fetchSupportLoaders();
}

/** 刷新加载器版本列表（清除缓存后重新拉取） */
async function refreshLoaderVersions() {
  if (!newVersion.value || NO_VERSION_LOADERS.includes(addLoader.value)) return;
  loaderVerCache.delete(`${addLoader.value}:${newVersion.value}`);
  await fetchLoaderVersions();
}

// 查询数据期间（支持的加载器 / 加载器版本列表）开启窗口关闭保护（后端在 CloseRequested 阶段拒绝关闭）
watch([loaderLoading, loaderVerLoading], ([a, b]) => {
  api.setCloseGuard(a || b).catch(() => {});
});

/** 当前选择的加载器不被支持时回退到原版 */
function ensureLoaderSupported() {
  if (!loaders.value.includes(addLoader.value)) {
    addLoader.value = "normal";
  }
}

// 选中版本变化后重新查询支持的加载器
watch(newVersion, () => {
  fetchSupportLoaders();
});

/** 加载器版本列表（mcml-core 按加载器 + 游戏版本拉取） */
const loaderVersions = ref<string[]>([]);

/** 无版本列表的加载器 ID */
const NO_VERSION_LOADERS = ["normal", "custom"];

/** 支持列表查询失败时的兜底选项（原版 + 自定义） */
const FALLBACK_LOADERS = NO_VERSION_LOADERS;

/** 加载器版本列表缓存（`loader:mc` → 列表），刷新按钮强制重新拉取 */
const loaderVerCache = new Map<string, string[]>();

/** 按当前加载器 + 游戏版本拉取加载器版本列表，重置选中项 */
async function fetchLoaderVersions() {
  if (NO_VERSION_LOADERS.includes(addLoader.value) || !newVersion.value) {
    loaderVersions.value = [];
    addLoaderVer.value = "";
    return;
  }
  // 命中缓存：直接用（切换回旧加载器不重新请求）
  const key = `${addLoader.value}:${newVersion.value}`;
  const cached = loaderVerCache.get(key);
  if (cached) {
    loaderVersions.value = cached;
    addLoaderVer.value = cached[0] ?? "";
    return;
  }
  loaderVerLoading.value = true;
  // 立即清掉上一个加载器的列表：下拉锁定显示"获取中"，避免新旧列表串显
  loaderVersions.value = [];
  addLoaderVer.value = "";
  try {
    // 记录请求参数，返回后对比：期间选择已变化则丢弃过期结果
    const loader = addLoader.value;
    const mc = newVersion.value;
    const list = await api.addLoaderVersions(loader, mc);
    loaderVerCache.set(key, list);
    if (addLoader.value === loader && newVersion.value === mc) {
      loaderVersions.value = list;
      addLoaderVer.value = list[0] ?? "";
    }
  } catch {
    // 拉取失败（如数据源不可达）时清空并提示，可用刷新按钮重试
    loaderVersions.value = [];
    addLoaderVer.value = "";
    showToast(t("add.loaderVerFail"));
  } finally {
    loaderVerLoading.value = false;
  }
}

// 加载器或游戏版本变化时重新拉取（Forge 的镜像源、OptiFine / LiteLoader 按版本过滤）
watch([addLoader, newVersion], () => {
  fetchLoaderVersions();
});

/** NewMode 切换加载器时：更新类型（版本列表由 watch 拉取） */
function onLoaderChange(v: string) {
  addLoader.value = v;
}

// ================= 实例名自动填写 =================

/** 用户手动改过名字后不再自动填写 */
let nameEdited = false;

/** 用户在名字输入框手动输入：非空则停止自动填写，清空则恢复 */
function onNameInput(e: Event) {
  const value = (e.target as HTMLInputElement).value;
  nameEdited = value.trim().length > 0;
  newName.value = value;
}

/**
 * 自动实例名：`游戏版本-加载器-加载器版本`（如 26.2-Fabric-0.19.3）；
 * 原版只有游戏版本，自定义 / 未拉到加载器版本时省略对应段
 */
function autoName() {
  if (addMode.value !== "new" || nameEdited) return;
  const parts = [newVersion.value];
  if (newVersion.value && addLoader.value !== "normal") {
    parts.push(t(`add.loader.${addLoader.value}`));
    if (addLoaderVer.value) parts.push(addLoaderVer.value);
  }
  newName.value = parts.filter(Boolean).join("-");
}

// 版本 / 加载器 / 加载器版本变化时刷新自动名（切换加载器时会先无版本号、列表到达后补全）
watch([newVersion, addLoader, addLoaderVer], autoName);

// ================= 模式二：导入压缩包（路径框 + 文件树） =================

const addPackType = ref("curseforge");
const addArchivePath = ref("");
const archiveInput = ref<HTMLInputElement | null>(null);
const archiveTree = ref<FileNode[]>([]);
const archiveChecked = ref<Set<string>>(new Set());
const archiveExpanded = ref<Set<string>>(new Set());

function onArchivePick(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  applyArchivePath(`C:\\Users\\demo\\Downloads\\${file.name}`);
}

/** 应用压缩包路径并生成文件树（默认全部选中、全部展开） */
function applyArchivePath(path: string) {
  addArchivePath.value = path;
  archiveTree.value = buildTree(MOCK_ARCHIVE_FILES);
  archiveChecked.value = new Set(collectFileKeys(archiveTree.value));
  archiveExpanded.value = new Set(collectDirKeys(archiveTree.value));
}

/** 选择压缩包：Tauri 用系统文件对话框，浏览器回退到文件输入 */
async function pickArchive() {
  if (isTauri()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const picked = await open({
      title: t("add.archive"),
      multiple: false,
      filters: [{ name: "Modpack", extensions: ["zip", "mrpack"] }],
    });
    if (typeof picked === "string") applyArchivePath(picked);
    return;
  }
  archiveInput.value?.click();
}

function setAllArchive(on: boolean) {
  archiveChecked.value = on
    ? new Set(collectFileKeys(archiveTree.value))
    : new Set();
}

// ================= 模式三：添加文件夹（路径框 + 内容树） =================

const addFolderPath = ref("");
const folderInput = ref<HTMLInputElement | null>(null);
const folderTree = ref<FileNode[]>([]);
const folderChecked = ref<Set<string>>(new Set());
const folderExpanded = ref<Set<string>>(new Set());

function onFolderPick(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  const folder = file.webkitRelativePath.split("/")[0];
  addFolderPath.value = `C:\\Users\\demo\\.minecraft\\versions\\${folder}`;
  // 浏览器回退：用 mock 内容树（全部展开、默认不勾选）
  folderTree.value = buildTree(MOCK_FOLDER_FILES);
  folderChecked.value = new Set();
  folderExpanded.value = new Set(collectDirKeys(folderTree.value));
}

/** 选择文件夹：Tauri 用系统目录对话框，选中后列出文件夹内容树 */
async function pickFolder() {
  if (isTauri()) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const picked = await open({
      title: t("add.folder"),
      directory: true,
      multiple: false,
    });
    if (typeof picked === "string") {
      addFolderPath.value = picked;
      await loadFolderTree(picked);
    }
    return;
  }
  folderInput.value?.click();
}

/** 列出目录直接内容为树节点（目录标记 lazy，展开时再加载） */
async function listDirNodes(dirPath: string, rel: string): Promise<FileNode[]> {
  const entries = await invoke<Array<{ name: string; is_dir: boolean }>>(AddListDir, {
    path: dirPath,
  });
  return entries.map((en) => {
    const key = rel ? `${rel}/${en.name}` : en.name;
    const node: FileNode = { key, name: en.name, isDir: en.is_dir, children: [] };
    if (en.is_dir) node.lazy = true;
    return node;
  });
}

/** 加载文件夹根目录内容树 */
async function loadFolderTree(rootPath: string) {
  try {
    const nodes = await listDirNodes(rootPath, "");
    folderTree.value = nodes;
    // 默认展开顶层目录，展示两层内容；默认不勾选
    folderExpanded.value = new Set(nodes.filter((n) => n.isDir).map((n) => n.key));
    folderChecked.value = new Set();
  } catch {
    folderTree.value = [];
    addError.value = t("add.folderReadFail");
  }
}

/** 懒加载：展开目录时读取其子节点 */
async function onFolderLazyLoad(node: FileNode) {
  node.lazy = false;
  const dirPath = folderPathOf(node.key);
  try {
    node.children = await listDirNodes(dirPath, node.key);
  } catch {
    node.children = [];
  }
}

/** 相对 key → 完整路径 */
function folderPathOf(rel: string): string {
  const root = addFolderPath.value.replace(/[\\/]+$/, "");
  return `${root}\\${rel.replace(/\//g, "\\")}`;
}

function setAllFolder(on: boolean) {
  folderChecked.value = on
    ? new Set(collectFileKeys(folderTree.value))
    : new Set();
}

function onFolderToggleFile(key: string) {
  toggleTreeFile(folderChecked, key);
}
function onFolderToggleDir(n: FileNode, on: boolean) {
  toggleTreeDir(folderChecked, n, on);
}
function onFolderToggleExpand(key: string) {
  toggleTreeExpand(folderExpanded, key);
}

// ================= 文件树操作（压缩包 / 文件夹共用） =================

function toggleTreeFile(set: Ref<Set<string>>, key: string) {
  const s = new Set(set.value);
  if (s.has(key)) s.delete(key);
  else s.add(key);
  set.value = s;
}

function toggleTreeDir(set: Ref<Set<string>>, node: FileNode, on: boolean) {
  const s = new Set(set.value);
  const files = collectFileKeys(node.children);
  for (const f of files) {
    if (on) s.add(f);
    else s.delete(f);
  }
  set.value = s;
}

function toggleTreeExpand(expanded: Ref<Set<string>>, key: string) {
  const s = new Set(expanded.value);
  if (s.has(key)) s.delete(key);
  else s.add(key);
  expanded.value = s;
}

// 压缩包 / 文件夹各自的树事件包装（模板里 ref 已解包，这里包一层）
function onArchiveToggleFile(key: string) {
  toggleTreeFile(archiveChecked, key);
}
function onArchiveToggleDir(n: FileNode, on: boolean) {
  toggleTreeDir(archiveChecked, n, on);
}
function onArchiveToggleExpand(key: string) {
  toggleTreeExpand(archiveExpanded, key);
}

// ================= 模式四：在线实例 =================

const addUrl = ref("");

// ================= mock 文件列表 =================

const MOCK_ARCHIVE_FILES = [
  "manifest.json",
  "modlist.html",
  "minecraft/",
  "minecraft/mods/",
  "minecraft/mods/jei-1.20.1-15.2.0.27.jar",
  "minecraft/mods/sodium-fabric-0.5.8.jar",
  "minecraft/config/",
  "minecraft/config/jei.toml",
  "overrides/",
];
const MOCK_FOLDER_FILES = [
  "options.txt",
  "servers.dat",
  "config/",
  "config/mouse-tweaks.json",
  "mods/",
  "mods/example-mod-1.0.0.jar",
  "resourcepacks/",
  "saves/",
  "saves/新世界/level.dat",
  "logs/",
];

// ================= 版本排序 =================

function compareVersion(a: string, b: string): number {
  const pa = a.split(".").map((n) => parseInt(n, 10) || 0);
  const pb = b.split(".").map((n) => parseInt(n, 10) || 0);
  for (let i = 0; i < Math.max(pa.length, pb.length); i++) {
    const x = pa[i] ?? 0;
    const y = pb[i] ?? 0;
    if (x !== y) return x - y;
  }
  return 0;
}

// ================= 创建 =================

async function create() {
  const name = newName.value.trim();
  if (!name) {
    addError.value = t("add.nameEmpty");
    return;
  }
  const group = addGroup.value.trim() === "" ? null : addGroup.value.trim();
  creating.value = true;
  addError.value = "";
  try {
    let uuid: string;
    if (addMode.value === "new") {
      if (!newVersion.value) {
        addError.value = t("add.versionEmpty");
        creating.value = false;
        return;
      }
      uuid = await api.addCreateNew(
        name,
        newVersion.value,
        addLoader.value,
        addLoader.value === "custom" ? loaderPath.value || null : addLoaderVer.value || null,
        group,
      );
    } else if (addMode.value === "archive") {
      if (!addArchivePath.value.trim()) {
        addError.value = t("add.archiveEmpty");
        creating.value = false;
        return;
      }
      uuid = await api.addImportArchive(addArchivePath.value.trim(), addPackType.value, name, group);
    } else if (addMode.value === "folder") {
      if (!addFolderPath.value.trim()) {
        addError.value = t("add.folderEmpty");
        creating.value = false;
        return;
      }
      uuid = await api.addImportFolder(addFolderPath.value.trim(), name, group);
    } else {
      if (!addUrl.value.trim()) {
        addError.value = t("add.urlEmpty");
        creating.value = false;
        return;
      }
      uuid = await api.addImportUrl(addUrl.value.trim(), name, group);
    }
    // 通知主窗口选中新实例（后端已发 instance-change 刷新列表），然后关闭本窗口
    localStorage.setItem("mcml.addedInstance", uuid);
    emit("close");
  } catch (e) {
    addError.value = t("add.createFail", { msg: String(e) });
  } finally {
    creating.value = false;
  }
}

// ================= 实例重名确认 =================

/** 当前弹出的重名确认（kind：overwrite 覆盖 / rename 自动改名；null = 无） */
const nameConflict = ref<{ id: number; kind: string; name: string } | null>(null);

/** 答复后端创建流程（关闭弹窗视为拒绝，避免创建流程挂起） */
async function answerConflict(answer: boolean) {
  const cur = nameConflict.value;
  nameConflict.value = null;
  if (cur) {
    await answerNameConflict(cur.id, answer).catch(() => {});
  }
}

// ================= 初始化 =================

/** 版本列表未就绪（核心尚在加载 / 请求失败）时轮询重试，最长约 60s */
async function pollVersions() {
  for (let i = 0; i < 30; i++) {
    await new Promise((r) => setTimeout(r, 2000));
    if (versions.value.length) return;
    try {
      const list = await api.getVersions();
      if (list.length) {
        versions.value = list;
        return;
      }
    } catch {
      // 忽略，继续重试
    }
  }
}

onMounted(async () => {
  // 关闭被拒绝（查询数据期间后端拒关）：弹提示说明原因
  const unlisten = await onCloseBlocked(() => showToast(t("add.closeBlocked")));
  onUnmounted(unlisten);
  // 加载器支持列表查询进度（每查完一种加载器推一步）
  const unlistenProgress = await onAddLoaderProgress((e) => {
    loaderProgressStep.value = e.step;
    loaderProgressTotal.value = e.total;
  });
  onUnmounted(unlistenProgress);
  // 实例重名确认（后端创建流程暂停等待答复）
  const unlistenConflict = await onAddNameConflict((e) => {
    nameConflict.value = e;
  });
  onUnmounted(unlistenConflict);
  try {
    // 默认分组（空白键）不进下拉：输入框留空即默认分组
    groups.value = (await api.getGroups()).filter((g) => g.trim());
  } catch {
    groups.value = [];
  }
  try {
    const list = await api.getVersions();
    versions.value = list;
  } catch {
    versions.value = [];
  }
  // 默认不选中游戏版本：由用户选择后再查询支持的加载器
  await fetchOptions();
  await fetchLoaderVersions();
  if (!versions.value.length) pollVersions();
});
</script>

<template>
  <WindowFrame :title="t('add.title')" @close="$emit('close')">
    <!-- 模式切换 -->
    <div class="add-modes">
      <button
        v-for="m in ADD_MODES"
        :key="m.id"
        class="add-mode-btn"
        :class="{ active: addMode === m.id }"
        @click="addMode = m.id"
      >
        <svg v-if="m.icon === 'cube'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 3 4.5 7.5v9L12 21l7.5-4.5v-9L12 3z" />
          <path d="M4.5 7.5 12 12l7.5-4.5" />
          <path d="M12 12v9" />
        </svg>
        <svg v-else-if="m.icon === 'box'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 8v13H3V8" />
          <path d="M1 3h22v5H1z" />
          <path d="M10 12h4" />
        </svg>
        <svg v-else-if="m.icon === 'folder'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
        </svg>
        <svg v-else viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="9" />
          <path d="M3 12h18M12 3a15 15 0 0 1 0 18M12 3a15 15 0 0 0 0 18" />
        </svg>
        <span>{{ t(m.labelKey) }}</span>
      </button>
    </div>

    <!-- 实例名称 + 分组 -->
    <div class="add-card">
      <div class="add-row2">
        <div class="add-field">
          <label class="field-label">{{ t("add.name") }} <span class="req">*</span></label>
          <input :value="newName" class="field-input" :placeholder="t('add.namePlaceholder')" spellcheck="false" @input="onNameInput" />
        </div>
        <div class="add-field">
          <label class="field-label">{{ t("add.group") }}</label>
          <div class="group-combo">
            <input
              v-model="addGroup"
              class="field-input"
              :placeholder="t('add.groupPlaceholder')"
              spellcheck="false"
              @focus="groupOpen = true"
              @input="groupOpen = true"
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
      </div>
    </div>

    <!-- 模式内容（各模式组件见 modes/ 目录） -->
    <div class="add-card add-card-content">
      <NewMode
        v-if="addMode === 'new'"
        :versions="filteredVersions"
        :versions-loaded="versions.length > 0"
        :ver-loading="verLoading"
        :ver-types="verTypes"
        :version-types="versionTypes"
        :new-version="newVersion"
        :loader="addLoader"
        :loaders="loaders"
        :loader-loading="loaderLoading"
        :loader-version="addLoaderVer"
        :loader-versions="loaderVersions"
        :loader-ver-loading="loaderVerLoading"
        :no-loader-version="NO_VERSION_LOADERS.includes(addLoader)"
        :loader-path="loaderPath"
        @refresh-versions="refreshVersions"
        @refresh-loaders="refreshSupportLoaders"
        @refresh-loader-versions="refreshLoaderVersions"
        @update:ver-types="verTypes = $event"
        @update:new-version="newVersion = $event"
        @update:loader="onLoaderChange"
        @update:loader-version="addLoaderVer = $event"
        @update:loader-path="loaderPath = $event"
      />
      <ArchiveMode
        v-else-if="addMode === 'archive'"
        :path="addArchivePath"
        :tree="archiveTree"
        :checked="archiveChecked"
        :expanded="archiveExpanded"
        :pack-types="packTypes"
        :pack-type="addPackType"
        @update:path="addArchivePath = $event"
        @pick="pickArchive"
        @toggle-file="onArchiveToggleFile"
        @toggle-dir="onArchiveToggleDir"
        @toggle-expand="onArchiveToggleExpand"
        @set-all="setAllArchive"
        @update:pack-type="addPackType = $event"
      />
      <FolderMode
        v-else-if="addMode === 'folder'"
        :path="addFolderPath"
        :tree="folderTree"
        :checked="folderChecked"
        :expanded="folderExpanded"
        @update:path="addFolderPath = $event"
        @pick="pickFolder"
        @toggle-file="onFolderToggleFile"
        @toggle-dir="onFolderToggleDir"
        @toggle-expand="onFolderToggleExpand"
        @lazy-load="onFolderLazyLoad"
        @set-all="setAllFolder"
      />
      <OnlineMode v-else :url="addUrl" @update:url="addUrl = $event" />
    </div>

    <p v-if="addError" class="error-text">{{ addError }}</p>

    <div class="modal-actions">
      <BaseButton @click="$emit('close')">{{ t("add.cancel") }}</BaseButton>
      <BaseButton variant="primary" :disabled="creating" @click="create">
        {{
          creating
            ? t("add.creating")
            : t(addMode === "new" ? "add.btnNew" : addMode === "archive" ? "add.btnArchive" : addMode === "folder" ? "add.btnFolder" : "add.btnOnline")
        }}
      </BaseButton>
    </div>

    <!-- 浏览器回退的文件 / 文件夹选择器（Tauri 下走系统对话框，不使用） -->
    <input ref="archiveInput" type="file" accept=".zip,.mrpack" class="hidden-input" @change="onArchivePick" />
    <input ref="folderInput" type="file" webkitdirectory class="hidden-input" @change="onFolderPick" />

    <!-- 实例重名确认弹窗（后端创建流程暂停等待答复，关闭视为拒绝） -->
    <BaseModal
      v-if="nameConflict"
      :title="t('add.nameConflictTitle')"
      @close="answerConflict(false)"
    >
      <p class="conflict-text">
        {{
          nameConflict.kind === "overwrite"
            ? t("add.nameConflictOverwrite", { name: nameConflict.name })
            : t("add.nameConflictRename")
        }}
      </p>
      <div class="modal-actions">
        <BaseButton @click="answerConflict(false)">{{ t("add.no") }}</BaseButton>
        <BaseButton variant="primary" @click="answerConflict(true)">{{ t("add.yes") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 右上角浮动进度提示（不阻挡操作，每查完一种加载器推进一步） -->
    <Teleport to="body">
      <Transition name="load-pop">
        <div v-if="showLoaderProgress" class="load-float">
          <span class="load-float-spinner"></span>
          <span class="load-float-label">{{ t("add.loaderQuerying") }}</span>
          <div class="load-float-bar">
            <div
              class="load-float-fill"
              :style="{ width: loaderProgressTotal ? (loaderProgressStep / loaderProgressTotal) * 100 + '%' : '0%' }"
            ></div>
          </div>
          <span class="load-float-text">{{ loaderProgressStep }} / {{ loaderProgressTotal }}</span>
        </div>
      </Transition>
    </Teleport>
  </WindowFrame>
</template>

<style scoped>
/* 重名确认弹窗文案 */
.conflict-text {
  margin: 0 0 10px;
  color: var(--text);
}

/* 右上角浮动进度提示：不阻挡窗口操作 */
.load-float {
  position: fixed;
  top: 14px;
  right: 16px;
  z-index: 500;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  background: var(--bg-card);
  border: 1px solid var(--accent-border);
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  font-size: 12.5px;
  color: var(--text);
  pointer-events: none;
}

.load-float-spinner {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  animation: load-float-spin 0.8s linear infinite;
  flex-shrink: 0;
}

@keyframes load-float-spin {
  to {
    transform: rotate(360deg);
  }
}

.load-float-label {
  color: var(--text-dim);
  white-space: nowrap;
}

.load-float-bar {
  width: 90px;
  height: 6px;
  border-radius: 3px;
  background: var(--border);
  overflow: hidden;
}

.load-float-fill {
  height: 100%;
  border-radius: 3px;
  background: var(--accent);
  transition: width 0.2s;
}

.load-float-text {
  color: var(--text-dim);
  min-width: 30px;
  text-align: right;
}

.load-pop-enter-active,
.load-pop-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}

.load-pop-enter-from,
.load-pop-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

/* 模式切换：图标 + 渐变激活态 */
.add-modes {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  margin-bottom: 14px;
}

.add-mode-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 7px;
  padding: 11px 6px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
  color: var(--text-dim);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.add-mode-btn:hover {
  border-color: var(--accent-border);
  color: var(--text);
  transform: translateY(-1px);
}

.add-mode-btn.active {
  border-color: transparent;
  background: var(--accent-grad);
  color: #fff;
  font-weight: 600;
  box-shadow: 0 6px 16px var(--accent-soft);
}

/* 内容卡片 */
.add-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 16px 18px;
  margin-bottom: 14px;
}

/* 模式内容卡片：四个模式保持同一高度，切换时不跳动 */
.add-card-content {
  min-height: 180px;
}

/* 卡片顶部标签不留多余空白 */
.add-card > .field-label:first-child {
  margin-top: 0;
}

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

/* 分组组合框 */
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

/* 隐藏的文件选择器（浏览器回退） */
.hidden-input {
  display: none;
}
</style>
