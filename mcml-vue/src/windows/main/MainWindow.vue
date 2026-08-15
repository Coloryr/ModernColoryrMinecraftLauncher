<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import {
  api,
  onGameExit,
  onGameLog,
  onInstanceChange,
  onLaunchError,
  onLaunchState,
} from "../../lib/api";
import { t } from "../../lib/i18n";
import { theme, toggleTheme } from "../../lib/theme";
import { newsImage } from "../../lib/newsBanner";
import { openWindow } from "../windowManager";
import {
  sidebarCollapsed,
  sidebarSide,
  setSidebarCollapsed,
} from "../../lib/settings";
import type {
  Account,
  InstanceArgs,
  InstanceInfo,
  JavaInfo,
  NewsItem,
  VersionInfo,
} from "../../lib/types";
import InstanceIcon from "../../components/InstanceIcon.vue";
import InstanceSelect from "../../components/InstanceSelect.vue";
import InstanceMetaPanel from "../../components/InstanceMetaPanel.vue";
import LaunchArgsPanel from "../../components/LaunchArgsPanel.vue";
import AccountSelector from "../../components/AccountSelector.vue";
import HomePage from "../../components/HomePage.vue";
import CustomExecPanel from "../../components/CustomExecPanel.vue";
import ProxyPanel from "../../components/ProxyPanel.vue";
import LaunchScreen from "../../components/LaunchScreen.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import NumberStepper from "../../components/ui/NumberStepper.vue";

// ================= 基础状态 =================

/** 初始化是否失败（失败则显示引导页） */
const bootFailed = ref(false);
const initError = ref("");
const initLoading = ref(false);
import { closeSplash, splashVisible } from "../../lib/splash";
const localDir = ref(localStorage.getItem("mcml.localDir") ?? "");
const playerName = ref(localStorage.getItem("mcml.playerName") ?? "Player");

// ================= 数据 =================

const instances = ref<InstanceInfo[]>([]);
const javas = ref<JavaInfo[]>([]);
const selected = ref<InstanceInfo | null>(null);

// ================= 实例搜索 =================

const searchText = ref("");
const searching = computed(() => searchText.value.trim().length > 0);

function matchInst(inst: InstanceInfo, q: string): boolean {
  return [
    inst.name,
    inst.version,
    inst.versionType ?? "",
    inst.loader,
    inst.loaderVersion ?? "",
    inst.group ?? "",
  ].some((s) => s.toLowerCase().includes(q));
}

const filteredInstances = computed(() => {
  const q = searchText.value.trim().toLowerCase();
  if (!q) return instances.value;
  return instances.value.filter((i) => matchInst(i, q));
});

const filteredGroups = computed(() => {
  const q = searchText.value.trim().toLowerCase();
  if (!q) return groups.value;
  return groups.value
    .map((g) => ({
      name: g.name,
      items: g.name.toLowerCase().includes(q)
        ? g.items
        : g.items.filter((i) => matchInst(i, q)),
    }))
    .filter((g) => g.items.length > 0);
});

function select(inst: InstanceInfo) {
  selected.value = inst;
  newsActive.value = false;
  // 分组模式下展开所在分组
  if (mode.value === "group") {
    const key = inst.group || t("group.default");
    if (collapsedGroups.value[key]) {
      collapsedGroups.value = { ...collapsedGroups.value, [key]: false };
    }
  }
}

// ================= 游戏列表模式 =================

type ViewMode = "group" | "grid" | "list";
const mode = ref<ViewMode>("group");
const MODE_OPTIONS = computed(() => [
  { value: "group", label: t("mode.group"), icon: "folder" },
  { value: "grid", label: t("mode.grid"), icon: "grid" },
  { value: "list", label: t("mode.list"), icon: "list" },
]);

// 手动添加的空分组（来自 api.getGroups）
const extraGroups = ref<string[]>([]);

const groups = computed(() => {
  const map = new Map<string, InstanceInfo[]>();
  for (const inst of instances.value) {
    const key = inst.group || t("group.default");
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(inst);
  }
  for (const g of extraGroups.value) {
    if (!map.has(g)) map.set(g, []);
  }
  return [...map.entries()].map(([name, items]) => ({ name, items }));
});

// 分组收缩状态
const collapsedGroups = ref<Record<string, boolean>>({});

function isCollapsed(name: string): boolean {
  return collapsedGroups.value[name] ?? false;
}

function toggleGroup(name: string) {
  collapsedGroups.value = { ...collapsedGroups.value, [name]: !isCollapsed(name) };
}

/** 默认只展开选中实例所在的分组 */
function collapseToSelected() {
  const selGroup = selected.value?.group || t("group.default");
  const map: Record<string, boolean> = {};
  for (const g of groups.value) {
    map[g.name] = g.name !== selGroup;
  }
  collapsedGroups.value = map;
}

/** 启动时：选中上次启动的实例并展开其分组（无记录则展开默认分组） */
function initSelection() {
  const lastUuid = localStorage.getItem("mcml.lastInstance");
  const lastInst = lastUuid
    ? instances.value.find((i) => i.uuid === lastUuid)
    : undefined;

  if (lastInst) {
    selected.value = lastInst;
    const key = lastInst.group || t("group.default");
    const map: Record<string, boolean> = {};
    for (const g of groups.value) {
      map[g.name] = g.name !== key;
    }
    collapsedGroups.value = map;
  } else {
    collapseToSelected();
  }
}

// ================= 启动器主页（默认打开） =================

const newsActive = ref(true);

/** 打开 / 关闭启动器主页（保留选中实例） */
function toggleNews() {
  newsActive.value = !newsActive.value;
}

// 切换选中实例后自动滚动到详情顶部
const detailEl = ref<HTMLElement | null>(null);

watch(selected, () => {
  nextTick(() => {
    if (detailEl.value) detailEl.value.scrollTop = 0;
  });
});

// 上次启动的实例
const lastInstance = computed<InstanceInfo | null>(() => {
  const uuid = localStorage.getItem("mcml.lastInstance");
  if (!uuid) return null;
  return instances.value.find((i) => i.uuid === uuid) ?? null;
});

function quickLaunch() {
  const inst = lastInstance.value;
  if (inst) {
    select(inst);
    launch();
  }
}

const news = ref<NewsItem[]>([
  {
    id: 1,
    title: "Minecraft 1.21.5 正式版发布，全新装饰方块上线",
    date: "2026-04-15",
    tag: "更新",
    image: newsImage(1, "#3f8cff", "#7c5cff"),
  },
  {
    id: 2,
    title: "夏季更新预览：新增生物群系与结构",
    date: "2026-04-08",
    tag: "预览",
    image: newsImage(2, "#34d399", "#22d3ee"),
  },
  {
    id: 3,
    title: "年度建筑大赛开始报名，奖品丰厚",
    date: "2026-03-30",
    tag: "活动",
    image: newsImage(3, "#f59e0b", "#ef4444"),
  },
]);

// ================= 账户 =================

// 账户与当前账户来自共享存储（与账户窗口联动）
import {
  accounts as storeAccounts,
  currentAccount as storeCurrentAccount,
  setCurrentAccount,
} from "../../lib/accountStore";
const accounts = storeAccounts;
const currentAccount = storeCurrentAccount;

function onAccountChange(account: Account) {
  setCurrentAccount(account);
  playerName.value = account.name;
  localStorage.setItem("mcml.playerName", account.name);
}

// ================= 启动状态 =================

const gameActive = ref(false);
const running = ref(false);
const statusText = ref(t("launch.ready"));
const logs = ref<string[]>([]);

function stateText(state: string): string {
  const key = `state.${state}`;
  const msg = t(key);
  return msg === key ? state : msg;
}

