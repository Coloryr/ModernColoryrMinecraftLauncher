// 界面设置（侧栏位置 / 收起状态 / 列表显示模式 / 皮肤与头像显示），持久化到 localStorage + gui_config.json
import { ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import {
  loadGuiConfig,
  normalizeHeadType,
  normalizeSkinDisplay,
  normalizeViewMode,
  saveGuiConfig,
  type HeadType,
  type SidebarSide,
  type SkinDisplay,
  type ViewMode,
} from "./guiConfig";
import { bumpImageVersion } from "./accountImages";
import { isTauri } from "../windows/windowManager";
import { SkinConfigChange } from "./listens";
export type { SidebarSide, ViewMode, SkinDisplay, HeadType } from "./guiConfig";

const SIDE_KEY = "mml.sidebarSide";
const COLLAPSE_KEY = "mml.sidebarCollapsed";
const VIEW_KEY = "mml.viewMode";
const SELECT_KEY = "mml.selectedInstance";

export const sidebarSide = ref<SidebarSide>(
  localStorage.getItem(SIDE_KEY) === "Right" ? "Right" : "Left",
);

// 侧栏是否收起：默认收起（只有显式存过 "0" 才是展开）
export const sidebarCollapsed = ref(localStorage.getItem(COLLAPSE_KEY) !== "0");

// 实例列表显示模式：默认列表；用户改过后用用户的值
export const viewMode = ref<ViewMode>(normalizeViewMode(localStorage.getItem(VIEW_KEY)));

export function setSidebarSide(side: SidebarSide) {
  sidebarSide.value = side;
  localStorage.setItem(SIDE_KEY, side);
  saveGuiConfig({ mainWindow: { sidebarSide: side } });
}

export function setSidebarCollapsed(v: boolean) {
  sidebarCollapsed.value = v;
  localStorage.setItem(COLLAPSE_KEY, v ? "1" : "0");
  saveGuiConfig({ mainWindow: { sidebarCollapsed: v } });
}

export function setViewMode(v: ViewMode) {
  viewMode.value = v;
  localStorage.setItem(VIEW_KEY, v);
  saveGuiConfig({ mainWindow: { viewMode: v } });
}

/** 当前选中实例 uuid（空串 = 未选中） */
export const selectedInstance = ref(localStorage.getItem(SELECT_KEY) ?? "");

function mirrorSelected(uuid: string) {
  if (uuid) localStorage.setItem(SELECT_KEY, uuid);
  else localStorage.removeItem(SELECT_KEY);
}

/** 启动引导：从 gui_config 恢复选中实例（只同步内存 + localStorage，不回写配置文件） */
export function restoreSelectedInstance(uuid: string) {
  selectedInstance.value = uuid;
  mirrorSelected(uuid);
}

/** 选中实例变化：写 localStorage + gui_config.json（uuid 未变则直接返回，避免无谓的 IPC 与落盘） */
export function setSelectedInstance(uuid: string) {
  if (selectedInstance.value === uuid) return;
  selectedInstance.value = uuid;
  mirrorSelected(uuid);
  saveGuiConfig({ mainWindow: { selectedInstance: uuid } });
}

// ---- 皮肤显示模式（账户界面皮肤预览：2D TypeA / 2D TypeB / 3D 模型） ----
const SKIN_KEY = "mml.skinDisplay";

export const skinDisplay = ref<SkinDisplay>(
  normalizeSkinDisplay(localStorage.getItem(SKIN_KEY)),
);

export function setSkinDisplay(v: SkinDisplay) {
  skinDisplay.value = v;
  localStorage.setItem(SKIN_KEY, v);
  saveGuiConfig({ skinDisplay: v });
  bumpImageVersion();
}

/** 启动恢复：只同步内存与镜像，不回写 gui_config.json */
export function restoreSkinDisplay(v: SkinDisplay) {
  skinDisplay.value = v;
  localStorage.setItem(SKIN_KEY, v);
}

// ---- 头像显示模式（渲染端按 head 配置出图，改完自动生效） ----
const HEAD_TYPE_KEY = "mml.headType";
const HEAD_X_KEY = "mml.headX";
const HEAD_Y_KEY = "mml.headY";

export const headType = ref<HeadType>(normalizeHeadType(localStorage.getItem(HEAD_TYPE_KEY)));
// 3D 旋转模式默认角度：x 15 / y 65（没存过或存了非法值时用默认）
export const headX = ref(readAngle(HEAD_X_KEY, 15));
export const headY = ref(readAngle(HEAD_Y_KEY, 65));

function readAngle(key: string, dflt: number): number {
  const raw = localStorage.getItem(key);
  if (raw === null) return dflt;
  const v = Number(raw);
  return Number.isFinite(v) ? v : dflt;
}

/** 设置头像渲染模式（3D 旋转模式附带 X / Y 角度） */
export function setHeadConfig(type: HeadType, x = headX.value, y = headY.value) {
  headType.value = type;
  headX.value = x;
  headY.value = y;
  localStorage.setItem(HEAD_TYPE_KEY, type);
  localStorage.setItem(HEAD_X_KEY, String(x));
  localStorage.setItem(HEAD_Y_KEY, String(y));
  saveGuiConfig({ head: { headType: type, x, y } });
  bumpImageVersion();
}

/** 启动恢复：只同步内存与镜像，不回写 gui_config.json */
export function restoreHeadConfig(type: HeadType, x: number, y: number) {
  headType.value = type;
  headX.value = x;
  headY.value = y;
  localStorage.setItem(HEAD_TYPE_KEY, type);
  localStorage.setItem(HEAD_X_KEY, String(x));
  localStorage.setItem(HEAD_Y_KEY, String(y));
}

// ---- 跨窗口同步 ----
// 每个窗口是独立 JS 上下文，ref 不会自动同步：设置窗口改了皮肤 / 头像模式后，
// 后端在 saveGuiConfig 时广播 skin-config-change，本窗口重读配置并让所有
// 账户图片带新版本号重取（后端渲染缓存键含这两项配置，取到的就是新模式渲染图）
if (isTauri()) {
  void listen(SkinConfigChange, async () => {
    const cfg = await loadGuiConfig();
    if (!cfg) return;
    restoreSkinDisplay(cfg.skinDisplay);
    restoreHeadConfig(cfg.head.headType, cfg.head.x, cfg.head.y);
    bumpImageVersion();
  });
}

