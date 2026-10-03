<script setup lang="ts">
// 主窗口：实例列表（分组 / 平铺 / 搜索）、实例详情与设置、启动流程与日志、启动器主页
// 子组件在 topbar / sidebar / ctxmenu/；拖拽、多选、文件拖放逻辑在 composables/
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import {
  api,
  getLoadState,
  onClientConfigChange,
  onCustomHomeChange,
  onGameExit,
  onInstanceChange,
  onJavaChange,
  onLaunchError,
  onLaunchState,
} from "../../lib/api";
import { commands } from "../../lib/bindings";
import { loadGuiConfig, type ClientConfig } from "../../lib/guiConfig";
import { t, tErr } from "../../lib/i18n";
import { showToast } from "../../lib/toast";
import { openWindow } from "../windowManager";
import {
  selectedInstance,
  setSelectedInstance,
  sidebarCollapsed,
  sidebarSide,
  setSidebarCollapsed,
  setViewMode,
  viewMode,
} from "../../lib/settings";
import type { AccountStoreDto, CustomHomeInfoDto, InstanceArgsDto, InstanceInfoDto, JavaInfoDto, MotdDto, MotdSegmentDto, NewsItem, VersionInfoDto } from "../../lib/bindings";
import InstanceIcon from "../../components/InstanceIcon.vue";
import InstanceSelect from "../../components/InstanceSelect.vue";
import InstanceMetaPanel from "../../components/InstanceMetaPanel.vue";
import LaunchArgsPanel from "../../components/LaunchArgsPanel.vue";
import HomePage from "../../components/HomePage.vue";
import CustomHomePage from "../../components/CustomHomePage.vue";
import CustomExecPanel from "../../components/CustomExecPanel.vue";
import ProxyPanel from "../../components/ProxyPanel.vue";
import { useModpackStatus } from "../../lib/modpackTasks";
import ResourceDownloadBar from "../../components/ResourceDownloadBar.vue";
import { useResourceStatus } from "../../lib/resourceTasks";
import SplashScreen from "../../components/ui/SplashScreen.vue";
import MainTopbar from "./topbar/MainTopbar.vue";
import MainSidebar from "./sidebar/MainSidebar.vue";
import MainCtxMenu from "./ctxmenu/MainCtxMenu.vue";
import IconPickModal from "./IconPickModal.vue";
import { useInstanceDrag } from "../../composables/useInstanceDrag";
import { useMultiSelect } from "../../composables/useMultiSelect";
import { useFileDrop } from "../../composables/useFileDrop";
import type { ViewMode } from "../../lib/bindings";
import type { CtxMenuState, FeatureId, InstMenuAction } from "./types";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import NumberStepper from "../../components/ui/NumberStepper.vue";
import CollapsePanel from "../../components/ui/CollapsePanel.vue";
import { useWindowTitle } from "../../lib/titlebar";

// 注意：不能用顶层 await —— 会让 <script setup> 变成 async setup，
// App.vue 没有 <Suspense> 包裹，Vue 将不渲染该组件（窗口白屏）
// 系统窗口标题（任务栏 / Alt+Tab）走自定义命令 window_set_title：
// JS API setTitle 需要 core:window:allow-set-title 权限（capabilities 未放行）。
// 必须用 useWindowTitle 而不是在这里直接设一次：单窗口模式下本页被 KeepAlive 缓存，
// 从子窗口切回来不重新挂载，只设一次的话任务栏上会一直挂着子窗口的标题
useWindowTitle(() => t("winTitle.main"));

// ================= 基础状态 =================

import { closeSplash, splashError, splashVisible } from "../../lib/splash";

// ================= 数据 =================

const instances = ref<InstanceInfoDto[]>([]);
const javas = ref<JavaInfoDto[]>([]);
const selected = ref<InstanceInfoDto | null>(null);

// ================= 实例搜索 =================

const searchText = ref("");
const searching = computed(() => searchText.value.trim().length > 0);