function appendLog(line: string) {
  logs.value.push(line);
  if (logs.value.length > 3000) logs.value.splice(0, logs.value.length - 3000);
}

// ================= 启动参数（模拟） =================

const argsOpen = ref(false);
const execOpen = ref(false);
const serverOpen = ref(false);
const proxyOpen = ref(false);
const argsMap = ref<Record<string, InstanceArgs>>({});

function argsOf(uuid: string): InstanceArgs {
  if (!argsMap.value[uuid]) {
    argsMap.value[uuid] = {
      memory: 4096,
      minMemory: 512,
      fullscreen: false,
      width: 1280,
      height: 720,
      javaName: "",
      javaPath: "",
      gc: "auto",
      gcCustom: "",
      mainClass: "",
      jvmArgs: [],
      gameArgs: [],
      classPath: [],
      envVars: [],
      lang: "zh_cn",
      logEncoding: "utf8",
      preEnabled: false,
      preCmd: "",
      postEnabled: false,
      postCmd: "",
      proxyIp: "",
      proxyPort: 1080,
      proxyUser: "",
      proxyPass: "",
      serverIp: "",
      serverPort: 25565,
      joinServer: false,
    };
  }
  return argsMap.value[uuid];
}

/** 自动加入服务器信息（按实例存储） */
function onServerIp(value: string) {
  if (selected.value) argsMap.value[selected.value.uuid] = { ...argsOf(selected.value.uuid), serverIp: value };
}

function onServerPort(v: number) {
  if (selected.value) argsMap.value[selected.value.uuid] = { ...argsOf(selected.value.uuid), serverPort: v };
}

function onServerJoin(checked: boolean) {
  if (selected.value) argsMap.value[selected.value.uuid] = { ...argsOf(selected.value.uuid), joinServer: checked };
}

// 累计游戏时间（模拟数据）
const PLAY_HOURS: Record<string, number> = {
  "11111111-1111-4111-8111-111111111111": 18.5,
  "22222222-2222-4222-8222-222222222222": 6.8,
  "33333333-3333-4333-8333-333333333333": 41.2,
  "44444444-4444-4444-8444-444444444444": 27.4,
  "55555555-5555-4555-8555-555555555555": 3.1,
};

function playHoursOf(uuid: string): number {
  return PLAY_HOURS[uuid] ?? 0;
}

function updateArgs(v: InstanceArgs) {
  if (selected.value) argsMap.value[selected.value.uuid] = v;
}

// ================= 实例操作 =================

type ActionId =
  | "addResource"
  | "manageResource"
  | "export"
  | "openFolder"
  | "viewLog"
  | "editConfig"
  | "genOnline"
  | "genInfo"
  | "rename"
  | "delete";

const ACTION_LABELS: Record<ActionId, string> = {
  addResource: "actions.addResource",
  manageResource: "actions.manageResource",
  export: "actions.export",
  openFolder: "actions.openFolder",
  viewLog: "actions.viewLog",
  editConfig: "actions.editConfig",
  genOnline: "actions.genOnline",
  genInfo: "actions.genInfo",
  rename: "actions.rename",
  delete: "actions.delete",
};

// ================= 二级菜单操作（导出 / 生成在线实例 / 生成实例信息） =================

const openMenu = ref<string | null>(null);

const menuActions: Array<{
  id: ActionId;
  labelKey: string;
  options: Array<{ id: string; labelKey: string }>;
}> = [
  {
    id: "export",
    labelKey: "actions.export",
    options: [
      { id: "pack", labelKey: "actions.exportPack" },
      { id: "zip", labelKey: "actions.exportZip" },
      { id: "mmc", labelKey: "actions.exportMmc" },
    ],
  },
  {
    id: "genOnline",
    labelKey: "actions.genOnline",
    options: [
      { id: "share", labelKey: "actions.genShare" },
      { id: "link", labelKey: "actions.genLink" },
    ],
  },
  {
    id: "genInfo",
    labelKey: "actions.genInfo",
    options: [
      { id: "json", labelKey: "actions.genInfoJson" },
      { id: "text", labelKey: "actions.genInfoText" },
    ],
  },
];

function toggleMenu(id: string) {
  openMenu.value = openMenu.value === id ? null : id;
}

function onMenuPick(opt: { labelKey: string }) {
  openMenu.value = null;
  showToast(t("actions.wip", { name: t(opt.labelKey) }));
}

// ================= 元信息（版本 / 加载器 / 整合包 / 语言，合并进实例设置） =================

const LOADERS = ["原版", "Forge", "Fabric", "Quilt", "NeoForge", "OptiFine", "LiteLoader", "自定义"];

async function onMetaUpdate(patch: Partial<InstanceInfo>) {
  if (!selected.value) return;
  await api.updateInstance(selected.value.uuid, patch);
  await loadInstances();
}

// ================= 分组拖拽移动 =================

const dragUuid = ref<string | null>(null);
const dropGroup = ref<string | null>(null);

function onDragStart(inst: InstanceInfo, e: DragEvent) {
  dragUuid.value = inst.uuid;
  if (e.dataTransfer) {
    e.dataTransfer.effectAllowed = "move";
    e.dataTransfer.setData("text/plain", inst.uuid);
  }
}

function onDragEnd() {
  dragUuid.value = null;
  dropGroup.value = null;
}

function onDragOverGroup(groupName: string) {
  dropGroup.value = groupName;
}

function onDropGroup(groupName: string) {
  const uuid = dragUuid.value;
  dropGroup.value = null;
  dragUuid.value = null;
  if (!uuid) return;
  const inst = instances.value.find((i) => i.uuid === uuid);
  if (!inst) return;
  const target = groupName === t("group.default") ? null : groupName;
  if (inst.group === target) return;
  api.updateInstance(uuid, { group: target }).then(() => {
    loadInstances();
    // 展开目标分组
    const key = target || t("group.default");
    collapsedGroups.value = { ...collapsedGroups.value, [key]: false };
  });
}

// 轻提示
const toast = ref("");
let toastTimer: number | undefined;

function showToast(msg: string) {
  toast.value = msg;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.value = "";
  }, 2200);
}

// 重命名 / 删除
const showRename = ref(false);
const renameName = ref("");
const renameBusy = ref(false);
const showDelete = ref(false);
const deleteBusy = ref(false);

function onAction(id: ActionId) {
  if (!selected.value) return;
  switch (id) {
    case "rename":
      renameName.value = selected.value.name;
      showRename.value = true;
      break;
    case "delete":
      showDelete.value = true;
      break;
    case "manageResource":
      // 跳转到资源管理窗口（记录当前实例）
      localStorage.setItem("mcml.activeInstance", selected.value.uuid);
      openWindow("resource");
      break;
    default:
      showToast(t("actions.wip", { name: t(ACTION_LABELS[id]) }));
  }
}

async function doRename() {
  if (!selected.value) return;
  renameBusy.value = true;
  const ok = await api.renameInstance(selected.value.uuid, renameName.value);
  renameBusy.value = false;
  if (ok) {
    showRename.value = false;
    await loadInstances();
    showToast(t("actions.rename"));
  }
}

async function doDelete() {
  if (!selected.value) return;
  deleteBusy.value = true;
  const ok = await api.deleteInstance(selected.value.uuid);
  deleteBusy.value = false;
  showDelete.value = false;
  if (ok) {
    selected.value = null;
    await Promise.all([loadInstances(), loadGroups()]);
  }
}

// ================= 启动器功能入口（顶部栏） =================

const features: Array<{ id: "settings" | "stats" | "skin" | "help"; icon: string }> = [
  { id: "settings", icon: "gear" },
  { id: "stats", icon: "chart" },
  { id: "skin", icon: "user" },
  { id: "help", icon: "book" },
];

