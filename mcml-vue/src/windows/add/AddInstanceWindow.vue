<script setup lang="ts">
// 添加实例窗口（独立窗口）
// 模式：从头新建 / 导入压缩包 / 添加文件夹 / 在线实例
import { computed, onMounted, ref, type Ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import NewMode from "./modes/NewMode.vue";
import ArchiveMode from "./modes/ArchiveMode.vue";
import FolderMode from "./modes/FolderMode.vue";
import OnlineMode from "./modes/OnlineMode.vue";
import { buildTree, collectDirKeys, collectFileKeys, type FileNode } from "../../lib/fileTree";
import { api } from "../../lib/api-ipc";
import { t } from "../../lib/i18n";
import { isTauri } from "../windowManager";
import { invoke } from "@tauri-apps/api/core";
import type { InstanceInfo, VersionInfo } from "../../lib/types";

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

/** 加载器版本表（用于切换加载器时重置默认版本） */
const LOADER_VERSIONS: Record<string, string[]> = {
  Forge: ["47.3.0", "47.2.0", "43.4.0", "36.1.0", "14.23.5.2860"],
  Fabric: ["0.16.9", "0.16.5", "0.15.11", "0.14.25"],
  NeoForge: ["21.1.0", "21.0.167", "20.4.80-beta"],
  Quilt: ["0.27.1", "0.26.0", "0.25.0"],
  OptiFine: ["HD_U_I6", "HD_U_H6", "HD_U_G5"],
  LiteLoader: ["1.12.2-SNAPSHOT"],
};

const versions = ref<VersionInfo[]>([]);
/** 版本类型：发布 / Beta */
const verType = ref<"release" | "beta">("release");

const filteredVersions = computed(() => {
  const list = versions.value.filter((v) =>
    verType.value === "release" ? v.versionType === "release" : v.versionType !== "release",
  );
  return list.sort((a, b) => compareVersion(b.id, a.id));
});

const addLoader = ref("原版");
const addLoaderVer = ref("");
const loaderPath = ref("");

/** NewMode 切换加载器时：更新类型并重置加载器版本 */
function onLoaderChange(v: string) {
  addLoader.value = v;
  addLoaderVer.value = LOADER_VERSIONS[v]?.[0] ?? "";
}

// ================= 模式二：导入压缩包（路径框 + 文件树） =================

const PACK_TYPES = ["CurseForge", "Modrinth", "McMod", "本地"];
const addPackType = ref("CurseForge");
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
  const entries = await invoke<Array<{ name: string; is_dir: boolean }>>("add_list_dir", {
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
    let inst: InstanceInfo;
    if (addMode.value === "new") {
      if (!newVersion.value) {
        addError.value = t("add.versionEmpty");
        creating.value = false;
        return;
      }
      inst = await api.createInstance(name, newVersion.value, {
        loader: addLoader.value,
        loaderVersion: addLoader.value === "自定义" ? loaderPath.value || null : addLoaderVer.value || null,
        group,
      });
    } else if (addMode.value === "archive") {
      if (!addArchivePath.value.trim()) {
        addError.value = t("add.archiveEmpty");
        creating.value = false;
        return;
      }
      inst = await api.createInstance(name, newVersion.value || "1.21.1", {
        group,
        modpackType: addPackType.value,
        source: addArchivePath.value.trim(),
      });
    } else if (addMode.value === "folder") {
      if (!addFolderPath.value.trim()) {
        addError.value = t("add.folderEmpty");
        creating.value = false;
        return;
      }
      inst = await api.createInstance(name, "本地", {
        group,
        modpackType: "文件夹",
        source: addFolderPath.value.trim(),
      });
    } else {
      if (!addUrl.value.trim()) {
        addError.value = t("add.urlEmpty");
        creating.value = false;
        return;
      }
      inst = await api.createInstance(name, newVersion.value || "1.21.1", {
        group,
        source: addUrl.value.trim(),
      });
    }
    // 通知主窗口选中新实例，然后关闭本窗口
    localStorage.setItem("mcml.addedInstance", inst.uuid);
    emit("close");
  } catch (e) {
    addError.value = t("add.createFail", { msg: String(e) });
  } finally {
    creating.value = false;
  }
}

// ================= 初始化 =================

onMounted(async () => {
  try {
    groups.value = await api.getGroups();
  } catch {
    groups.value = [];
  }
  try {
    const list = await api.getVersions();
    versions.value = list;
    newVersion.value = filteredVersions.value[0]?.id ?? "";
  } catch {
    versions.value = [];
  }
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
          <input v-model="newName" class="field-input" :placeholder="t('add.namePlaceholder')" spellcheck="false" />
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
        :ver-type="verType"
        :new-version="newVersion"
        :loader="addLoader"
        :loader-version="addLoaderVer"
        :loader-path="loaderPath"
        @update:ver-type="verType = $event"
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
        :pack-types="PACK_TYPES"
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
  </WindowFrame>
</template>

<style scoped>
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