function matchInst(inst: InstanceInfoDto, q: string): boolean {
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

function select(inst: InstanceInfoDto) {
  // 实例锁定：只能使用锁定的那个实例（对玩家静默拦截，不暴露"被锁定"这件事）
  if (lockActive.value && inst.uuid !== lockUuid.value) return;
  selected.value = inst;
  // 持久化当前选中实例到 gui_config.json（uuid 未变时内部跳过，不会重复写盘）
  setSelectedInstance(inst.uuid);
  newsActive.value = false;
  // 分组模式下展开所在分组
  if (mode.value === "group") {
    const key = groupKeyOf(inst.group);
    if (collapsedGroups.value[key]) {
      collapsedGroups.value = { ...collapsedGroups.value, [key]: false };
    }
  }
}

// ================= 游戏列表模式 =================

// 显示模式：共享状态（默认「列表」，用户改过后持久化到 gui_config.json）
const mode = viewMode;
const MODE_OPTIONS = computed(() => [
  { value: "group", label: t("mode.group"), icon: "folder" },
  { value: "grid", label: t("mode.grid"), icon: "grid" },
  { value: "list", label: t("mode.list"), icon: "list" },
]);

// 手动添加的空分组（来自 api.getGroups）
const extraGroups = ref<string[]>([]);

/** 空白分组名（后端默认分组的 key 是空格）统一视为默认分组 */
function groupKeyOf(group?: string | null): string {
  return group && group.trim() ? group : t("group.default");
}

const groups = computed(() => {
  const defaultKey = t("group.default");
  const map = new Map<string, InstanceInfoDto[]>();
  for (const inst of instances.value) {
    const key = groupKeyOf(inst.group);
    if (!map.has(key)) map.set(key, []);
    map.get(key)!.push(inst);
  }
  for (const g of extraGroups.value) {
    // 空白分组就是默认分组，不重复展示
    if (!g.trim() || map.has(g)) continue;
    map.set(g, []);
  }
  // 默认分组永远存在且置顶
  if (!map.has(defaultKey)) map.set(defaultKey, []);
  // 分组顺序：默认分组 → extraGroups（持久顺序，空白跳过）→ 其余按首次出现顺序
  const order = [
    defaultKey,
    ...extraGroups.value.filter((k) => k.trim() && k !== defaultKey),
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

// ================= 侧栏：收起 / 展开 =================

/** 是否启用侧栏过渡：默认关闭，只在用户主动收起 / 展开的这 300ms 内打开。
    常开的话，启动时按配置恢复侧栏宽度也会被播成一次滑动动画 */
const sidebarAnim = ref(false);
let sidebarAnimTimer: number | undefined;

/** 用户主动收起 / 展开侧栏（唯一会播放动画的入口） */
function collapseSidebar(collapsed: boolean) {
  sidebarAnim.value = true;
  setSidebarCollapsed(collapsed);
  window.clearTimeout(sidebarAnimTimer);
  sidebarAnimTimer = window.setTimeout(() => {
    sidebarAnim.value = false;
  }, 300);
}

// ================= 启动器主页（默认打开） =================

const newsActive = ref(true);
const newsLoading = ref(false);

/** 打开 / 关闭启动器主页（保留选中实例） */
function toggleNews() {
  newsActive.value = !newsActive.value;
}

// ---- 自定义主页面（服主导入的 zip 顶替内置主页；导入 / 删除在设置窗口「客户端设置」） ----
// 启用与否在 gui_config.client.customHome，已导入情况来自常驻压缩包，都由 status 现算

/** 自定义主页面状态（null = 还没拉到 / 出错 → 一律回落内置主页） */
const customHome = ref<CustomHomeInfoDto | null>(null);

/** 是否用自定义页面顶替内置主页：启用 + 已导入 + 有入口 URL，任一不满足都回落，不白屏 */
const useCustomHome = computed(
  () =>
    !!customHome.value?.enabled && !!customHome.value.installed && !!customHome.value.entryUrl,
);

/** 拉取自定义主页面状态（没导入过就是 installed=false，不报错） */
async function loadCustomHome() {
  try {
    customHome.value = await commands.customHome.status();
  } catch {
    customHome.value = null;
  }
}

// 切换选中实例后自动滚动到详情顶部
//
// 监听 uuid 而不是 selected 对象本身：改实例设置会触发 instance-change → 重拉实例列表
// （loadInstances），那里会把 selected 换成一个新对象（uuid 没变）。监听对象引用的话，
// 每次改设置都会把详情滚回顶部
const detailEl = ref<HTMLElement | null>(null);

watch(
  () => selected.value?.uuid,
  () => {
    nextTick(() => {
      if (detailEl.value) detailEl.value.scrollTop = 0;
    });
  },
);

/** 主页卡片显示的就是当前选中实例（持久化在 gui_config.json） */
function quickLaunch() {
  const inst = selected.value;
  if (inst) {
    select(inst);
    launch();
  }
}

const news = ref<NewsItem[]>([]);
const newsPage = ref(1);
const newsHasMore = ref(true);
let newsLoadedPage = 0; // 已成功加载的页码（0 = 未加载）

/** 拉取指定页的新闻（页码从 1 开始；翻页翻到空页时回退并禁用下一页） */
async function fetchNews(page: number) {
  if (newsLoading.value) return;
  newsLoading.value = true;
  try {
    const list = await api.getNews(page);
    if (list.length === 0 && page > 1) {
      newsHasMore.value = false;
      return;
    }
    newsHasMore.value = true;
    newsPage.value = page;
    news.value = list;
    newsLoadedPage = page;
  } catch (e) {
    console.warn("[news] 加载失败", e);
  } finally {
    newsLoading.value = false;
  }
}

/** 启动 / 核心加载完成后加载首页新闻（已加载则跳过） */
function loadNews() {
  if (newsLoadedPage) return;
  fetchNews(1);
}

function nextNewsPage() {
  if (!newsHasMore.value) return;
  fetchNews(newsPage.value + 1);
}

/** 打开新闻原文（系统浏览器） */
function openNews(url: string) {
  api.openUrl(url);
}

function prevNewsPage() {
  if (newsPage.value > 1) fetchNews(newsPage.value - 1);
}

// ================= 账户 =================

// 账户与当前账户来自共享存储（与账户窗口联动）
import {
  accounts as storeAccounts,
  currentAccount as storeCurrentAccount,
  setCurrentAccount,
  clearCurrentAccount,
} from "../../lib/accountStore";
import { listen } from "@tauri-apps/api/event";
import { LoadDone } from "../../lib/listens.ts";
import type { LoadState } from "../../lib/bindings";
const accounts = storeAccounts;
const currentAccount = storeCurrentAccount;

function onAccountChange(account: AccountStoreDto) {
  setCurrentAccount(account);
}

// ================= 启动状态 =================

const statusText = ref(t("launch.ready"));

function stateText(state: string): string {
  const key = `state.${state}`;
  const msg = t(key);
  return msg === key ? state : msg;
}

// ================= 启动进度（顶部进度条） =================
// 进度百分比由后端按 LaunchState 阶段随状态事件下发；无进度语义的状态走滚动条
const launchStage = ref("");
const launchPct = ref<number | null>(null);

const launchStageText = computed(() =>
  launchStage.value ? stateText(launchStage.value) : statusText.value,
);

// ================= 启动参数（经 IPC 读写核心实例配置） =================

/** 列表模式下的实例设置面板（版本/加载器/内存/Java + 启动参数，与分组模式一致） */
const settingsOpen = ref(false);
const execOpen = ref(false);
const serverOpen = ref(false);
const proxyOpen = ref(false);

/** 打开独立日志窗口（实例日志只在那个窗口里看，主页面不展示日志内容） */
function openLogWindow() {
  if (selected.value) openWindow("log", { uuid: selected.value.uuid });
}

function toggleSettings() {
  settingsOpen.value = !settingsOpen.value;
}
const argsMap = ref<Record<string, InstanceArgsDto>>({});
const argsLoaded = ref<Record<string, boolean>>({});

// 加载完成前先用骨架默认值占位（真实值随后端返回覆盖）
function argsOf(uuid: string): InstanceArgsDto {
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

async function loadArgs(uuid: string) {
  try {
    argsMap.value[uuid] = await api.getInstanceArgs(uuid);
    argsLoaded.value[uuid] = true;
  } catch {
    showToast(t("args.loadFailed"));
  }
}

// 切换选中实例后拉取该实例的启动参数
watch(
  () => selected.value?.uuid,
  (uuid) => {
    if (uuid && !argsLoaded.value[uuid]) loadArgs(uuid);
  },
  { immediate: true },
);

// 修改后防抖写回后端（每个按键都保存会产生大量写盘）
let argsSaveTimer: ReturnType<typeof setTimeout> | null = null;

function patchArgs(patch: Partial<InstanceArgsDto>) {
  if (selected.value) updateArgs({ ...argsOf(selected.value.uuid), ...patch });
}

function onServerIp(value: string) {
  patchArgs({ serverIp: value });
}

function onServerPort(v: number) {
  patchArgs({ serverPort: v });
}

function onServerJoin(checked: boolean) {
  patchArgs({ joinServer: checked });
}

// 累计游戏时间（TEMP 模拟数据，接入后端统计接口后移除）
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

function updateArgs(v: InstanceArgsDto) {
  const uuid = selected.value?.uuid;
  if (!uuid) return;
  argsMap.value[uuid] = v;
  if (argsSaveTimer) clearTimeout(argsSaveTimer);
  argsSaveTimer = setTimeout(() => {
    api.updateInstanceArgs(uuid, argsMap.value[uuid]).catch((e) => showToast(tErr(e)));
  }, 600);
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

/** 打开实例导出窗口（导出整合包） */
function openExportWindow() {
  if (!selected.value) return;
  openWindow("export", { uuid: selected.value.uuid });
}

function toggleMenu(id: string) {
  openMenu.value = openMenu.value === id ? null : id;
}

function onMenuPick(opt: { labelKey: string }) {
  openMenu.value = null;
  showToast(t("actions.wip", { name: t(opt.labelKey) }));
}

// ================= 元信息（版本 / 加载器 / 整合包 / 语言，合并进实例设置） =================

async function onMetaUpdate(patch: Partial<InstanceInfoDto>) {
  if (!selected.value) return;
  try {
    await api.updateInstance(selected.value.uuid, patch);
    await loadInstances();
  } catch (e) {
    showToast(tErr(e));
  }
}

/** 实例设置面板刷新版本列表（清空后端缓存重新拉取） */
function onVersionsRefreshed(list: VersionInfoDto[]) {
  versions.value = list;
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
function onInstClick(inst: InstanceInfoDto) {
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
      await api.launchGame(inst.uuid);
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
    // 空白分组名与默认分组（null）等价
    if (groupKeyOf(inst.group) !== groupKeyOf(target)) {
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
    if (groupKeyOf(inst.group) === name) {
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
function onInstContext(e: MouseEvent, inst: InstanceInfoDto) {
  if (multiSelect.value) {
    // 右键未勾选的实例时先加入选择
    if (!selectedIds.value.has(inst.uuid)) {
      const s = new Set(selectedIds.value);
      s.add(inst.uuid);
      selectedIds.value = s;
    }
    openCtxMenu(e, { kind: "multi" });
  } else {
    // 实例锁定时选不中别的实例，菜单动作又是按选中实例执行的，这里直接不给开
    if (lockActive.value && inst.uuid !== lockUuid.value) return;
    // 普通模式：先选中该实例，再弹出实例菜单
    select(inst);
    openCtxMenu(e, { kind: "instance", instance: inst });
  }
}

/** 实例右键菜单动作（与详情面板一致） */
function onInstMenuAction(id: InstMenuAction) {
  // 菜单一关 ctxMenu 就清空，右键目标要在这之前取出来（"修改图标"要用）
  const target = ctxMenu.value?.instance ?? null;
  closeCtxMenu();
  switch (id) {
    case "launch":
      launch();
      break;
    case "changeIcon":
      if (target) void pickIconFile(target);
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

/** "修改图标"的目标实例与已选图片（弹窗只在选到图之后才开） */
const iconPickInst = ref<InstanceInfoDto | null>(null);
const iconPickPath = ref("");

/**
 * 右键菜单「选择图片」：**直接开系统文件对话框**，不先弹一层自己的窗口
 *
 * 选到图才开截图弹窗（裁剪范围要用图片本身，那一步需要界面）；
 * 取消选择就什么都不发生。
 */
async function pickIconFile(inst: InstanceInfoDto) {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const path = await open({
      title: t("actions.pickImage"),
      multiple: false,
      filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "webp"] }],
    });
    if (typeof path !== "string") return;
    iconPickPath.value = path;
    iconPickInst.value = inst;
  } catch (e) {
    showToast(tErr(e));
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
    // 空白分组名与默认分组（null）等价
    if (inst && groupKeyOf(inst.group) !== groupKeyOf(target)) {
      await api.updateInstance(uuid, { group: target });
    }
  }
  closeCtxMenu();
  await loadInstances();
  const key = groupKeyOf(target);
  collapsedGroups.value = { ...collapsedGroups.value, [key]: false };
  showToast(t("multi.moved", { count: ids.length }));
}

/** 多选删除 */
const showMultiDelete = ref(false);
const multiDeleteBusy = ref(false);

// 多选删除进度：整体进度可知（已完成 / 总数），单实例内部不可知
const multiDone = ref(0);
const multiTotal = ref(0);

/** 多选删除总进度百分比（0-100） */
const multiPct = computed(() => {
  if (!multiTotal.value) return 0;
  return Math.min(100, Math.round((multiDone.value / multiTotal.value) * 100));
});

function onMultiDelete() {
  closeCtxMenu();
  showMultiDelete.value = true;
}

async function doMultiDelete() {
  multiDeleteBusy.value = true;
  const ids = [...selectedIds.value];
  multiDone.value = 0;
  multiTotal.value = ids.length;
  try {
    for (const uuid of ids) {
      await api.deleteInstance(uuid);
      multiDone.value += 1;
    }
  } finally {
    multiDeleteBusy.value = false;
    multiTotal.value = 0;
  }
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
      await api.launchGame(uuid);
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

/** 刷新实例列表并选中给定实例（添加 / 整合包安装完成后调用） */
async function adoptAddedInstance(uuid: string) {
  await Promise.all([loadInstances(), loadGroups()]);
  const inst = instances.value.find((i) => i.uuid === uuid);
  if (inst) select(inst);
}

/** 添加实例窗口创建成功后（跨窗口 storage 事件），刷新并选中新实例 */
async function onAddedInstanceStorage(e: StorageEvent) {
  if (e.key !== "mml.addedInstance" || !e.newValue) return;
  await adoptAddedInstance(e.newValue);
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
      // 跳转到资源管理窗口（当前实例已持久化在 gui_config.json，窗口自己读）
      openWindow("resource");
      break;
    case "addResource":
      // 跳转到添加资源窗口（当前实例已持久化在 gui_config.json，窗口自己读）
      openWindow("add_resource");
      break;
    case "viewLog":
      // 日志只在独立窗口里看（主页面不展示日志内容）
      openLogWindow();
      break;
    default:
      showToast(t("actions.wip", { name: t(ACTION_LABELS[id]) }));
  }
}

async function doRename() {
  if (!selected.value) return;
  renameBusy.value = true;
  try {
    const ok = await api.renameInstance(selected.value.uuid, renameName.value);
    if (ok) {
      showRename.value = false;
      await loadInstances();
      showToast(t("actions.rename"));
    }
  } catch (e) {
    // 重名等核心错误直接提示
    showToast(tErr(e));
  } finally {
    renameBusy.value = false;
  }
}

async function doDelete() {
  if (!selected.value) return;
  deleteBusy.value = true;
  try {
    await api.deleteInstance(selected.value.uuid);
    showDelete.value = false;
    selected.value = null;
    await Promise.all([loadInstances(), loadGroups()]);
  } catch (e) {
    // 删除失败（如文件被占用）直接提示
    showToast(tErr(e));
  } finally {
    deleteBusy.value = false;
  }
}

// ================= 启动器功能入口（顶部栏） =================

const features: Array<{ id: FeatureId; icon: string }> = [
  { id: "settings", icon: "gear" },
  { id: "stats", icon: "chart" },
  { id: "collect", icon: "star" },
  { id: "help", icon: "book" },
];

// ================= 添加分组 弹窗 =================

const versions = ref<VersionInfoDto[]>([]);

// ================= 拖拽整合包文件（逻辑见 composables/useFileDrop） =================

const { fileDragOver } = useFileDrop({
  versions,
  loadInstances,
});

const showAddGroup = ref(false);
const groupName = ref("");
const groupError = ref("");
const groupAdding = ref(false);

// ================= 服务器 MOTD 卡片 =================
// 客户端设置里配置 MOTD 显示地址，走内核 Server List Ping 查询；
// 悬浮卡片查全局地址，实例详情里的卡片查实例自己的服务器地址

const motdInfo = ref<MotdDto | null>(null);
const motdLoading = ref(false);
// 竞态保护：旧请求晚到不覆盖新结果
let motdSeq = 0;

async function refreshMotd() {
  const addr = clientConfig.value.motdServer.trim();
  if (!addr || motdLoading.value) return;
  motdLoading.value = true;
  const seq = ++motdSeq;
  try {
    const dto = await api.getMotd(addr);
    if (seq === motdSeq) motdInfo.value = dto;
  } catch {
    if (seq === motdSeq) motdInfo.value = null;
  } finally {
    if (seq === motdSeq) motdLoading.value = false;
  }
}

// 实例详情的 MOTD 卡片（查实例配置的服务器地址）
const instMotd = ref<MotdDto | null>(null);
const instMotdLoading = ref(false);
let instMotdSeq = 0;

async function refreshInstMotd() {
  const inst = selected.value;
  const ip = inst ? argsOf(inst.uuid).serverIp.trim() : "";
  if (!ip) {
    instMotd.value = null;
    return;
  }
  if (instMotdLoading.value) return;
  const port = inst ? argsOf(inst.uuid).serverPort || 25565 : 25565;
  instMotdLoading.value = true;
  const seq = ++instMotdSeq;
  try {
    const dto = await api.getMotd(`${ip}:${port}`);
    if (seq === instMotdSeq) instMotd.value = dto;
  } catch {
    if (seq === instMotdSeq) instMotd.value = null;
  } finally {
    if (seq === instMotdSeq) instMotdLoading.value = false;
  }
}

// 切换实例或改实例服务器地址后重新查询
watch(
  () => {
    const inst = selected.value;
    if (!inst) return "";
    const a = argsOf(inst.uuid);
    return `${inst.uuid}|${a.serverIp}|${a.serverPort}`;
  },
  () => void refreshInstMotd(),
);

/** MOTD 文字段渲染样式（颜色 + 加粗 / 斜体 / 下划线 / 删除线） */
function motdSegStyle(seg: MotdSegmentDto): Record<string, string> {
  const deco = [seg.underlined ? "underline" : "", seg.strikethrough ? "line-through" : ""]
    .filter(Boolean)
    .join(" ");
  return {
    color: seg.color,
    fontWeight: seg.bold ? "700" : "inherit",
    fontStyle: seg.italic ? "italic" : "inherit",
    textDecoration: deco || "none",
  };
}

/** 服务器图标（Base64 PNG → data URI，直接喂 <img>） */
function faviconOf(m: MotdDto | null): string | null {
  const f = m?.favicon;
  if (!f) return null;
  return f.startsWith("data:") ? f : `data:image/png;base64,${f}`;
}

// ---- 客户端设置（gui_config.client）：MOTD 卡片显示与自动刷新间隔 ----
const clientConfig = ref<ClientConfig>({
  motdCard: true,
  motdInterval: 15,
  loginLockOn: false,
  loginLock: [],
  autoJoin: false,
  autoJoinServer: "",
  motdServer: "",
  lockInstance: "",
  customHome: false,
});
const motdCardVisible = ref(true);
let motdTimer: number | null = null;

// ---- 实例锁定：只允许使用锁定的那个实例（设置窗口「客户端设置 → 实例锁定」） ----
/** 锁定的实例 uuid（空 = 未锁定） */
const lockUuid = computed(() => clientConfig.value.lockInstance);
/** 锁定是否生效：uuid 非空且实例还在（实例被删掉后走 lockMissing 的错误页） */
const lockActive = computed(
  () => !!lockUuid.value && instances.value.some((i) => i.uuid === lockUuid.value),
);
/** 实例列表是否已加载过：区分「还在加载」与「确实没有这个实例」，避免启动瞬间闪错误页 */
const instancesLoaded = ref(false);
/** 锁定的实例已不存在（整合包作者的实例目录被删 / 改名）：主窗口整块换成错误页 */
const lockMissing = computed(
  () =>
    instancesLoaded.value &&
    !!lockUuid.value &&
    !instances.value.some((i) => i.uuid === lockUuid.value),
);
/** 锁定生效时强制按列表模式渲染（只覆盖渲染，不写回 gui_config，解锁后自动回到原模式） */
const effectiveMode = computed<ViewMode>(() => (lockActive.value ? "list" : mode.value));

// 锁定生效时把选中实例压到锁定实例：切换入口已隐藏，若还停在别处（比如上次退出时选的是另一个）
// 就会启动错的实例。不走 select()——那会关掉启动器主页。
watch(
  () => (lockActive.value ? lockUuid.value : ""),
  (uuid) => {
    if (!uuid || selected.value?.uuid === uuid) return;
    const inst = instances.value.find((i) => i.uuid === uuid);
    if (!inst) return;
    selected.value = inst;
    setSelectedInstance(inst.uuid);
  },
);

// ---- 登录方式锁定：账户选择列表只显示锁定类型，当前账户被滤掉就取消选择 ----
/** 锁定的登录类型（总开关关闭或列表为空 = 不过滤） */
const lockedTypes = computed(
  () =>
    new Set(
      clientConfig.value.loginLockOn
        ? clientConfig.value.loginLock.map((e) => e.ty)
        : [],
    ),
);

/** 过滤后的账户列表（传给顶栏选择器） */
const visibleAccounts = computed(() => {
  const lock = lockedTypes.value;
  const list = accounts.value;
  return lock.size ? list.filter((a) => lock.has(a.authType)) : list;
});

watch([lockedTypes, currentAccount], ([lock, acc]) => {
  if (lock.size && acc && !lock.has(acc.authType)) void clearCurrentAccount();
});

function restartMotdTimer() {
  if (motdTimer !== null) clearInterval(motdTimer);
  motdTimer = null;
  if (!motdCardVisible.value) return;
  const sec = clientConfig.value.motdInterval;
  if (sec >= 5) {
    motdTimer = window.setInterval(refreshMotd, sec * 1000);
  }
}

/** 设置窗口保存 client 后广播：实时更新卡片显隐与刷新间隔 */
function onClientConfig(c: ClientConfig) {
  clientConfig.value = c;
  // 没配地址就没东西可查，卡片不显示
  motdCardVisible.value = c.motdCard && !!c.motdServer.trim();
  restartMotdTimer();
  void refreshMotd();
  // 「启用自定义主页面」开关在这里；enabled 由 status 现算，顺手回拉一次
  void loadCustomHome();
}

// ================= 事件订阅 =================

const unlistens: Array<() => void> = [];

// 整合包安装任务：进度本身已交给标题栏上的指示器（ModpackTitleIndicator），
// 这里只借它做一件事 —— 安装成功后刷新实例列表并选中刚装好的那个（adoptAddedInstance）。
// 所以不取 status，只用 init 返回的订阅。
const { init: initModpackStatus } = useModpackStatus(false, adoptAddedInstance);

// 资源下载任务（添加资源窗口关闭后，进度条迁到主窗口显示）
const { status: resourceStatus, init: initResourceStatus } = useResourceStatus(false);

async function subscribeEvents() {
  const fns = await Promise.all([
    onLaunchState((e) => {
      if (e.uuid !== selected.value?.uuid) return;
      statusText.value = stateText(e.state);
      launchStage.value = e.state;
      launchPct.value = e.progress ?? null;
    }),
    onGameExit((e) => {
      if (e.uuid !== selected.value?.uuid) return;
      statusText.value =
        e.code === 0 ? t("launch.exited") : t("launch.exitedCode", { code: e.code });
      launchStage.value = "";
      launchPct.value = null;
      const inst = instances.value.find((i) => i.uuid === e.uuid);
      if (inst) inst.running = false;
    }),
    onLaunchError((e) => {
      if (e.uuid && e.uuid !== selected.value?.uuid) return;
      statusText.value = t("launch.failed");
      launchStage.value = "";
      launchPct.value = null;
      if (e.uuid) {
        const inst = instances.value.find((i) => i.uuid === e.uuid);
        if (inst) inst.running = false;
      }
    }),
    onInstanceChange(() => {
      loadInstances();
      loadGroups();
    }),
    // 自定义主页面导入 / 删除（该动作不改 gui_config，client-config-change 不会发）
    onCustomHomeChange(() => {
      void loadCustomHome();
    }),
    // Java 列表变更（添加 / 删除 / 配置加载完成）→ 重新拉取
    onJavaChange(() => {
      loadJava();
    }),
  ]);
  unlistens.push(...fns);
}

// ================= 初始化 =================

// 只初始化一次：load-done 事件与下面的状态查询兜底可能先后到达
let inited = false;

async function doInit() {
  if (inited) return;
  inited = true;
  try {
    await Promise.all([loadInstances(), loadGroups(), loadJava(), loadCustomHome()]);
    restoreSelection();
    closeSplash();
    loadVersions();
    loadNews();
  } catch (e) {
    // 初始化失败：关闭启动页并显示错误页
    closeSplash(String(e));
  }
}

/** 初始化失败反馈：打开 GitHub Issues */
function openFeedback() {
  api.openUrl("https://github.com/Coloryr/ModernColoryrMinecraftLauncher/issues");
}

// ================= 数据加载 =================

async function loadInstances() {
  instances.value = await api.getInstances();
  instancesLoaded.value = true;
  // 保留当前选中；不自动选中实例（默认停在启动器主页）
  if (selected.value) {
    const now = instances.value.find((i) => i.uuid === selected.value?.uuid);
    if (now) selected.value = now;
    else selected.value = null;
  }
}

/** 启动时按 gui_config 恢复上次选中的实例。
 *  故意不走 select()：那会置 newsActive=false 关掉启动器主页，而默认应停在主页。 */
function restoreSelection() {
  const uuid = selectedInstance.value;
  if (!uuid) return;
  selected.value = instances.value.find((i) => i.uuid === uuid) ?? null;
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
  selected.value.running = true;
  statusText.value = t("launch.launching");
  launchStage.value = "";
  launchPct.value = null;
  try {
    await api.launchGame(uuid);
  } catch (e) {
    if (selected.value) selected.value.running = false;
    statusText.value = t("launch.failed");
    showToast(t("launch.error", { msg: tErr(e) }));
  }
}

function onPickInstance(uuid: string) {
  const inst = instances.value.find((i) => i.uuid === uuid);
  if (inst) select(inst);
}

// ================= 添加实例（独立窗口） =================

function openAdd() {
  // 实例锁定生效时禁止新建实例（入口本就随侧边栏 / 禁用按钮不可达，这里兜底）
  if (lockActive.value) return;
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
  initModpackStatus().then((unlisten) => unlistens.push(unlisten));
  initResourceStatus().then((unlisten) => unlistens.push(unlisten));
  document.addEventListener("click", onDocClick);
  document.addEventListener("keydown", onDocKeyDown);
  window.addEventListener("storage", onAddedInstanceStorage);
  // 客户端设置：初始加载 + 设置窗口保存后实时跟随
  try {
    const cfg = await loadGuiConfig();
    if (cfg?.client) onClientConfig(cfg.client);
  } catch {
    /* 浏览器环境忽略 */
  }
  onClientConfigChange(onClientConfig).then((unlisten) => unlistens.push(unlisten));
});

onUnmounted(() => {
  unlistens.forEach((fn) => fn());
  if (motdTimer !== null) clearInterval(motdTimer);
  document.removeEventListener("click", onDocClick);
  document.removeEventListener("keydown", onDocKeyDown);
  window.removeEventListener("storage", onAddedInstanceStorage);
});

// 后端核心加载完成事件：决定关闭启动页（进入主界面）还是显示错误页
listen<LoadState>(LoadDone, async (data) => {
  const state = data.payload;
  if (state.ok) {
    doInit();
  } else {
    closeSplash(state.error || t("init.failed"));
  }
});

// 兜底：启动很快时 load-done 可能在页面监听注册前就已发出而被错过，
// 挂载后主动查一次核心状态（已加载则直接进主界面）
onMounted(async () => {
  try {
    const state = await getLoadState();
    if (state.ok) {
      doInit();
    }
  } catch {
    /* 浏览器环境忽略，等 load-done 事件 */
  }
});
</script>

<template>
  <div class="main-window">
    <!-- ===== 启动画面 / 初始化失败错误页（SplashScreen 组件） ===== -->
    <SplashScreen
      v-if="splashVisible || splashError"
      :splash-visible="splashVisible"
      :splash-error="splashError"
      @feedback="openFeedback"
    />

    <!-- ===== 主界面 ===== -->
    <template v-else>
      <!-- 顶部栏 -->
      <MainTopbar
        :features="features"
        :news-active="newsActive"
        :current-account="currentAccount"
        :accounts="visibleAccounts"
        @toggle-news="toggleNews"
        @feature="openWindow"
        @update:account="onAccountChange"
      />

      <!-- 整合包安装进度不再内嵌在这里：它改成了标题栏上的指示器 + 弹窗
           （ModpackTitleIndicator / ModpackPopup），否则这条卡片会一直占着内容区的高度、把列表顶下去。
           状态订阅在各处自己拿（标题栏指示器自包含），弹窗由 App.vue 渲染 -->

      <!-- 资源下载进度（添加资源窗口关闭后迁到这里显示） -->
      <ResourceDownloadBar
        v-if="resourceStatus?.tasks.length && !resourceStatus.windowOpen"
        :status="resourceStatus"
        class="mpbar-in-main"
      />

      <!-- 启动进度（启动期间显示：阶段文本 + 进度条；未知阶段走滚动条） -->
      <div v-if="selected?.running" class="mpbar-in-main launch-progress">
        <div class="launch-progress-head">
          <span class="launch-spinner"></span>
          <span class="launch-stage">{{ launchStageText }}</span>
          <span v-if="launchPct !== null" class="launch-pct">{{ launchPct }}%</span>
        </div>
        <div class="launch-progress-track">
          <div
            v-if="launchPct !== null"
            class="launch-progress-bar"
            :style="{ width: launchPct + '%' }"
          ></div>
          <div v-else class="launch-progress-bar rolling"></div>
        </div>
      </div>

      <!-- 主体 -->
      <main class="main" :class="{ 'side-right': sidebarSide === 'Right', 'sidebar-anim': sidebarAnim }">
        <!-- 多选模式浮动工具栏 -->
        <div v-if="multiSelect" class="multi-bar" @contextmenu.prevent @click.stop>
          <span class="multi-count">{{ t("multi.selected", { count: selectedIds.size }) }}</span>
          <button class="multi-btn" @click="openMoveFromBar($event)">{{ t("multi.moveGroup") }}</button>
          <button class="multi-btn danger" @click="onMultiDelete">{{ t("multi.delete") }}</button>
          <button class="multi-btn" @click="multiLaunch">{{ t("multi.launch") }}</button>
          <span class="multi-sep"></span>
          <button class="multi-exit" @click="exitMultiSelect">{{ t("multi.exit") }}</button>
        </div>

        <!-- 锁定的实例已不存在（整合包作者的实例被删 / 改名）：整块换成错误页，顶部栏保留 -->
        <template v-if="lockMissing">
          <section class="lock-error">
            <div class="lock-error-badge">!</div>
            <p class="lock-error-text">{{ t("launch.instanceLockError") }}</p>
          </section>
        </template>

        <!-- 空实例：强制打开启动器主页，主页内融合空状态引导（隐藏实例分组） -->
        <template v-else-if="instances.length === 0">
          <!-- 自定义主页面：整块铺满内容区（不留四周内边距，也不给底部留白） -->
          <section v-if="useCustomHome" class="custom-home-page">
            <CustomHomePage :entry-url="customHome?.entryUrl ?? ''" />
          </section>
          <section v-else class="news-page">
            <HomePage
              :items="news"
              :loading="newsLoading"
              @refresh="fetchNews(newsPage)"
              :page="newsPage"
              :has-more="newsHasMore"
              @prev="prevNewsPage"
              @next="nextNewsPage"
              @open="openNews"
              :current-instance="null"
              :empty="true"
              @add-instance="openAdd"
              @add-account="openWindow('account')"
              @add-java="openWindow('settings')"
            />
          </section>
        </template>

        <!-- 列表模式：启动器主页 ↔ 实例列表。两者布局角色一致（flex:1 纵向），
             直接作为进出动画的两个子节点，内容不用包一层 -->
        <template v-else-if="effectiveMode === 'list'">
          <Transition name="view-swap" mode="out-in">
            <!-- 自定义主页面：与列表模式一样铺满内容区（自己滚动，不留内边距） -->
            <section v-if="newsActive && useCustomHome" key="custom-home" class="custom-home-page">
              <CustomHomePage :entry-url="customHome?.entryUrl ?? ''" />
            </section>

            <section v-else-if="newsActive" key="home" class="news-page">
            <HomePage
              :items="news"
              :loading="newsLoading"
              @refresh="fetchNews(newsPage)"
              :page="newsPage"
              :has-more="newsHasMore"
              @prev="prevNewsPage"
              @next="nextNewsPage"
              @open="openNews"
              :current-instance="selected"
              @select="(inst: InstanceInfoDto) => select(inst)"
              @quick-launch="quickLaunch"
              @back="newsActive = false"
            />
            </section>

            <section v-else key="list" class="list-mode">
            <!-- 实例锁定生效时视图固定为列表，模式切换控件整块隐藏（只留个空工具栏会很怪） -->
            <div v-if="!lockActive" class="list-toolbar">
              <SegmentedTabs
                :model-value="mode"
                :options="MODE_OPTIONS"
                @update:model-value="setViewMode($event as ViewMode)"
              />
            </div>

            <InstanceIcon
              :name="selected?.name ?? '—'"
              :uuid="selected?.uuid ?? '0'"
              :size="120"
            />
            <h2 class="list-title">{{ selected?.name ?? t("launch.selectInstance") }}</h2>

            <!-- 实例锁定生效：只有这一个实例可用，切换入口整块隐藏 -->
            <InstanceSelect
              v-if="!lockActive"
              :instances="instances"
              :model-value="selected?.uuid ?? null"
              @update:model-value="onPickInstance"
              @add="openAdd"
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
              <!-- 实例设置：含启动参数（与分组模式下的设置面板一致） -->
              <BaseButton :disabled="!selected" @click="toggleSettings">
                ⚙ {{ t("detail.settings") }}
              </BaseButton>
              <!-- 实例日志：主页面不展示日志内容，直接开独立日志窗口 -->
              <BaseButton :disabled="!selected" @click="openLogWindow">
                📄 {{ t("detail.logs") }}
              </BaseButton>
            </div>
            <!-- 设置面板整体展开/收起；宽度与居中由外层槽位承担，与原 .list-args 的占位一致 -->
            <CollapsePanel :open="settingsOpen && !!selected" class="list-settings-wrap">
              <div v-if="selected" class="list-args list-settings">
                <InstanceMetaPanel
                  :instance="selected"
                  :versions="versions"
                  :locked="lockActive"
                  @update="onMetaUpdate"
                  @refreshed="onVersionsRefreshed"
                />
                <LaunchArgsPanel
                  :args="argsOf(selected.uuid)"
                  :javas="javas"
                  :locked="lockActive"
                  @update:args="updateArgs"
                />
              </div>
            </CollapsePanel>
            </section>
          </Transition>
        </template>

        <!-- 模式：游戏分组 / 平铺（列表模式已在上面的分支处理，这里兜底） -->
        <template v-else>
          <!-- 侧栏槽位：宽度在展开(300px) / 收起(24px) 之间过渡，动画期间内容被裁掉而不被压扁 -->
          <div class="sidebar-slot" :class="{ collapsed: sidebarCollapsed }">
            <MainSidebar
              :mode="effectiveMode"
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
              @update:mode="setViewMode($event)"
              @update:search-text="searchText = $event"
              @add-instance="openAdd"
              @add-group="showAddGroup = true"
              @collapse="collapseSidebar(true)"
            />

            <!-- 展开把手：绝对定位在槽位外缘，收起时淡入（不占位，避免展开瞬间内容跳动） -->
            <button
              class="sidebar-expand"
              v-tip="t('sidebar.expand')"
              @click="collapseSidebar(false)"
            >›</button>
          </div>

          <!-- 右侧内容区：实例详情 / 启动器主页（进出都走动画） -->
          <section ref="detailEl" class="detail">
            <!-- 自定义主页面走自己的 flex 布局：.detail 带 padding 且可滚动，直接塞 iframe 时
                 height:100% 解析不出高度（父元素没有确定高度），会把页面压扁；这里单独包一层
                 去掉 padding 的撑满容器，让 iframe 铺满整个内容区 -->
            <Transition name="view-swap" mode="out-in">
              <div
                v-if="newsActive && useCustomHome"
                key="custom-home"
                class="custom-home-fill"
              >
                <CustomHomePage :entry-url="customHome?.entryUrl ?? ''" />
              </div>

              <HomePage
                v-else-if="newsActive"
                key="home"
                :items="news"
                :loading="newsLoading"
                @refresh="fetchNews(newsPage)"
                :page="newsPage"
                :has-more="newsHasMore"
                @prev="prevNewsPage"
                @next="nextNewsPage"
                @open="openNews"
                :current-instance="selected"
                @select="(inst: InstanceInfoDto) => select(inst)"
                @quick-launch="quickLaunch"
                @back="newsActive = false"
              />

              <div v-else key="detail" class="detail-content">
              <!-- 按实例 uuid 做 key：切换实例时整块详情走进出动画 -->
              <Transition name="view-swap" mode="out-in">
              <div v-if="selected" :key="selected.uuid" class="detail-instance">
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
                    <button class="icon-btn" v-tip="t('actions.openFolder')" @click="onAction('openFolder')">
                      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M3 7a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V7z" />
                      </svg>
                    </button>
                    <button class="icon-btn" v-tip="t('actions.editConfig')" @click="onAction('editConfig')">
                      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3" />
                        <path d="M1 14h6M9 8h6M17 16h6" />
                      </svg>
                    </button>
                    <button class="icon-btn" v-tip="t('actions.rename')" @click="onAction('rename')">
                      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M17 3a2.85 2.85 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z" />
                      </svg>
                    </button>
                    <button class="icon-btn danger" v-tip="t('actions.delete')" @click="onAction('delete')">
                      <svg viewBox="0 0 24 24" width="15" height="15" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6M10 11v6M14 11v6" />
                      </svg>
                    </button>
                  </div>
                </div>

                <!-- 实例操作（导出直接开窗 / 生成为二级菜单） -->
                <div class="action-grid">
                  <button class="action-btn" @click="openExportWindow">
                    {{ t("actions.export") }}
                  </button>
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
                    <Transition name="drop">
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
                    </Transition>
                  </div>
                </div>

                <!-- 实例设置（直接展示：版本 / 加载器 / 整合包 / 语言 / 内存 / Java / 启动参数） -->
                <div class="inline-settings">
                  <InstanceMetaPanel
                    :instance="selected"
                    :versions="versions"
                    :locked="lockActive"
                    @update="onMetaUpdate"
                    @refreshed="onVersionsRefreshed"
                  />
                  <LaunchArgsPanel
                    :args="argsOf(selected.uuid)"
                    :javas="javas"
                    :locked="lockActive"
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
                  <CollapsePanel :open="execOpen">
                    <CustomExecPanel :args="argsOf(selected.uuid)" @update:args="updateArgs" />
                  </CollapsePanel>
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
                  <CollapsePanel :open="serverOpen">
                    <div class="server-config">
                      <!-- 自动加入服务器设置：地址（可带 `:端口`，不写用 25565）+ 启动时加入 -->
                      <div class="server-row">
                        <span class="server-label">{{ t("server.ip") }}</span>
                        <input
                          class="field-input grow"
                          :value="argsOf(selected.uuid).serverIp"
                          :placeholder="t('server.ipPlaceholder')"
                          spellcheck="false"
                          @input="onServerIp(($event.target as HTMLInputElement).value)"
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

                      <!-- MOTD 展示：只在**填了服务器地址**时出现。
                           没填时整块不显示 —— 原来会渲染一张只有兜底文案的卡片
                           （"M²L 服务器 / 欢迎来到 M²L 服务器大厅"），看着像查询成功了，其实没查 -->
                      <div v-if="argsOf(selected.uuid).serverIp.trim()" class="motd-card">
                        <img v-if="faviconOf(instMotd)" class="motd-icon" :src="faviconOf(instMotd)!" alt="" />
                        <div v-else class="motd-icon">MC</div>
                        <div class="motd-info">
                          <div class="motd-name">{{ instMotd?.ip || argsOf(selected.uuid).serverIp }}</div>
                          <div class="motd-text">
                            <template v-if="instMotd && instMotd.state === 'ok' && instMotd.segments.length">
                              <span v-for="(seg, i) in instMotd.segments" :key="i" :style="motdSegStyle(seg)">{{ seg.text }}</span>
                            </template>
                            <span v-else-if="instMotd">{{ instMotd.message || t("server.offline") }}</span>
                            <!-- 有地址但结果还没回来（这一块只在填了地址时才渲染） -->
                            <span v-else>{{ t("server.refreshing") }}</span>
                          </div>
                          <div class="motd-meta">
                            <template v-if="instMotd && instMotd.state === 'ok'">
                              <span class="motd-online">{{
                                t("server.players", { now: instMotd.playersOnline ?? 0, max: instMotd.playersMax ?? 0 })
                              }}</span>
                              <span class="sep">·</span>
                              <span>{{ instMotd.version || t("server.unknown") }}</span>
                              <span class="sep">·</span>
                              <span>{{ t("server.ping", { ms: instMotd.ping }) }}</span>
                            </template>
                            <span v-else-if="instMotdLoading" class="motd-online">{{ t("server.refreshing") }}</span>
                          </div>
                        </div>
                      </div>
                    </div>
                  </CollapsePanel>
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
                  <CollapsePanel :open="proxyOpen">
                    <ProxyPanel :args="argsOf(selected.uuid)" @update:args="updateArgs" />
                  </CollapsePanel>
                </div>
              </div>
              </Transition>

              <!-- 未选中实例：多选提示 / 空提示（保持是 .detail-content 的直接 flex 子节点，
                   两者都靠 flex:1 垂直居中） -->
              <template v-if="!selected">
              <template v-if="multiSelect">
                <div class="multi-detail">
                  <div class="multi-detail-icon">☑</div>
                  <h2>{{ t("multi.detailTitle") }}</h2>
                  <p>{{ t("multi.detailDesc", { count: selectedIds.size }) }}</p>
                </div>
              </template>
              <div v-else class="placeholder">{{ t("detail.selectHint") }}</div>
              </template>
              </div>
            </Transition>
          </section>
        </template>

      </main>

      <!-- 服务器 MOTD 悬浮卡片（启动器下方；客户端设置里配置地址与开关） -->
      <div v-if="motdCardVisible" class="motd-float">
        <button
          class="motd-refresh"
          v-tip="t('server.refresh')"
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
            :class="{ spin: motdLoading }"
          >
            <path d="M21 12a9 9 0 1 1-2.64-6.36M21 3v6h-6" />
          </svg>
        </button>
        <img v-if="faviconOf(motdInfo)" class="motd-float-icon" :src="faviconOf(motdInfo)!" alt="" />
        <div v-else class="motd-float-icon">MC</div>
        <div class="motd-float-info">
          <div class="motd-float-name">{{ motdInfo?.ip || t("server.name") }}</div>
          <div class="motd-float-text">
            <template v-if="motdInfo && motdInfo.state === 'ok' && motdInfo.segments.length">
              <span v-for="(seg, i) in motdInfo.segments" :key="i" :style="motdSegStyle(seg)">{{ seg.text }}</span>
            </template>
            <span v-else-if="motdInfo">{{ motdInfo.message || t("server.offline") }}</span>
            <span v-else>{{ t("server.refreshing") }}</span>
          </div>
          <div class="motd-float-meta">
            <template v-if="motdInfo && motdInfo.state === 'ok'">
              <span class="motd-online">{{
                t("server.players", { now: motdInfo.playersOnline ?? 0, max: motdInfo.playersMax ?? 0 })
              }}</span>
              <span class="sep">·</span>
              <span>{{ motdInfo.version || t("server.unknown") }}</span>
              <span class="sep">·</span>
              <span>{{ t("server.ping", { ms: motdInfo.ping }) }}</span>
            </template>
            <span v-else-if="motdLoading" class="motd-online">{{ t("server.refreshing") }}</span>
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
    <BaseModal v-if="showRename && selected" :title="t('actions.renameTitle')" :closable="false" @close="showRename = false">
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
    <BaseModal v-if="showDelete && selected" :title="t('actions.deleteTitle')" :closable="false" @close="!deleteBusy && (showDelete = false)">
      <p class="delete-tip">{{ t("actions.deleteConfirm", { name: selected.name }) }}</p>

      <!-- 删除进度（整目录挪回收站无法取得真实进度，显示滚动动画条） -->
      <div v-if="deleteBusy" class="delete-progress">
        <div class="delete-progress-bar rolling"></div>
      </div>
      <p v-if="deleteBusy" class="delete-progress-text">{{ t("actions.deleting") }}</p>

      <div class="modal-actions">
        <BaseButton :disabled="deleteBusy" @click="showDelete = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" :disabled="deleteBusy" @click="doDelete">
          {{ deleteBusy ? t("actions.deleting") : t("actions.delete") }}
        </BaseButton>
      </div>
    </BaseModal>

    <!-- ===== 多选删除确认 ===== -->
    <BaseModal v-if="showMultiDelete" :title="t('multi.deleteTitle')" :closable="false" @close="!multiDeleteBusy && (showMultiDelete = false)">
      <p class="delete-tip">{{ t("multi.deleteConfirm", { count: selectedIds.size }) }}</p>

      <!-- 删除进度（真实进度：已完成实例数 / 总数） -->
      <div v-if="multiDeleteBusy" class="delete-progress">
        <div class="delete-progress-bar" :style="{ width: multiPct + '%' }"></div>
      </div>
      <p v-if="multiDeleteBusy" class="delete-progress-text">
        {{ t("multi.deleting", { done: multiDone, total: multiTotal }) }}
      </p>

      <div class="modal-actions">
        <BaseButton :disabled="multiDeleteBusy" @click="showMultiDelete = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" :disabled="multiDeleteBusy" @click="doMultiDelete">
          {{ multiDeleteBusy ? t("actions.deleting") : t("multi.delete") }}
        </BaseButton>
      </div>
    </BaseModal>

    <!-- ===== 修改实例图标（右键实例 → 选择图片 → 截图范围）===== -->
    <IconPickModal
      v-if="iconPickInst"
      :uuid="iconPickInst.uuid"
      :name="iconPickInst.name"
      :path="iconPickPath"
      @close="((iconPickInst = null), (iconPickPath = ''))"
    />

    <!-- ===== 删除分组确认 ===== -->
    <BaseModal v-if="showDeleteGroup" :title="t('group.delete')" :closable="false" @close="showDeleteGroup = false">
      <p class="delete-tip">{{ t("group.deleteConfirm", { name: deleteGroupName, count: deleteGroupCount }) }}</p>

      <div class="modal-actions">
        <BaseButton @click="showDeleteGroup = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" :disabled="deleteGroupBusy" @click="doDeleteGroup">
          {{ t("group.delete") }}
        </BaseButton>
      </div>
    </BaseModal>

    <!-- ===== 添加分组弹窗 ===== -->
    <BaseModal v-if="showAddGroup" :title="t('group.addGroup')" :closable="false" @close="showAddGroup = false">
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

/* 顶部整合包安装进度条（窗口内容有自身内边距，这里只加外边距） */
.mpbar-in-main {
  margin: 4px 12px 0;
}

/* 顶部启动进度条 */
.launch-progress {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 10px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
}

.launch-progress-head {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text);
}

.launch-spinner {
  flex-shrink: 0;
  width: 12px;
  height: 12px;
  border: 2px solid var(--border);
  border-top-color: var(--blue, var(--accent));
  border-radius: 50%;
  animation: launch-spin 0.8s linear infinite;
}

@keyframes launch-spin {
  to {
    transform: rotate(360deg);
  }
}

.launch-stage {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.launch-pct {
  flex-shrink: 0;
  font-size: 12px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.launch-progress-track {
  height: 6px;
  border-radius: 3px;
  background: var(--bg-side);
  overflow: hidden;
}

.launch-progress-bar {
  height: 100%;
  border-radius: 3px;
  background: var(--blue, var(--accent));
  transition: width 0.3s ease;
}

.launch-progress-bar.rolling {
  width: 40%;
  animation: launch-slide 1.1s ease-in-out infinite;
}

@keyframes launch-slide {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(350%);
  }
}

/* 删除实例进度条（删除弹窗内） */
.delete-progress {
  height: 6px;
  border-radius: 3px;
  background: var(--bg-side);
  overflow: hidden;
  margin-bottom: 14px;
}

/* 真实进度条（多选删除：宽度由已完成实例数决定） */
.delete-progress-bar {
  height: 100%;
  border-radius: 3px;
  background: var(--accent);
  transition: width 0.25s ease;
}

/* 滚动动画条（单实例删除：整目录挪回收站拿不到真实进度） */
.delete-progress-bar.rolling {
  width: 40%;
  animation: delete-slide 1.1s ease-in-out infinite;
}

@keyframes delete-slide {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(350%);
  }
}

.delete-progress-text {
  font-size: 12px;
  color: var(--text-dim);
  text-align: center;
  margin-bottom: 14px;
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

/* 侧栏槽位：展开 300px ↔ 收起 24px。
   收窄时用 overflow 裁掉侧栏内容（而非把内容压扁），停靠模式下内容区因此平滑跟随。
   宽度过渡只在用户主动收起 / 展开时挂上（.main.sidebar-anim，见 MainWindow 的 collapseSidebar）——
   常开的话，启动时按配置恢复槽位宽度也会被播成一次动画 */
.sidebar-slot {
  position: relative;
  display: flex;
  flex-shrink: 0;
  width: 300px; /* 与 MainSidebar 的 .sidebar 同宽 */
  overflow: hidden;
}

.main.sidebar-anim .sidebar-slot {
  transition: width 0.22s ease;
}

.sidebar-slot.collapsed {
  width: 24px;
}

/* 侧栏贴右时，让侧栏钉在槽位右缘，收窄时从左侧裁掉（视觉上向右滑出） */
.main.side-right .sidebar-slot {
  justify-content: flex-end;
}

/* 展开把手：绝对定位挂在槽位外缘，不参与占位，收起时淡入 */
.sidebar-expand {
  position: absolute;
  top: 0;
  bottom: 0;
  left: 0;
  width: 24px;
  border: none;
  background: var(--bg-side);
  color: var(--text-dim);
  font-size: 18px;
  line-height: 1;
  cursor: pointer;
  border-right: 1px solid var(--border);
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.15s;
}

.main.side-right .sidebar-expand {
  left: auto;
  right: 0;
  border-right: none;
  border-left: 1px solid var(--border);
}

.sidebar-slot.collapsed .sidebar-expand {
  opacity: 1;
  pointer-events: auto;
}

.sidebar-expand:hover {
  background: var(--bg-hover);
  color: var(--accent);
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

/* 视图切换动画：启动器主页 ↔ 实例列表 / 实例详情，以及实例详情之间切换实例。
   淡入淡出 + 轻微上收（out-in 保证同一时刻只有一个子节点，避免 flex:1 的两者叠在一起） */
.view-swap-enter-active,
.view-swap-leave-active {
  transition: opacity 0.18s ease, transform 0.18s ease;
}

.view-swap-enter-from,
.view-swap-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

/* 实例详情分支的包裹层：复刻 .detail 的纵向 flex 间距，避免包一层后间距丢失 */
.detail-content {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

/* 单个实例的详情：按 uuid 做 key，切换实例时触发动画；同样复刻纵向 flex 间距 */
.detail-instance {
  display: flex;
  flex-direction: column;
  gap: 16px;
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

/* 自定义主页面占位块：整块铺满内容区，内边距与底部留白都不给（页面自己负责排版与滚动）。
   高度必须走 flex 而不能只靠 height:100%：.detail 只有主轴上的确定尺寸，
   子元素 height:100% 解析不出高度，会把 iframe 压扁 */
.custom-home-page {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
}

/* 兜底分支里的自定义主页面：.detail 带 padding 且自身可滚动，要在它内部撑满，
   得有一层确定高度的容器把 padding 抵消掉 */
.custom-home-fill {
  flex: 1;
  display: flex;
  flex-direction: column;
  min-height: 0;
  margin: -16px -26px -130px;
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
  /* 低于账户下拉菜单（200）：菜单展开时悬浮卡片不能压住列表 */
  z-index: 150;
  display: flex;
  align-items: center;
  gap: 12px;
  max-width: 560px;
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
  width: 60px;
  height: 60px;
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
  /* MOTD 可能多行，保留 \n 换行 */
  white-space: pre-wrap;
  word-break: break-all;
  overflow: hidden;
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
  /* MOTD 可能多行，保留 \n 换行 */
  white-space: pre-wrap;
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
  width: 28px;
  height: 28px;
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
  height: 28px;
  padding: 0 10px;
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
  /* 菜单贴按钮左缘，展开时以左上角为原点 */
  transform-origin: top left;
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

/* 端口步进器不许被压扁：这一行是 nowrap 的 flex，步进器作为 flex item 默认可收缩，
   会被挤到只剩一条缝（"端口"右边的框看着就不对了） */
.server-row .server-port {
  flex-shrink: 0;
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
  max-width: 640px;
}

/* 列表模式下设置面板的折叠槽位：.list-mode 是 align-items: center 的 flex 列，
   折叠容器接管了原 .list-args 的 flex 子项身份，宽度上限与居中需由它承担 */
.list-settings-wrap {
  width: 100%;
  max-width: 640px;
}

/* 列表模式的实例设置面板：与分组模式的 .inline-settings 同款排列 */
.list-settings {
  display: flex;
  flex-direction: column;
  gap: 16px;
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

/* ----- 实例锁定失效（锁定的实例已不存在）----- */

/* 整块内容区被替换：居中放置一个警示徽标 + 文案，其余入口随内容一起消失 */
.lock-error {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 16px;
  padding: 40px 28px 130px;
  min-width: 0;
}

.lock-error-badge {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 30px;
  font-weight: 700;
  color: var(--red);
  background: var(--bg-card);
  border: 2px solid var(--red);
}

.lock-error-text {
  font-size: 15px;
  font-weight: 600;
  color: var(--text-dim);
  text-align: center;
}

/* ----- 列表模式（下拉框选中实例） ----- */

.list-mode {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  /* 内容不满一屏时整块垂直居中；展开设置面板后内容超高时退回顶部对齐，
     这样第一项仍然可见、能正常往下滚（`center` 会把溢出部分顶到滚动范围之外） */
  justify-content: safe center;
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
  border: 1px solid var(--border);
  background: var(--bg-raised);
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