// ================= 添加实例 / 添加分组 弹窗 =================

const showAdd = ref(false);
const versions = ref<VersionInfo[]>([]);
const newName = ref("");
const newVersion = ref("");
const creating = ref(false);
const addError = ref("");

const showAddGroup = ref(false);
const groupName = ref("");
const groupError = ref("");
const groupAdding = ref(false);

// ================= 服务器 MOTD 悬浮卡片 =================

const motdNow = ref(128);
const motdPing = ref(32);
const motdRefreshing = ref(false);

function refreshMotd() {
  if (motdRefreshing.value) return;
  motdRefreshing.value = true;
  setTimeout(() => {
    motdNow.value = 118 + Math.floor(Math.random() * 30);
    motdPing.value = 18 + Math.floor(Math.random() * 60);
    motdRefreshing.value = false;
  }, 800);
}

// ================= 事件订阅 =================

const unlistens: Array<() => void> = [];

async function subscribeEvents() {
  const fns = await Promise.all([
    onGameLog((e) => {
      if (e.uuid !== selected.value?.uuid) return;
      if (e.clear) {
        logs.value = [];
        return;
      }
      appendLog(e.text);
    }),
    onLaunchState((e) => {
      if (e.uuid !== selected.value?.uuid) return;
      statusText.value = stateText(e.state);
    }),
    onGameExit((e) => {
      if (e.uuid !== selected.value?.uuid) return;
      gameActive.value = false;
      running.value = false;
      statusText.value =
        e.code === 0 ? t("launch.exited") : t("launch.exitedCode", { code: e.code });
      appendLog(
        e.code === 0
          ? t("launch.processExited")
          : t("launch.processExitedCode", { code: e.code }),
      );
      const inst = instances.value.find((i) => i.uuid === e.uuid);
      if (inst) inst.running = false;
    }),
    onLaunchError((e) => {
      if (e.uuid && e.uuid !== selected.value?.uuid) return;
      gameActive.value = false;
      running.value = false;
      statusText.value = t("launch.failed");
      appendLog(t("launch.error", { msg: e.message }));
      if (e.uuid) {
        const inst = instances.value.find((i) => i.uuid === e.uuid);
        if (inst) inst.running = false;
      }
    }),
    onInstanceChange(() => {
      loadInstances();
      loadGroups();
    }),
  ]);
  unlistens.push(...fns);
}

// ================= 初始化 =================

async function doInit() {
  initLoading.value = true;
  initError.value = "";
  try {
    await api.initCore(localDir.value.trim() || null, playerName.value);
    localStorage.setItem("mcml.localDir", localDir.value);
    localStorage.setItem("mcml.playerName", playerName.value);
    bootFailed.value = false;
    await Promise.all([loadInstances(), loadGroups(), loadJava(), loadVersions()]);
    // 初始化完成，关闭启动画面
    closeSplash();
  } catch (e) {
    initError.value = String(e);
  } finally {
    initLoading.value = false;
  }
}

// ================= 数据加载 =================

async function loadInstances() {
  instances.value = await api.getInstances();
  // 保留当前选中；不自动选中实例（默认停在启动器主页）
  if (selected.value) {
    const now = instances.value.find((i) => i.uuid === selected.value?.uuid);
    if (now) selected.value = now;
    else selected.value = null;
  }
}

async function loadGroups() {
  try {
    extraGroups.value = await api.getGroups();
  } catch {
    extraGroups.value = [];
  }
}

async function loadJava() {
  try {
    javas.value = await api.getJavaList();
    // 给每个实例的 Java 参数补默认值
    for (const inst of instances.value) {
      const args = argsMap.value[inst.uuid];
      if (args && !args.javaName && javas.value.length) {
        args.javaName = javas.value[0].name;
      }
    }
  } catch {
    javas.value = [];
  }
}

async function loadVersions() {
  try {
    versions.value = await api.getVersions();
  } catch {
    versions.value = [];
  }
}

// ================= 启动 / 停止 =================

async function launch() {
  if (!selected.value) return;
  const uuid = selected.value.uuid;
  localStorage.setItem("mcml.lastInstance", uuid);
  gameActive.value = true;
  running.value = true;
  statusText.value = t("launch.launching");
  logs.value = [];
  try {
    await api.launchGame(uuid, playerName.value);
  } catch (e) {
    gameActive.value = false;
    running.value = false;
    statusText.value = t("launch.failed");
    appendLog(t("launch.error", { msg: String(e) }));
  }
}

async function stop() {
  if (!selected.value) return;
  try {
    await api.stopGame(selected.value.uuid);
  } catch (e) {
    appendLog(t("launch.error", { msg: String(e) }));
  }
}

function onPickInstance(uuid: string) {
  const inst = instances.value.find((i) => i.uuid === uuid);
  if (inst) select(inst);
}

// ================= 添加实例 =================

async function openAdd() {
  showAdd.value = true;
  addError.value = "";
  versions.value = [];
  newName.value = selected.value ? `${selected.value.name} 副本` : "";
  newVersion.value = "";
  try {
    const list = await api.getVersions();
    versions.value = sortVersions(list);
    newVersion.value = versions.value[0]?.id ?? "";
  } catch (e) {
    addError.value = t("add.versionFail", { msg: String(e) });
  }
}

function sortVersions(list: VersionInfo[]): VersionInfo[] {
  const rank = (v: string) => (v === "release" ? 0 : 1);
  return [...list].sort((a, b) => {
    const r = rank(a.versionType) - rank(b.versionType);
    if (r !== 0) return r;
    return compareVersion(b.id, a.id);
  });
}

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

async function create() {
  if (!newName.value.trim()) {
    addError.value = t("add.nameEmpty");
    return;
  }
  if (!newVersion.value) {
    addError.value = t("add.versionEmpty");
    return;
  }
  creating.value = true;
  addError.value = "";
  try {
    const inst = await api.createInstance(newName.value.trim(), newVersion.value);
    showAdd.value = false;
    await loadInstances();
    select(inst);
  } catch (e) {
    addError.value = t("add.createFail", { msg: String(e) });
  } finally {
    creating.value = false;
  }
}

// ================= 添加分组 =================

async function createGroup() {
  if (!groupName.value.trim()) {
    groupError.value = t("group.nameEmpty");
    return;
  }
  groupAdding.value = true;
  groupError.value = "";
  const ok = await api.addGroup(groupName.value.trim());
  if (!ok) {
    groupError.value = t("group.exists");
    groupAdding.value = false;
    return;
  }
  groupAdding.value = false;
  groupName.value = "";
  showAddGroup.value = false;
  await loadGroups();
}

// ================= 生命周期 =================

onMounted(async () => {
  subscribeEvents();
  initLoading.value = true;
  try {
    await api.initCore(localDir.value.trim() || null, playerName.value);
    bootFailed.value = false;
    await Promise.all([loadInstances(), loadGroups(), loadJava(), loadVersions()]);
    // 启动时选中上次启动的实例并展开其分组
    initSelection();
    // 初始化完成，关闭启动画面
    closeSplash();
  } catch {
    bootFailed.value = true;
    closeSplash();
  } finally {
    initLoading.value = false;
  }
});

onUnmounted(() => {
  unlistens.forEach((fn) => fn());
});
</script>

