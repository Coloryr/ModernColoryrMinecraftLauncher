<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import {
  api,
  onGameExit,
  onGameLog,
  onInstanceChange,
  onLaunchError,
  onLaunchState,
} from "../../lib/api-ipc";
import { t } from "../../lib/i18n";
import { showToast } from "../../lib/toast";
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
import HomePage from "../../components/HomePage.vue";
import CustomExecPanel from "../../components/CustomExecPanel.vue";
import ProxyPanel from "../../components/ProxyPanel.vue";
import SplashScreen from "../../components/ui/SplashScreen.vue";
import MainTopbar from "./topbar/MainTopbar.vue";
import MainSidebar from "./sidebar/MainSidebar.vue";
import MainCtxMenu from "./ctxmenu/MainCtxMenu.vue";
import { useInstanceDrag } from "../../composables/useInstanceDrag";
import { useMultiSelect } from "../../composables/useMultiSelect";
import { useFileDrop } from "../../composables/useFileDrop";
import type { CtxMenuState, FeatureId, InstMenuAction, ViewMode } from "./types";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import NumberStepper from "../../components/ui/NumberStepper.vue";

await getCurrentWindow().setTitle(t("winTitle.main"));

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

const mode = ref<ViewMode>("group");
const MODE_OPTIONS = computed(() => [
  { value: "group", label: t("mode.group"), icon: "folder" },
  { value: "grid", label: t("mode.grid"), icon: "grid" },
  { value: "list", label: t("mode.list"), icon: "list" },
]);

// 手动添加的空分组（来自 api.getGroups）
const extraGroups = ref<string[]>([]);