<template>
  <div class="main-window">
    <!-- ===== 启动画面（初始化中，由 closeSplash() 关闭） ===== -->
    <div v-if="splashVisible" class="splash">
      <div class="splash-logo">MC</div>
      <div class="splash-name">{{ t("app.name") }}</div>
      <div class="splash-spinner"></div>
      <div class="splash-text">{{ t("init.splash") }}</div>
    </div>

    <!-- ===== 初始化引导（仅初始化失败时出现） ===== -->
    <div v-else-if="bootFailed" class="setup">
      <div class="setup-card">
        <div class="setup-logo">MCML</div>
        <h1>{{ t("app.name") }}</h1>
        <p class="setup-sub">{{ t("init.title") }}</p>

        <label class="field-label">{{ t("init.dataDir") }}</label>
        <input v-model="localDir" class="field-input" :placeholder="t('init.dataDirPlaceholder')" spellcheck="false" />

        <label class="field-label">{{ t("init.playerName") }}</label>
        <input v-model="playerName" class="field-input" :placeholder="t('init.playerNamePlaceholder')" spellcheck="false" />

        <p v-if="initError" class="error-text">{{ initError }}</p>

        <BaseButton variant="primary" size="lg" block :disabled="initLoading" @click="doInit">
          {{ initLoading ? t("init.initializing") : t("init.button") }}
        </BaseButton>
      </div>
    </div>

    <!-- ===== 主界面 ===== -->
    <template v-else>
      <!-- 顶部栏 -->
      <header class="topbar">
        <div class="brand">
          <div class="brand-logo">MC</div>
          <div class="brand-text">
            <div class="brand-name">{{ t("app.name") }}</div>
            <div class="brand-sub">{{ t("app.sub") }}</div>
          </div>
        </div>

        <div class="topbar-right">
          <!-- 启动器主页（切换按钮） -->
          <button
            class="topbar-icon-btn"
            :class="{ pressed: newsActive }"
            :title="t('home.entry')"
            @click="toggleNews"
          >
            <svg viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="m3 10 9-7 9 7v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V10z" />
              <path d="M9 22V12h6v10" />
            </svg>
          </button>

          <!-- 功能入口 -->
          <button
            v-for="f in features"
            :key="f.id"
            class="topbar-icon-btn"
            :title="t('features.' + f.id)"
            @click="openWindow(f.id)"
          >
            <svg v-if="f.icon === 'gear'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8">
              <circle cx="12" cy="12" r="3.2" />
              <path d="M19.4 15a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1.03 1.56V21a2 2 0 1 1-4 0v-.09a1.7 1.7 0 0 0-1.03-1.56 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.56-1.03H3a2 2 0 1 1 0-4h.09a1.7 1.7 0 0 0 1.56-1.03 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34h.01a1.7 1.7 0 0 0 1.03-1.56V3a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 1.03 1.56 1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87v.01a1.7 1.7 0 0 0 1.56 1.03H21a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.56 1.03z" />
            </svg>
            <svg v-else-if="f.icon === 'chart'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
              <path d="M18 20V10M12 20V4M6 20v-6" />
            </svg>
            <svg v-else-if="f.icon === 'user'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
              <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
              <circle cx="12" cy="7" r="4" />
            </svg>
            <svg v-else viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
              <path d="M4 19.5A2.5 2.5 0 0 1 6.5 17H20" />
              <path d="M6.5 2H20v20H6.5A2.5 2.5 0 0 1 4 19.5v-15A2.5 2.5 0 0 1 6.5 2z" />
            </svg>
          </button>

          <!-- 主题切换 -->
          <button class="topbar-icon-btn" :title="theme === 'dark' ? 'Light' : 'Dark'" @click="toggleTheme">
            <svg v-if="theme === 'dark'" viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
              <circle cx="12" cy="12" r="4.5" />
              <path d="M12 2v2.5M12 19.5V22M4.9 4.9l1.8 1.8M17.3 17.3l1.8 1.8M2 12h2.5M19.5 12H22M4.9 19.1l1.8-1.8M17.3 6.7l1.8-1.8" />
            </svg>
            <svg v-else viewBox="0 0 24 24" width="17" height="17" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
              <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" />
            </svg>
          </button>

          <AccountSelector
            :account="currentAccount"
            :accounts="accounts"
            @update:account="onAccountChange"
          />
        </div>
      </header>

      <!-- 主体 -->
      <main class="main" :class="{ 'side-right': sidebarSide === 'right' }">
        <!-- 列表模式下启动器主页整页显示 -->
        <template v-if="newsActive && mode === 'list'">
          <section class="news-page">
            <HomePage
              :items="news"
              :last-instance="lastInstance"
              @select="(inst: InstanceInfo) => select(inst)"
              @quick-launch="quickLaunch"
            />
          </section>
        </template>

        <!-- 模式：游戏分组 / 平铺 -->
        <template v-else-if="mode === 'group' || mode === 'grid'">
          <!-- 侧栏收起时的展开把手 -->
          <button
            v-if="sidebarCollapsed"
            class="sidebar-expand"
            :title="t('sidebar.expand')"
            @click="setSidebarCollapsed(false)"
          >›</button>

          <div v-if="!sidebarCollapsed" class="sidebar-backdrop" @click="setSidebarCollapsed(true)"></div>

          <aside v-if="!sidebarCollapsed" class="sidebar">
            <div class="sidebar-head">
              <SegmentedTabs
                :model-value="mode"
                :options="MODE_OPTIONS"
                @update:model-value="mode = $event as ViewMode"
              />
              <div class="sidebar-head-actions">
                <button class="icon-btn" :title="t('group.addGroup')" @click="showAddGroup = true">＋</button>
                <button class="icon-btn" :title="t('sidebar.collapse')" @click="setSidebarCollapsed(true)">‹</button>
              </div>
            </div>

            <!-- 实例搜索 -->
            <div class="search-box">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                <circle cx="11" cy="11" r="7" />
                <path d="m20 20-3.5-3.5" />
              </svg>
              <input
                v-model="searchText"
                class="search-input"
                :placeholder="t('search.placeholder')"
                spellcheck="false"
              />
              <button v-if="searchText" class="search-clear" @click="searchText = ''">✕</button>
            </div>

            <!-- 分组：可收缩，组内顶部有添加 -->
            <div v-if="mode === 'group'" class="group-list">
              <div
                v-for="g in filteredGroups"
                :key="g.name"
                class="group-block"
                :class="{ 'drop-target': dropGroup === g.name }"
                @dragover.prevent="onDragOverGroup(g.name)"
                @drop.prevent="onDropGroup(g.name)"
              >
                <div class="group-title-row">
                  <button class="group-title" :title="t('group.collapse')" @click="toggleGroup(g.name)">
                    <svg
                      class="group-chevron"
                      :class="{ collapsed: isCollapsed(g.name) }"
                      viewBox="0 0 24 24"
                      width="13"
                      height="13"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <path d="m6 9 6 6 6-6" />
                    </svg>
                    <span>{{ g.name }}</span>
                    <span class="group-count">{{ g.items.length }}</span>
                  </button>
                </div>

                <!-- 拖拽时自动展开，便于投放 -->
                <div
                  v-show="!isCollapsed(g.name) || searching || dragUuid !== null"
                  class="group-items"
                >
                  <!-- 添加实例（与实例行同尺寸，位于实例上方） -->
                  <button class="add-inst-row" @click="openAdd">
                    <span class="add-inst-icon">＋</span>
                    <span class="add-inst-text">{{ t("list.add") }}</span>
                  </button>
                  <div
                    v-for="inst in g.items"
                    :key="inst.uuid"
                    class="inst-row"
                    :class="{
                      active: selected?.uuid === inst.uuid,
                      dragging: dragUuid === inst.uuid,
                    }"
                    draggable="true"
                    @dragstart="onDragStart(inst, $event)"
                    @dragend="onDragEnd"
                    @click="select(inst)"
                  >
                    <InstanceIcon :name="inst.name" :uuid="inst.uuid" :size="38" />
                    <span v-if="inst.loader !== '原版'" class="loader-text">{{ inst.loader }}</span>
                    <span class="inst-name">{{ inst.name }}</span>
                    <span v-if="inst.running" class="run-dot" title="running"></span>
                  </div>
                </div>
              </div>
              <div v-if="groups.length === 0" class="empty-tip">{{ t("list.empty") }}</div>
              <div v-else-if="filteredGroups.length === 0" class="empty-tip">{{ t("search.empty") }}</div>
            </div>

            <!-- 平铺：首格为添加 -->
            <div v-else class="tile-list">
              <div class="tile add-tile" @click="openAdd">
                <span class="add-plus">＋</span>
                <span class="tile-name">{{ t("group.add") }}</span>
              </div>
              <div
                v-for="inst in filteredInstances"
                :key="inst.uuid"
                class="tile"
                :class="{ active: selected?.uuid === inst.uuid }"
                @click="select(inst)"
              >
                <InstanceIcon :name="inst.name" :uuid="inst.uuid" :size="44" />
                <span
                  v-if="inst.loader !== '原版'"
                  class="loader-corner loader-text"
                >{{ inst.loader }}</span>
                <span class="tile-name">{{ inst.name }}</span>
                <span v-if="inst.running" class="run-dot" title="running"></span>
              </div>
              <div v-if="instances.length === 0" class="empty-tip tile-empty">{{ t("list.empty") }}</div>
              <div v-else-if="filteredInstances.length === 0" class="empty-tip tile-empty">{{ t("search.empty") }}</div>
            </div>
          </aside>

          <!-- 右侧内容区：实例详情 / 启动器主页 -->
          <section ref="detailEl" class="detail">
            <template v-if="newsActive">
              <HomePage
                :items="news"
                :last-instance="lastInstance"
                @select="(inst: InstanceInfo) => select(inst)"
                @quick-launch="quickLaunch"
              />
            </template>

            <template v-else>
              <template v-if="selected">
                <div class="detail-top">
                  <InstanceIcon :name="selected.name" :uuid="selected.uuid" :size="84" />
                  <div class="detail-info">
                    <h2>{{ selected.name }}</h2>
                  </div>
                  <!-- 右侧：启动游戏（大）+ 添加/管理资源（小） -->
                  <div class="detail-side">
                    <BaseButton variant="primary" size="lg" class="play-btn" @click="launch">▶ {{ t("launch.play") }}</BaseButton>
                    <div class="side-row">
                      <BaseButton size="sm" variant="accent" @click="onAction('addResource')">{{ t("actions.addResource") }}</BaseButton>
                      <BaseButton size="sm" variant="accent" @click="onAction('manageResource')">{{ t("actions.manageResource") }}</BaseButton>
                    </div>
                  </div>
                </div>

                <div class="detail-meta">
                  <span>{{ t("detail.playTime", { hours: playHoursOf(selected.uuid) }) }}</span>
                  <span class="sep">·</span>
                  <span>{{ t("detail.launchCount", { count: 0 }) }}</span>
                  <!-- 小图标快捷操作（单色 SVG） -->
                  <div class="meta-actions">
                    <button class="icon-btn" :title="t('actions.openFolder')" @click="onAction('openFolder')">
                      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
                      </svg>
                    </button>
                    <button class="icon-btn" :title="t('actions.viewLog')" @click="onAction('viewLog')">
                      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8l-6-6z" />
                        <path d="M14 2v6h6M16 13H8M16 17H8M10 9H8" />
                      </svg>
                    </button>
                    <button class="icon-btn" :title="t('actions.editConfig')" @click="onAction('editConfig')">
                      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3" />
                        <path d="M1 14h6M9 8h6M17 16h6" />
                      </svg>
                    </button>
                    <button class="icon-btn" :title="t('actions.rename')" @click="onAction('rename')">
                      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M17 3a2.85 2.85 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z" />
                      </svg>
                    </button>
                    <button class="icon-btn danger" :title="t('actions.delete')" @click="onAction('delete')">
                      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6" />
                      </svg>
                    </button>
                  </div>
                </div>

                <!-- 实例操作（导出 / 生成为二级菜单） -->
                <div class="action-grid">
                  <div v-for="m in menuActions" :key="m.id" class="menu-wrap">
                    <button class="action-btn" @click="toggleMenu(m.id)">
                      {{ t(m.labelKey) }}
                      <svg
                        class="export-chevron"
                        :class="{ flip: openMenu === m.id }"
                        viewBox="0 0 24 24"
                        width="11"
                        height="11"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                      >
                        <path d="m6 9 6 6 6-6" />
                      </svg>
                    </button>
                    <div v-if="openMenu === m.id" class="menu-drop">
                      <button
                        v-for="opt in m.options"
                        :key="opt.id"
                        class="menu-item"
                        @click="onMenuPick(opt)"
                      >
                        {{ t(opt.labelKey) }}
                      </button>
                    </div>
                  </div>
                </div>

                <!-- 实例设置（直接展示：版本 / 加载器 / 整合包 / 语言 / 内存 / Java / 启动参数） -->
                <div class="inline-settings">
                  <InstanceMetaPanel
                    :instance="selected"
                    :versions="versions"
                    :loaders="LOADERS"
                    @update="onMetaUpdate"
                  />
                  <LaunchArgsPanel
                    :args="argsOf(selected.uuid)"
                    :javas="javas"
                    @update:args="updateArgs"
                  />
                </div>

                <!-- 自定义执行 -->
                <div class="args-section">
                  <button class="args-toggle" @click="execOpen = !execOpen">
                    <span>⚙ {{ t("exec.title") }}</span>
                    <svg
                      class="args-chevron"
                      :class="{ flip: execOpen }"
                      viewBox="0 0 24 24"
                      width="13"
                      height="13"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <path d="m6 9 6 6 6-6" />
                    </svg>
                  </button>
                  <CustomExecPanel
                    v-if="execOpen"
                    :args="argsOf(selected.uuid)"
                    @update:args="updateArgs"
                  />
                </div>

                <!-- 自定义服务器（自动加入 + MOTD 展示） -->
                <div class="args-section">
                  <button class="args-toggle" @click="serverOpen = !serverOpen">
                    <span>⚙ {{ t("server.title") }}</span>
                    <svg
                      class="args-chevron"
                      :class="{ flip: serverOpen }"
                      viewBox="0 0 24 24"
                      width="13"
                      height="13"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <path d="m6 9 6 6 6-6" />
                    </svg>
                  </button>
                  <div v-if="serverOpen" class="server-config">
                    <!-- 自动加入服务器设置：地址 + 端口 + 启动时加入（一行） -->
                    <div class="server-row">
                      <span class="server-label">{{ t("server.ip") }}</span>
                      <input
                        class="field-input grow"
                        :value="argsOf(selected.uuid).serverIp"
                        placeholder="127.0.0.1"
                        spellcheck="false"
                        @input="onServerIp(($event.target as HTMLInputElement).value)"
                      />
                      <span class="server-label small">{{ t("server.port") }}</span>
                      <NumberStepper
                        :model-value="argsOf(selected.uuid).serverPort"
                        :min="1"
                        :max="65535"
                        :step="1"
                        @update:model-value="onServerPort"
                      />
                      <label class="chk">
                        <input
                          type="checkbox"
                          :checked="argsOf(selected.uuid).joinServer"
                          @change="onServerJoin(($event.target as HTMLInputElement).checked)"
                        />
                        {{ t("server.join") }}
                      </label>
                    </div>

                    <!-- MOTD 展示：两行服务器信息 + 一行状态 -->
                    <div class="motd-card">
                      <div class="motd-icon">MC</div>
                      <div class="motd-info">
                        <div class="motd-name">{{ t("server.name") }}</div>
                        <div class="motd-text">{{ t("server.motd") }}</div>
                        <div class="motd-meta">
                          <span class="motd-online">{{ t("server.players", { now: motdNow, max: 200 }) }}</span>
                          <span class="sep">·</span>
                          <span>{{ t("server.version", { v: "1.21.1" }) }}</span>
                          <span class="sep">·</span>
                          <span>{{ t("server.ping", { ms: motdPing }) }}</span>
                        </div>
                      </div>
                    </div>
                  </div>
                </div>

                <!-- 游戏内代理 -->
                <div class="args-section">
                  <button class="args-toggle" @click="proxyOpen = !proxyOpen">
                    <span>⚙ {{ t("proxy.title") }}</span>
                    <svg
                      class="args-chevron"
                      :class="{ flip: proxyOpen }"
                      viewBox="0 0 24 24"
                      width="13"
                      height="13"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="2"
                    >
                      <path d="m6 9 6 6 6-6" />
                    </svg>
                  </button>
                  <ProxyPanel
                    v-if="proxyOpen"
                    :args="argsOf(selected.uuid)"
                    @update:args="updateArgs"
                  />
                </div>
              </template>
              <div v-else class="placeholder">{{ t("detail.selectHint") }}</div>
            </template>
          </section>
        </template>

        <!-- 模式：游戏实例列表（下拉框选中实例） -->
        <template v-else>
          <section class="list-mode">
            <div class="list-toolbar">
              <SegmentedTabs
                :model-value="mode"
                :options="MODE_OPTIONS"
                @update:model-value="mode = $event as ViewMode"
              />
            </div>

            <InstanceIcon
              :name="selected?.name ?? '—'"
              :uuid="selected?.uuid ?? '0'"
              :size="120"
            />
            <h2 class="list-title">{{ selected?.name ?? t("launch.selectInstance") }}</h2>

            <InstanceSelect
              :instances="instances"
              :model-value="selected?.uuid ?? null"
              @update:model-value="onPickInstance"
            />

            <div class="launch-actions">
              <BaseButton variant="primary" size="lg" :disabled="!selected" @click="launch">
                ▶ {{ t("launch.play") }}
              </BaseButton>
              <BaseButton :disabled="!selected" @click="argsOpen = !argsOpen">
                ⚙ {{ t("args.title") }}
              </BaseButton>
            </div>
            <div v-if="argsOpen && selected" class="list-args">
              <LaunchArgsPanel :args="argsOf(selected.uuid)" :javas="javas" @update:args="updateArgs" />
            </div>
          </section>
        </template>
      </main>

      <!-- 服务器 MOTD 悬浮卡片（启动器下方） -->
      <div class="motd-float">
        <button
          class="motd-refresh"
          :title="t('server.refresh')"
          @click="refreshMotd"
        >
          <svg
            viewBox="0 0 24 24"
            width="13"
            height="13"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            :class="{ spin: motdRefreshing }"
          >
            <path d="M21 12a9 9 0 1 1-2.64-6.36M21 3v6h-6" />
          </svg>
        </button>
        <div class="motd-float-icon">MC</div>
        <div class="motd-float-info">
          <div class="motd-float-name">{{ t("server.name") }}</div>
          <div class="motd-float-text">{{ t("server.motd") }}</div>
          <div class="motd-float-meta">
            <span class="motd-online">{{ t("server.players", { now: motdNow, max: 200 }) }}</span>
            <span class="sep">·</span>
            <span>{{ t("server.version", { v: "1.21.1" }) }}</span>
            <span class="sep">·</span>
            <span>{{ t("server.ping", { ms: motdPing }) }}</span>
          </div>
        </div>
      </div>
    </template>

    <!-- ===== 启动界面（启动中 / 运行中） ===== -->
    <LaunchScreen
      v-if="gameActive && selected"
      :instance="selected"
      :status-text="statusText"
      :logs="logs"
      :running="running"
      @stop="stop"
    />

    <!-- ===== 添加实例弹窗 ===== -->
    <BaseModal v-if="showAdd" :title="t('add.title')" @close="showAdd = false">
      <label class="field-label">{{ t("add.name") }}</label>
      <input v-model="newName" class="field-input" :placeholder="t('add.namePlaceholder')" spellcheck="false" />

      <label class="field-label">{{ t("add.version") }}</label>
      <select v-model="newVersion" class="version-select" size="8">
        <option v-for="v in versions" :key="v.id" :value="v.id">
          {{ v.id }}（{{ v.versionType }}）
        </option>
      </select>
      <div v-if="versions.length === 0 && !addError" class="empty-tip">{{ t("add.loading") }}</div>

      <p v-if="addError" class="error-text">{{ addError }}</p>

      <div class="modal-actions">
        <BaseButton @click="showAdd = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" :disabled="creating" @click="create">
          {{ creating ? t("add.creating") : t("add.create") }}
        </BaseButton>
      </div>
    </BaseModal>

    <!-- ===== 重命名实例弹窗 ===== -->
    <BaseModal v-if="showRename && selected" :title="t('actions.renameTitle')" @close="showRename = false">
      <label class="field-label">{{ t("add.name") }}</label>
      <input v-model="renameName" class="field-input" @keyup.enter="doRename" spellcheck="false" />

      <div class="modal-actions">
        <BaseButton @click="showRename = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" :disabled="renameBusy" @click="doRename">
          {{ t("actions.confirm") }}
        </BaseButton>
      </div>
    </BaseModal>

    <!-- ===== 删除实例确认 ===== -->
    <BaseModal v-if="showDelete && selected" :title="t('actions.deleteTitle')" @close="showDelete = false">
      <p class="delete-tip">{{ t("actions.deleteConfirm", { name: selected.name }) }}</p>

      <div class="modal-actions">
        <BaseButton @click="showDelete = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" :disabled="deleteBusy" @click="doDelete">
          {{ t("actions.delete") }}
        </BaseButton>
      </div>
    </BaseModal>

    <!-- ===== 轻提示 ===== -->
    <Transition name="toast">
      <div v-if="toast" class="toast">{{ toast }}</div>
    </Transition>

    <!-- ===== 添加分组弹窗 ===== -->
    <BaseModal v-if="showAddGroup" :title="t('group.addGroup')" @close="showAddGroup = false">
      <label class="field-label">{{ t("group.namePlaceholder") }}</label>
      <input v-model="groupName" class="field-input" @keyup.enter="createGroup" spellcheck="false" />

      <p v-if="groupError" class="error-text">{{ groupError }}</p>

      <div class="modal-actions">
        <BaseButton @click="showAddGroup = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" :disabled="groupAdding" @click="createGroup">
          {{ t("group.addGroup") }}
        </BaseButton>
      </div>
    </BaseModal>
  </div>
</template>

<style scoped>
.main-window {
  display: flex;
  flex-direction: column;
  height: 100vh;
}

/* ================= 启动画面 ================= */

.splash {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  background: linear-gradient(160deg, var(--bg-side) 0%, var(--bg) 100%);
}

.splash-logo {
  width: 72px;
  height: 72px;
  border-radius: 18px;
  background: linear-gradient(135deg, #4f8cff, #7c5cff);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 800;
  font-size: 30px;
  color: #fff;
  box-shadow: 0 8px 24px rgba(79, 140, 255, 0.35);
}

.splash-name {
  font-size: 20px;
  font-weight: 700;
  color: var(--text);
}

.splash-spinner {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  border: 3px solid var(--border);
  border-top-color: var(--accent);
  animation: splash-rotate 0.9s linear infinite;
  margin-top: 8px;
}

@keyframes splash-rotate {
  to {
    transform: rotate(360deg);
  }
}

.splash-text {
  font-size: 13px;
  color: var(--text-dim);
}

/* ================= 初始化界面 ================= */

.setup {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(160deg, var(--bg-side) 0%, var(--bg) 100%);
}

.setup-card {
  width: 400px;
  padding: 36px;
  border-radius: 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-lg);
}

.setup-logo {
  font-size: 34px;
  font-weight: 800;
  letter-spacing: 2px;
  background: linear-gradient(120deg, #4f8cff, #7c5cff);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  text-align: center;
}

.setup-card h1 {
  font-size: 20px;
  text-align: center;
  margin: 10px 0 4px;
}

.setup-sub {
  color: var(--text-dim);
  font-size: 13px;
  text-align: center;
  margin-bottom: 22px;
}

/* ================= 顶部栏 ================= */

.topbar {
  height: 64px;
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 0 18px;
  background: var(--bg-side);
  border-bottom: 1px solid var(--border);
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
}

.brand-logo {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  background: linear-gradient(135deg, #4f8cff, #7c5cff);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 800;
  font-size: 16px;
  color: #fff;
  box-shadow: 0 3px 10px rgba(79, 140, 255, 0.35);
}

.brand-text {
  display: flex;
  flex-direction: column;
  line-height: 1.2;
}

.brand-name {
  font-size: 15px;
  font-weight: 700;
}

.brand-sub {
  font-size: 11px;
  color: var(--text-dim);
}

.topbar-right {
  display: flex;
  align-items: center;
  gap: 6px;
}

.topbar-icon-btn {
  width: 36px;
  height: 36px;
  border-radius: 10px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
}

.topbar-icon-btn:hover {
  background: var(--bg-card);
  border-color: var(--border);
  color: var(--text);
}

.topbar-icon-btn.pressed {
  background: var(--accent-soft);
  border-color: var(--accent-border);
  color: var(--accent);
}

/* ================= 主体 ================= */

.main {
  flex: 1;
  display: flex;
  min-height: 0;
}

.main.side-right {
  flex-direction: row-reverse;
}

/* ----- 侧栏（分组 / 平铺） ----- */

.sidebar {
  width: 320px;
  min-width: 320px;
  background: var(--bg-side);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.main.side-right .sidebar {
  border-right: none;
  border-left: 1px solid var(--border);
}

.sidebar-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 12px 12px 10px;
}

.sidebar-head-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

/* 实例搜索框 */
.search-box {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 12px 8px;
  padding: 0 10px;
  height: 34px;
  border-radius: 9px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
  flex-shrink: 0;
}

.search-box:focus-within {
  border-color: var(--accent);
}

.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  background: transparent;
  color: var(--text);
  font-size: 12.5px;
  outline: none;
  font-family: inherit;
}

.search-input::placeholder {
  color: var(--text-dim);
}

.search-clear {
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  cursor: pointer;
  padding: 2px 4px;
}

.search-clear:hover {
  color: var(--text);
}

/* 窗口过小时侧栏悬浮 */
.sidebar-backdrop {
  display: none;
}

@media (max-width: 880px) {
  .sidebar-backdrop {
    display: block;
    position: fixed;
    inset: 64px 0 0 0;
    background: var(--overlay);
    z-index: 154;
  }

  .sidebar {
    position: fixed;
    top: 64px;
    bottom: 0;
    left: 0;
    z-index: 160;
    box-shadow: var(--shadow-lg);
  }

  .main.side-right .sidebar {
    left: auto;
    right: 0;
  }
}

/* 侧栏收起后的展开把手 */
.sidebar-expand {
  width: 24px;
  border: none;
  background: var(--bg-side);
  color: var(--text-dim);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
  flex-shrink: 0;
  border-right: 1px solid var(--border);
}

.main.side-right .sidebar-expand {
  border-right: none;
  border-left: 1px solid var(--border);
}

.sidebar-expand:hover {
  background: var(--bg-hover);
  color: var(--accent);
}

.icon-btn {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text);
  font-size: 16px;
  line-height: 1;
  cursor: pointer;
  transition: all 0.15s;
  flex-shrink: 0;
}