const groups = computed(() => {
  const defaultKey = t("group.default");
  const map = new Map<string, InstanceInfo[]>();
  for (const inst of instances.value) {
    const key = inst.group || defaultKey;
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(inst);
  }
  for (const g of extraGroups.value) {
    if (!map.has(g)) map.set(g, []);
  }
  // 默认分组永远存在且置顶
  if (!map.has(defaultKey)) map.set(defaultKey, []);
  // 分组顺序：默认分组 → extraGroups（持久顺序）→ 其余按首次出现顺序
  const order = [
    defaultKey,
    ...extraGroups.value.filter((k) => k !== defaultKey),
    ...[...map.keys()].filter(
      (k) => k !== defaultKey && !extraGroups.value.includes(k),
    ),
  ];
  return order.map((name) => ({ name, items: map.get(name) ?? [] }));
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
import { getCurrentWindow } from "@tauri-apps/api/window";
const accounts = storeAccounts;
const currentAccount = storeCurrentAccount;

function onAccountChange(account: Account) {
  setCurrentAccount(account);
  playerName.value = account.userName;
  localStorage.setItem("mcml.playerName", account.userName);
}

// ================= 启动状态 =================

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

// ================= 自定义拖拽（useInstanceDrag，见下方多选初始化之后） =================

/** 点击分组标题（拖拽结束后抑制误触折叠） */
function onGroupTitleClick(name: string) {
  if (consumeSuppressClick()) return;
  toggleGroup(name);
}

// ================= 多选模式（逻辑见 composables/useMultiSelect） =================
// 入口：右键分组 → 全选 → 进入多选；多选模式下右键实例出现操作菜单。

const {
  multiSelect,
  selectedIds,
  enterMultiSelect,
  exitMultiSelect,
  toggleSelect,
} = useMultiSelect({
  selected,
  newsActive,
  collapsedGroups,
  onExit: closeCtxMenu,
});

const {
  dragActive,
  draggingUuid,
  onDragPointerDown,
  isInstInsert,
  isInstInsertEnd,
  isGroupInsert,
  consumeSuppressClick,
} = useInstanceDrag({
  multiSelect,
  groups,
  collapsedGroups,
  loadInstances,
  loadGroups,
});

/** 实例点击：多选模式切换勾选，普通模式单选（拖拽结束后忽略误触点击） */
function onInstClick(inst: InstanceInfo) {
  if (consumeSuppressClick()) return;
  if (multiSelect.value) toggleSelect(inst);
  else select(inst);
}

// ----- 自定义右键菜单 -----

const ctxMenu = ref<CtxMenuState | null>(null);
/** 移动分组二级视图（列出所有分组） */
const moveGroupView = ref(false);
/** 转移分组的源分组（null 表示多选实例移动） */
const groupMoveSource = ref<string | null>(null);

function openCtxMenu(e: MouseEvent, payload: Omit<CtxMenuState, "x" | "y">) {
  // 简单边界钳制，避免菜单超出窗口
  const x = Math.min(e.clientX, window.innerWidth - 200);
  const y = Math.min(e.clientY, window.innerHeight - 180);
  ctxMenu.value = { x, y, ...payload };
  moveGroupView.value = false;
  groupMoveSource.value = null;
}

function closeCtxMenu() {
  ctxMenu.value = null;
  moveGroupView.value = false;
}

/** 右键分组标题：菜单提供“全选”进入多选 */
function onGroupContext(e: MouseEvent, groupName: string) {
  openCtxMenu(e, { kind: "group", group: groupName });
}

/** 分组菜单“全选”：选中该分组全部实例并进入多选 */
function onGroupSelectAll(groupName?: string) {
  const g = groups.value.find((x) => x.name === groupName);
  closeCtxMenu();
  if (!g || g.items.length === 0) return;
  enterMultiSelect(g.items.map((i) => i.uuid));
  showToast(t("multi.selectAllDone", { count: g.items.length }));
}

/** 分组菜单“启动全部”：启动该分组全部实例 */
async function launchGroupAll(groupName?: string) {
  const g = groups.value.find((x) => x.name === groupName);
  closeCtxMenu();
  if (!g || g.items.length === 0) return;
  for (const inst of g.items) {
    try {
      await api.launchGame(inst.uuid, playerName.value);
      inst.running = true;
    } catch {
      /* 忽略单个失败 */
    }
  }
  showToast(t("multi.launched", { count: g.items.length }));
}

/** 分组菜单“转移分组”：打开目标分组选择视图 */
function openGroupMoveView(groupName?: string) {
  const g = groups.value.find((x) => x.name === groupName);
  if (!g) return;
  groupMoveSource.value = g.name;
  moveGroupView.value = true;
}

/** 把源分组的全部实例合并到目标分组（默认分组 = null），保留空分组 */
async function moveGroupTo(sourceName: string, targetName: string | null) {
  const g = groups.value.find((x) => x.name === sourceName);
  const target = targetName === t("group.default") ? null : targetName;
  closeCtxMenu();
  if (!g || g.items.length === 0 || target === sourceName) return;
  for (const inst of g.items) {
    if (inst.group !== target) {
      await api.updateInstance(inst.uuid, { group: target });
    }
  }
  await loadInstances();
  showToast(t("multi.moved", { count: g.items.length }));
}

/** 移动目标点击：按当前视图路由到分组转移或多选实例移动 */
function onMoveTarget(targetName: string | null) {
  if (groupMoveSource.value) moveGroupTo(groupMoveSource.value, targetName);
  else moveSelectedToGroup(targetName);
}

/** 删除分组（组内实例移至默认分组） */
const showDeleteGroup = ref(false);
const deleteGroupName = ref("");
const deleteGroupCount = ref(0);
const deleteGroupBusy = ref(false);

function onDeleteGroup(groupName?: string) {
  if (!groupName || groupName === t("group.default")) return;
  const g = groups.value.find((x) => x.name === groupName);
  closeCtxMenu();
  if (!g) return;
  deleteGroupName.value = g.name;
  deleteGroupCount.value = g.items.length;
  showDeleteGroup.value = true;
}

async function doDeleteGroup() {
  deleteGroupBusy.value = true;
  const name = deleteGroupName.value;
  for (const inst of instances.value) {
    const key = inst.group || t("group.default");
    if (key === name) {
      await api.updateInstance(inst.uuid, { group: null });
    }
  }
  await api.removeGroup(name);
  deleteGroupBusy.value = false;
  showDeleteGroup.value = false;
  await Promise.all([loadInstances(), loadGroups()]);
  showToast(t("group.deleted", { name }));
}

/** 右键实例：多选模式打开操作菜单；普通模式弹出实例操作菜单 */
function onInstContext(e: MouseEvent, inst: InstanceInfo) {
  if (multiSelect.value) {
    // 右键未勾选的实例时先加入选择
    if (!selectedIds.value.has(inst.uuid)) {
      const s = new Set(selectedIds.value);
      s.add(inst.uuid);
      selectedIds.value = s;
    }
    openCtxMenu(e, { kind: "multi" });
  } else {
    // 普通模式：先选中该实例，再弹出实例菜单
    select(inst);
    openCtxMenu(e, { kind: "instance", instance: inst });
  }
}

/** 实例右键菜单动作（与详情面板一致） */
function onInstMenuAction(id: InstMenuAction) {
  closeCtxMenu();
  switch (id) {
    case "launch":
      launch();
      break;
    case "rename":
      onAction("rename");
      break;
    case "delete":
      onAction("delete");
      break;
    default:
      onAction(id);
  }
}

/** 顶部工具栏的“移动分组”直接打开分组列表视图 */
function openMoveFromBar(e: MouseEvent) {
  openCtxMenu(e, { kind: "multi" });
  moveGroupView.value = true;
}

/** 把选中的实例移动到指定分组（null = 默认分组） */
async function moveSelectedToGroup(groupName: string | null) {
  const ids = [...selectedIds.value];
  const target = groupName === t("group.default") ? null : groupName;
  for (const uuid of ids) {
    const inst = instances.value.find((i) => i.uuid === uuid);
    if (inst && inst.group !== target) {
      await api.updateInstance(uuid, { group: target });
    }
  }
  closeCtxMenu();
  await loadInstances();
  const key = target || t("group.default");
  collapsedGroups.value = { ...collapsedGroups.value, [key]: false };
  showToast(t("multi.moved", { count: ids.length }));
}

/** 多选删除 */
const showMultiDelete = ref(false);
const multiDeleteBusy = ref(false);

function onMultiDelete() {
  closeCtxMenu();
  showMultiDelete.value = true;
}

async function doMultiDelete() {
  multiDeleteBusy.value = true;
  const ids = [...selectedIds.value];
  for (const uuid of ids) {
    await api.deleteInstance(uuid);
  }
  multiDeleteBusy.value = false;
  showMultiDelete.value = false;
  exitMultiSelect();
  await Promise.all([loadInstances(), loadGroups()]);
  showToast(t("multi.deleted", { count: ids.length }));
}

/** 启动所有选中实例 */
async function multiLaunch() {
  const ids = [...selectedIds.value];
  closeCtxMenu();
  for (const uuid of ids) {
    const inst = instances.value.find((i) => i.uuid === uuid);
    try {
      await api.launchGame(uuid, playerName.value);
      if (inst) inst.running = true;
    } catch {
      /* 忽略单个失败 */
    }
  }
  exitMultiSelect();
  showToast(t("multi.launched", { count: ids.length }));
}

// 点击空白处关闭菜单；Esc 依次关闭菜单 / 退出多选
function onDocClick() {
  if (ctxMenu.value) closeCtxMenu();
}

function onDocKeyDown(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  if (ctxMenu.value) closeCtxMenu();
  else if (multiSelect.value) exitMultiSelect();
}

/** 添加实例窗口创建成功后（跨窗口 storage 事件），刷新并选中新实例 */
async function onAddedInstanceStorage(e: StorageEvent) {
  if (e.key !== "mcml.addedInstance" || !e.newValue) return;
  await Promise.all([loadInstances(), loadGroups()]);
  const inst = instances.value.find((i) => i.uuid === e.newValue);
  if (inst) select(inst);
}

// 轻提示（统一使用全局 showToast）
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

const features: Array<{ id: FeatureId; icon: string }> = [
  { id: "settings", icon: "gear" },
  { id: "stats", icon: "chart" },
  { id: "skin", icon: "user" },
  { id: "help", icon: "book" },
];

// ================= 添加分组 弹窗 =================

const versions = ref<VersionInfo[]>([]);

// ================= 拖拽整合包文件（逻辑见 composables/useFileDrop） =================

const { fileDragOver } = useFileDrop({
  versions,
  loadInstances,
});

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
    await Promise.all([loadInstances(), loadGroups(), loadJava()]);
    // 版本列表走网络（Mojang 清单），不阻塞启动
    loadVersions();
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

// ================= 启动 =================
// 不弹窗：只有正在运行的实例的启动按钮显示加载态并禁用

async function launch() {
  if (!selected.value || selected.value.running) return;
  const uuid = selected.value.uuid;
  localStorage.setItem("mcml.lastInstance", uuid);
  selected.value.running = true;
  statusText.value = t("launch.launching");
  logs.value = [];
  try {
    await api.launchGame(uuid, playerName.value);
  } catch (e) {
    if (selected.value) selected.value.running = false;
    statusText.value = t("launch.failed");
    appendLog(t("launch.error", { msg: String(e) }));
  }
}

function onPickInstance(uuid: string) {
  const inst = instances.value.find((i) => i.uuid === uuid);
  if (inst) select(inst);
}

// ================= 添加实例（独立窗口） =================

function openAdd() {
  openWindow("add");
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
  document.addEventListener("click", onDocClick);
  document.addEventListener("keydown", onDocKeyDown);
  window.addEventListener("storage", onAddedInstanceStorage);
  initLoading.value = true;
  try {
    await api.initCore(localDir.value.trim() || null, playerName.value);
    bootFailed.value = false;
    await Promise.all([loadInstances(), loadGroups(), loadJava()]);
    // 版本列表走网络（Mojang 清单），不阻塞启动
    loadVersions();
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
  document.removeEventListener("click", onDocClick);
  document.removeEventListener("keydown", onDocKeyDown);
  window.removeEventListener("storage", onAddedInstanceStorage);
});
</script>

<template>
  <div class="main-window">
    <!-- ===== 启动画面 / 初始化引导（SplashScreen 组件） ===== -->
    <SplashScreen
      v-if="splashVisible || bootFailed"
      :splash-visible="splashVisible"
      :boot-failed="bootFailed"
      :init-loading="initLoading"
      :init-error="initError"
      v-model:local-dir="localDir"
      v-model:player-name="playerName"
      @retry="doInit"
    />

    <!-- ===== 主界面 ===== -->
    <template v-else>
      <!-- 顶部栏 -->
      <MainTopbar
        :features="features"
        :news-active="newsActive"
        :theme="theme"
        :current-account="currentAccount"
        :accounts="accounts"
        @toggle-news="toggleNews"
        @feature="openWindow"
        @toggle-theme="toggleTheme"
        @update:account="onAccountChange"
      />

      <!-- 主体 -->
      <main class="main" :class="{ 'side-right': sidebarSide === 'Right' }">
        <!-- 多选模式浮动工具栏 -->
        <div v-if="multiSelect" class="multi-bar" @contextmenu.prevent @click.stop>
          <span class="multi-count">{{ t("multi.selected", { count: selectedIds.size }) }}</span>
          <button class="multi-btn" @click="openMoveFromBar($event)">{{ t("multi.moveGroup") }}</button>
          <button class="multi-btn danger" @click="onMultiDelete">{{ t("multi.delete") }}</button>
          <button class="multi-btn" @click="multiLaunch">{{ t("multi.launch") }}</button>
          <span class="multi-sep"></span>
          <button class="multi-exit" @click="exitMultiSelect">{{ t("multi.exit") }}</button>
        </div>

        <!-- 空实例：强制打开启动器主页，主页内融合空状态引导（隐藏实例分组） -->
        <template v-if="instances.length === 0">
          <section class="news-page">
            <HomePage
              :items="news"
              :last-instance="null"
              :empty="true"
              @add-instance="openAdd"
              @add-account="openWindow('account')"
              @add-java="openWindow('settings')"
            />
          </section>
        </template>

        <!-- 列表模式下启动器主页整页显示 -->
        <template v-else-if="newsActive && mode === 'list'">
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

          <MainSidebar
            v-if="!sidebarCollapsed"
            :mode="mode"
            :mode-options="MODE_OPTIONS"
            :search-text="searchText"
            :groups="groups"
            :filtered-groups="filteredGroups"
            :filtered-instances="filteredInstances"
            :searching="searching"
            :selected="selected"
            :multi-select="multiSelect"
            :selected-ids="selectedIds"
            :collapsed-groups="collapsedGroups"
            :drag-active="dragActive"
            :dragging-uuid="draggingUuid"
            :is-collapsed="isCollapsed"
            :on-drag-pointer-down="onDragPointerDown"
            :on-inst-click="onInstClick"
            :on-inst-context="onInstContext"
            :on-group-context="onGroupContext"
            :on-group-title-click="onGroupTitleClick"
            :is-inst-insert="isInstInsert"
            :is-inst-insert-end="isInstInsertEnd"
            :is-group-insert="isGroupInsert"
            @update:mode="mode = $event"
            @update:search-text="searchText = $event"
            @add-instance="openAdd"
            @add-group="showAddGroup = true"
            @collapse="setSidebarCollapsed(true)"
          />

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
                    <BaseButton
                      variant="primary"
                      size="lg"
                      class="play-btn"
                      :disabled="selected.running"
                      @click="launch"
                    >
                      <span v-if="selected.running" class="btn-spinner"></span>
                      <span v-else>▶</span> {{ t("launch.play") }}
                    </BaseButton>
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
              <template v-else-if="multiSelect">
                <div class="multi-detail">
                  <div class="multi-detail-icon">☑</div>
                  <h2>{{ t("multi.detailTitle") }}</h2>
                  <p>{{ t("multi.detailDesc", { count: selectedIds.size }) }}</p>
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
              <BaseButton
                variant="primary"
                size="lg"
                class="list-launch-btn"
                :disabled="!selected || selected.running"
                @click="launch"
              >
                <span v-if="selected?.running" class="btn-spinner"></span>
                <span v-else>▶</span> {{ t("launch.play") }}
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

      <!-- ===== 右键菜单（MainCtxMenu 组件） ===== -->
      <MainCtxMenu
        :menu="ctxMenu"
        :move-group-view="moveGroupView"
        :groups="groups"
        @select-all="onGroupSelectAll"
        @launch-group="launchGroupAll"
        @open-move-view="openGroupMoveView"
        @delete-group="onDeleteGroup"
        @inst-action="onInstMenuAction"
        @back="moveGroupView = false"
        @move-target="onMoveTarget"
        @multi-move="moveGroupView = true"
        @multi-delete="onMultiDelete"
        @multi-launch="multiLaunch"
      />
    </template>

    <!-- ===== 启动界面：已移除（启动按钮显示加载态，不弹窗） ===== -->

    <!-- ===== 添加实例：独立窗口（openAdd 打开） ===== -->

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

    <!-- ===== 多选删除确认 ===== -->
    <BaseModal v-if="showMultiDelete" :title="t('multi.deleteTitle')" @close="showMultiDelete = false">
      <p class="delete-tip">{{ t("multi.deleteConfirm", { count: selectedIds.size }) }}</p>

      <div class="modal-actions">
        <BaseButton @click="showMultiDelete = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" :disabled="multiDeleteBusy" @click="doMultiDelete">
          {{ t("multi.delete") }}
        </BaseButton>
      </div>
    </BaseModal>

    <!-- ===== 删除分组确认 ===== -->
    <BaseModal v-if="showDeleteGroup" :title="t('group.delete')" @close="showDeleteGroup = false">
      <p class="delete-tip">{{ t("group.deleteConfirm", { name: deleteGroupName, count: deleteGroupCount }) }}</p>

      <div class="modal-actions">
        <BaseButton @click="showDeleteGroup = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" :disabled="deleteGroupBusy" @click="doDeleteGroup">
          {{ t("group.delete") }}
        </BaseButton>
      </div>
    </BaseModal>

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

    <!-- ===== 拖拽整合包文件遮罩层 ===== -->
    <div v-if="fileDragOver" class="file-drop-layer">
      <div class="file-drop-box">
        <svg viewBox="0 0 24 24" width="42" height="42" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 8v13H3V8" />
          <path d="M1 3h22v5H1z" />
          <path d="M10 12h4" />
        </svg>
        <h2>{{ t("drop.title") }}</h2>
        <p>{{ t("drop.desc") }}</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.main-window {
  display: flex;
  flex-direction: column;
  height: 100vh;
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

/* 侧栏收起后的展开把手 / 遮罩（侧栏本体样式见 MainSidebar.vue） */
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
}

/* ----- 侧栏（分组 / 平铺）：样式见 MainSidebar.vue ----- */

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

/* 启动游戏按钮：更大、阴影更柔和，撑满整列；固定最小高度，加载态切换不跳动 */
.play-btn {
  width: 100%;
  font-size: 16px;
  padding: 16px 0;
  min-height: 52px;
}

/* 列表模式的启动按钮：同样固定高度 */
.list-launch-btn {
  min-height: 46px;
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

/* ----- 弹窗 / 轻提示：统一使用全局样式（theme.css） ----- */

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

/* ================= 多选模式 ================= */

/* 顶部浮动工具栏 */
.multi-bar {
  position: fixed;
  top: 72px;
  left: 50%;
  transform: translateX(-50%);
  z-index: 90;
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 12px;
  background: var(--bg-card);
  border: 1px solid var(--accent-border);
  border-radius: 14px;
  box-shadow: var(--shadow-md);
}

.multi-count {
  font-size: 13px;
  font-weight: 700;
  color: var(--accent);
  margin-right: 4px;
  white-space: nowrap;
}

.multi-btn {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  padding: 7px 12px;
  border-radius: 9px;
  border: 1px solid var(--border);
  background: var(--bg);
  color: var(--text);
  font-size: 12.5px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.12s;
}

.multi-btn:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
}

.multi-btn.danger:hover {
  border-color: var(--red);
  color: var(--red);
}

.multi-sep {
  width: 1px;
  height: 18px;
  background: var(--border);
}

.multi-exit {
  padding: 7px 12px;
  border-radius: 9px;
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-size: 12.5px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.12s;
}

.multi-exit:hover {
  color: var(--red);
  background: rgba(255, 95, 86, 0.1);
}

/* 实例勾选 */
.row-check {
  position: absolute;
  left: 5px;
  top: 5px;
  z-index: 3;
  width: 19px;
  height: 19px;
  border-radius: 50%;
  border: 1.5px solid var(--text-dim);
  background: var(--bg-card);
  color: transparent;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.12s;
  pointer-events: none;
}

.row-check.on {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.inst-row.multi-checked {
  background: var(--accent-soft);
  outline: 1px solid var(--accent-border);
}

.tile.multi-checked {
  border-color: var(--accent);
  background: var(--accent-soft);
}

/* 多选详情提示 */
.multi-detail {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 40px;
}

.multi-detail-icon {
  width: 72px;
  height: 72px;
  border-radius: 22px;
  background: var(--accent-soft);
  border: 1px solid var(--accent-border);
  color: var(--accent);
  font-size: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 8px;
}

.multi-detail h2 {
  font-size: 19px;
  font-weight: 700;
}

.multi-detail p {
  font-size: 13px;
  color: var(--text-dim);
}

/* 右键菜单样式见 MainCtxMenu.vue */

/* ================= 拖拽整合包文件遮罩层 ================= */

.file-drop-layer {
  position: fixed;
  inset: 0;
  z-index: 300;
  background: var(--overlay);
  backdrop-filter: blur(2px);
  display: flex;
  align-items: center;
  justify-content: center;
  /* 事件穿透到 window 级拖拽处理器 */
  pointer-events: none;
}

.file-drop-box {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 44px 64px;
  border: 2px dashed var(--accent);
  border-radius: 20px;
  background: var(--bg-card);
  color: var(--accent);
  box-shadow: var(--shadow-lg);
}

.file-drop-box h2 {
  font-size: 19px;
  font-weight: 700;
  color: var(--text);
}

.file-drop-box p {
  font-size: 13px;
  color: var(--text-dim);
}

/* 启动按钮加载态：与 ▶ 字号保持一致，避免按钮高度跳动 */
.btn-spinner {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  border: 2px solid rgba(255, 255, 255, 0.35);
  border-top-color: #fff;
  animation: btn-spin 0.8s linear infinite;
  flex-shrink: 0;
}

@keyframes btn-spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