.icon-btn:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
}

/* ----- 分组模式列表 ----- */

.group-list {
  flex: 1;
  overflow-y: auto;
  padding: 2px 10px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.group-title-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 2px;
}

.group-title {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 700;
  color: var(--text-dim);
  padding: 8px 6px;
  border: none;
  background: transparent;
  cursor: pointer;
  letter-spacing: 0.5px;
  font-family: inherit;
  border-radius: 8px;
  transition: background 0.12s;
  text-align: left;
}

.group-title:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.group-chevron {
  transition: transform 0.15s;
  flex-shrink: 0;
}

.group-chevron.collapsed {
  transform: rotate(-90deg);
}

.group-count {
  font-size: 11px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 20px;
  padding: 0 8px;
  color: var(--text-dim);
  font-weight: 500;
  margin-left: auto;
}

.group-items {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 0;
}

/* 添加实例行（与实例行同尺寸，位于实例上方） */
.add-inst-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 9px 12px;
  border: 1px dashed var(--border);
  border-radius: 10px;
  background: transparent;
  color: var(--text-dim);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.12s;
  min-height: 48px;
}

.add-inst-row:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

.add-inst-icon {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  background: var(--bg-hover);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  flex-shrink: 0;
}

.add-inst-row:hover .add-inst-icon {
  background: var(--accent-soft);
}

.add-row {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 9px 12px;
  border: 1px dashed var(--border);
  border-radius: 10px;
  background: transparent;
  color: var(--text-dim);
  font-size: 12.5px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
  margin-bottom: 6px;
}

.add-row:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

.inst-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border-radius: 10px;
  cursor: grab;
  transition: background 0.12s;
  min-height: 50px;
}

.inst-row:active {
  cursor: grabbing;
}

.inst-row.dragging {
  opacity: 0.45;
}

.group-block.drop-target {
  border: 1px dashed var(--accent);
  border-radius: 12px;
  background: var(--accent-soft);
  margin: 0 -4px;
  padding: 0 4px;
}

.inst-row:hover {
  background: var(--bg-hover);
}

.inst-row.active {
  background: var(--accent-soft);
  outline: 1px solid var(--accent-border);
}

.inst-name {
  font-size: 13.5px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

/* ----- 平铺模式网格 ----- */

.tile-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px 10px 14px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
  gap: 10px;
  align-content: start;
}

.tile {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 16px 8px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.tile:hover {
  background: var(--bg-hover);
  transform: translateY(-2px);
}

.tile.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.tile.add-tile {
  border: 1px solid var(--border);
  background: var(--bg-card);
  justify-content: center;
}

.tile.add-tile:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

.add-plus {
  font-size: 26px;
  line-height: 1;
  color: var(--text-dim);
}

.tile.add-tile:hover .add-plus {
  color: var(--accent);
}

.add-tile .tile-name {
  color: var(--accent);
  font-weight: 600;
}

.tile-empty {
  grid-column: 1 / -1;
}

.loader-corner {
  position: absolute;
  top: 8px;
  left: 8px;
}

.tile-name {
  font-size: 12px;
  text-align: center;
  line-height: 1.3;
  word-break: break-all;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.tile .run-dot {
  position: absolute;
  top: 10px;
  right: 10px;
}

.run-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--green);
  box-shadow: 0 0 6px var(--green);
  flex-shrink: 0;
}

/* ----- 右侧内容区 ----- */

.detail {
  flex: 1;
  display: flex;
  flex-direction: column;
  /* 底部留白，避免被 MOTD 悬浮卡片遮挡 */
  padding: 16px 26px 130px;
  gap: 16px;
  min-width: 0;
  overflow-y: auto;
}

.news-head {
  display: flex;
  align-items: center;
}

.news-page {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding: 18px 28px 130px;
  overflow-y: auto;
  min-width: 0;
}

.news-page-head {
  display: flex;
  align-items: center;
}

.detail-top {
  display: flex;
  align-items: flex-start;
  gap: 18px;
}

.detail-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  justify-content: center;
  min-height: 84px;
}

/* 右侧：启动游戏（大）+ 添加/管理资源（小，平分启动按钮宽度） */
.detail-side {
  display: flex;
  flex-direction: column;
  align-items: stretch;
  gap: 8px;
  width: 240px;
  flex-shrink: 0;
}

.side-row {
  display: flex;
  gap: 8px;
}

.side-row :deep(.ui-btn) {
  flex: 1;
  min-width: 0;
}

/* 启动游戏按钮：更大、阴影更柔和，撑满整列 */
.play-btn {
  width: 100%;
  font-size: 16px;
  padding: 16px 0;
}

/* 服务器 MOTD 悬浮卡片（启动器下方居中，左图标右信息） */
.motd-float {
  position: fixed;
  left: 50%;
  bottom: 18px;
  transform: translateX(-50%);
  z-index: 260;
  display: flex;
  align-items: center;
  gap: 12px;
  max-width: 460px;
  padding: 12px 16px;
  border-radius: 14px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  box-shadow: var(--shadow-lg);
  cursor: default;
}

.motd-float:hover .motd-refresh {
  opacity: 1;
}

.motd-refresh {
  position: absolute;
  top: 6px;
  right: 6px;
  width: 24px;
  height: 24px;
  border-radius: 7px;
  border: 1px solid var(--border);
  background: var(--bg-side);
  color: var(--text-dim);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  opacity: 0;
  transition: opacity 0.15s, color 0.15s;
}

.motd-refresh:hover {
  color: var(--accent);
}

.motd-refresh .spin {
  animation: motd-spin 0.8s linear infinite;
}

@keyframes motd-spin {
  to {
    transform: rotate(360deg);
  }
}

.motd-float-icon {
  width: 44px;
  height: 44px;
  border-radius: 10px;
  background: linear-gradient(135deg, #3ecf8e, #22d3ee);
  color: #fff;
  font-weight: 800;
  font-size: 13px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.motd-float-info {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
}

.motd-float-name {
  font-size: 14px;
  font-weight: 700;
  color: var(--text);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.motd-float-text {
  font-size: 12.5px;
  font-weight: 500;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.motd-float-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-dim);
  white-space: nowrap;
}

/* 自定义服务器 MOTD 卡片 */
.motd-card {
  display: flex;
  align-items: center;
  gap: 14px;
  padding: 14px 16px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--bg-card);
}

.motd-icon {
  width: 46px;
  height: 46px;
  border-radius: 10px;
  background: linear-gradient(135deg, #3ecf8e, #22d3ee);
  color: #fff;
  font-weight: 800;
  font-size: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  flex-shrink: 0;
}

.motd-info {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
}

.motd-text {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text);
  word-break: break-all;
}

.motd-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
  color: var(--text-dim);
}

.motd-online {
  color: var(--green);
}

/* 元信息行右侧快捷操作 */
.meta-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
  flex-shrink: 0;
}

.icon-btn {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.15s;
  flex-shrink: 0;
}

.icon-btn:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
  color: var(--accent);
}

.icon-btn.danger:hover {
  border-color: var(--red);
  color: var(--red);
}

.detail-info h2 {
  font-size: 22px;
  font-weight: 700;
  word-break: break-all;
}

.badges {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-top: 8px;
  flex-wrap: wrap;
}

.badge {
  font-size: 11.5px;
  padding: 3px 10px;
  border-radius: 20px;
  background: var(--bg-hover);
  color: var(--text-dim);
  display: inline-flex;
  align-items: center;
  gap: 5px;
}

.badge.loader {
  background: var(--accent-soft);
  color: var(--accent);
}

.badge.type {
  background: rgba(62, 207, 142, 0.14);
  color: var(--green);
}

.badge.dim {
  background: transparent;
  border: 1px solid var(--border);
}

.detail-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12.5px;
  color: var(--text-dim);
}

.sep {
  opacity: 0.5;
}

.launch-row {
  display: flex;
  gap: 12px;
  margin-top: 4px;
}

/* 加载器文字标签 */
.loader-text {
  font-size: 10px;
  padding: 1px 7px;
  border-radius: 8px;
  background: var(--accent-soft);
  color: var(--accent);
  white-space: nowrap;
  flex-shrink: 0;
  line-height: 1.6;
}

/* ----- 实例操作按钮 ----- */

.action-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(132px, 1fr));
  gap: 8px;
}

.action-btn {
  width: 100%;
  padding: 9px 10px;
  border-radius: 9px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
  font-size: 12.5px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.action-btn:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

/* 二级菜单 */
.menu-wrap {
  position: relative;
}

.action-btn .export-chevron {
  margin-left: 4px;
  transition: transform 0.15s;
  vertical-align: -1px;
}

.action-btn .export-chevron.flip {
  transform: rotate(180deg);
}

.menu-drop {
  position: absolute;
  left: 0;
  top: calc(100% + 4px);
  min-width: 100%;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  padding: 5px;
  z-index: 120;
}

.menu-item {
  width: 100%;
  text-align: left;
  padding: 8px 12px;
  border: none;
  border-radius: 7px;
  background: transparent;
  color: var(--text);
  font-size: 12.5px;
  font-family: inherit;
  cursor: pointer;
  white-space: nowrap;
}

.menu-item:hover {
  background: var(--bg-hover);
  color: var(--accent);
}

/* ----- 启动参数 ----- */

.args-section {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

/* 实例设置：元信息 + 启动参数 直接展示 */
.inline-settings {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* 自定义服务器配置 */
.server-config {
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 14px 16px;
}

.server-row {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: nowrap;
}

.server-label {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-dim);
  min-width: 60px;
  white-space: nowrap;
}

.server-label.small {
  min-width: 0;
  margin-left: 8px;
  margin-right: 14px;
}

.server-row .chk {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  color: var(--text);
  cursor: pointer;
  accent-color: var(--accent);
  margin-left: 12px;
  white-space: nowrap;
}

.motd-name {
  font-size: 14px;
  font-weight: 700;
  color: var(--text);
}

.args-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  padding: 11px 14px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text);
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
}

.args-toggle:hover {
  border-color: var(--accent);
}

.args-chevron {
  color: var(--text-dim);
  transition: transform 0.15s;
}

.args-chevron.flip {
  transform: rotate(180deg);
}

.list-args {
  width: 100%;
  max-width: 560px;
}

.delete-tip {
  font-size: 13.5px;
  color: var(--text);
  line-height: 1.7;
  margin: 6px 0 2px;
}

.delete-tip.dim {
  color: var(--text-dim);
  font-size: 12.5px;
}

/* ----- 轻提示 ----- */

.toast {
  position: fixed;
  bottom: 34px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--bg-card);
  border: 1px solid var(--accent-border);
  color: var(--text);
  font-size: 13px;
  padding: 11px 22px;
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  z-index: 500;
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(8px);
}

.placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-dim);
  font-size: 14px;
}

/* ----- 列表模式（下拉框选中实例） ----- */

.list-mode {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 16px;
  padding: 20px 30px 130px;
  overflow-y: auto;
  min-width: 0;
}

.list-toolbar {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  width: 100%;
  max-width: 560px;
  margin-bottom: 4px;
}

.list-title {
  font-size: 20px;
  font-weight: 700;
  word-break: break-all;
  text-align: center;
}

.launch-actions {
  display: flex;
  gap: 12px;
  align-items: center;
}

/* ================= 弹窗细节 ================= */

.modal-sub {
  font-size: 12.5px;
  color: var(--text-dim);
  margin-bottom: 4px;
  word-break: break-all;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 18px;
}
</style>
