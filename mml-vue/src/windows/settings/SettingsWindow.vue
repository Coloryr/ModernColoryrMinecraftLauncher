<script setup lang="ts">
// 启动器设置窗口：左侧标签导航 + 右侧内容区
// 标签：界面（含窗口设置）/ 皮肤与头像 / 网络与下载 / 游戏启动 / Java
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import NumberStepper from "../../components/ui/NumberStepper.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseSwitch from "../../components/ui/BaseSwitch.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import CollapsePanel from "../../components/ui/CollapsePanel.vue";
import { t, locale, setLocale, tErr } from "../../lib/i18n";
import {
  commands,
  type JavaInfoDto,
  type JavaImportProgressDto,
  type NetworkSettingDto,
  type RunArgSettingDto,
  type WindowSettingDto,
} from "../../lib/bindings";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { SettingsJavaProgress } from "../../lib/listens";
import { multiWindow, setMultiWindow, isTauri, openWindow } from "../windowManager";
import {
  animations,
  setAnimations,
  bgImage,
  setBgImage,
  setBgImageFromOriginal,
  resizeBg,
  bgOpacity,
  setBgOpacity,
  bgBlur,
  setBgBlur,
  bgNativeSize,
  bgLoading,
  bgSource,
} from "../../lib/appearance";
import {
  sidebarSide,
  setSidebarSide,
  skinDisplay,
  setSkinDisplay,
  headType,
  headX,
  headY,
  setHeadConfig,
  type SkinDisplay,
  type HeadType,
  type SidebarSide,
} from "../../lib/settings";
import { theme, setTheme, accent, setAccent, customAccent, setCustomAccent, ACCENTS, type Theme } from "../../lib/theme";
import { setFontFamily, fontFamily } from "../../lib/fonts";
import { showToast } from "../../lib/toast";
import { loadGuiConfig, saveGuiConfig, type ClientConfig } from "../../lib/guiConfig";
import { typeLabelKey } from "../../lib/accountStore";
import { imageBase, imageVersion } from "../../lib/accountImages";

const inTauri = isTauri();

/** 设置页头像样例：内置纤细皮肤按当前头像配置渲染（配置变化经 imageVersion 重取） */
const headPreviewUrl = computed(() =>
  imageBase.value
    ? `${imageBase.value}/head/preview/0?v=${imageVersion.value}`
    : "",
);

/** 设置页皮肤样例：内置纤细皮肤按当前皮肤显示模式渲染 */
const skinPreviewUrl = computed(() =>
  imageBase.value
    ? `${imageBase.value}/skin/preview/0/slim?v=${imageVersion.value}`
    : "",
);

// ================= 标签导航 =================

type SettingsTab = "ui" | "skin" | "network" | "launch" | "java" | "client";

const TABS: Array<{ id: SettingsTab; icon: string }> = [
  { id: "ui", icon: "palette" },
  { id: "java", icon: "coffee" },
  { id: "network", icon: "download" },
  { id: "launch", icon: "play" },
  { id: "skin", icon: "user" },
  { id: "client", icon: "gear" },
];

const tab = ref<SettingsTab>("ui");

// ---- 窗口模式 ----
const windowMode = ref(multiWindow.value ? "Multi" : "Single");
const side = ref(sidebarSide.value);

// ---- 主题 ----
const themeValue = ref(theme.value);

// ---- 字体 ----
const fonts = ref<string[]>([]);
const fontPick = ref(fontFamily.value);
const fontsLoading = ref(false);

// ---- 网络与下载设置 ----
const network = ref<NetworkSettingDto | null>(null);
// ---- 自定义 DNS（DoH）逐行编辑 ----
// 编辑中只改本地草稿，失焦 / 回车才落盘；清空一行 = 删除该行（保留旧 textarea 过滤空行的语义）
const dnsLines = ref<string[]>([]);
/** 草稿 → network（过滤空行） */
function commitDns() {
  if (!network.value) return;
  network.value.dns.https = dnsLines.value.map((s) => s.trim()).filter((s) => s.length > 0);
  dnsLines.value = [...network.value.dns.https];
}
function setDnsLine(i: number, v: string) {
  dnsLines.value[i] = v;
}
function removeDnsLine(i: number) {
  dnsLines.value.splice(i, 1);
  commitDns();
  applyNetwork();
}
function addDnsLine() {
  commitDns();
  dnsLines.value.push("");
  applyNetwork();
}

// ---- 客户端设置（gui_config.client，即改即存） ----
const client = ref<ClientConfig>({
  motdCard: true,
  motdInterval: 15,
  loginLockOn: false,
  loginLock: [],
  autoJoin: false,
  autoJoinServer: "",
  motdServer: "",
});

/** 登录方式锁定的候选（复用账户添加类型的文案） */
const LOGIN_TYPES = ["offline", "microsoft", "littleskin", "selflittleskin", "authlib", "nide8"] as const;

/** 登录类型标签（key 用 accountStore.ACCOUNT_TYPES 的标准映射，别手拼——
 *  LittleSkin 中间的 S 是大写的，手拼会变成不存在的 typeLittleskin） */
function loginTypeLabel(ty: string): string {
  return t(typeLabelKey(ty));
}

// 登录方式锁定：可添加的条目列表（类型 + 登录模型名 + 服务器信息）
const addLockType = ref<string>("offline");
const addLockName = ref("");
const addLockServer = ref("");
/** 需要服务器信息的类型（外置登录 / 自定义皮肤站 = 服务器地址，统一通行证 = 服务器 ID）；
 *  这些类型可重复添加，条目带「登录模型名字」，添加账户时直接下拉选择 */
const SERVER_LOCK_TYPES = ["authlib", "selflittleskin", "nide8"];
const addLockHasServer = computed(() => SERVER_LOCK_TYPES.includes(addLockType.value));
/** 带服务器的类型始终可加（可添加不同地址）；其余类型加过就不再出现在下拉里 */
const addLockOptions = computed(() =>
  LOGIN_TYPES.filter(
    (ty) =>
      (SERVER_LOCK_TYPES as readonly string[]).includes(ty) ||
      !client.value.loginLock.some((e) => e.ty === ty),
  ),
);
/** 非空 = 服务器输入框红框 + 提示文字 */
const lockServerError = ref("");

function validateLockServer(): boolean {
  if (!addLockHasServer.value) {
    lockServerError.value = "";
    return true;
  }
  const v = addLockServer.value.trim();
  if (!v) {
    lockServerError.value = t("winSettings.loginLockServerRequired");
    return false;
  }
  if (client.value.loginLock.some((e) => e.ty === addLockType.value && e.server === v)) {
    lockServerError.value = t("winSettings.loginLockServerDup");
    return false;
  }
  lockServerError.value = "";
  return true;
}

/** 服务器输入框的占位文案 */
const lockServerPlaceholder = computed(() =>
  addLockType.value === "nide8"
    ? t("winSettings.lockNide8ServerHint")
    : addLockType.value === "selflittleskin"
      ? t("winSettings.lockSelfServerHint")
      : t("winSettings.lockAuthlibServerHint"),
);

/** 添加一个锁定条目 */
function addLock() {
  const ty = addLockType.value;
  if (!ty) return;
  if (!validateLockServer()) return;
  const name = addLockName.value.trim();
  if (addLockHasServer.value && !name) {
    lockServerError.value = t("winSettings.loginLockNameRequired");
    return;
  }
  client.value.loginLock = [
    ...client.value.loginLock,
    { ty, name, server: addLockServer.value.trim() },
  ];
  addLockName.value = "";
  addLockServer.value = "";
  lockServerError.value = "";
  applyClient();
}

/** 移除一个锁定条目 */
function removeLock(i: number) {
  client.value.loginLock.splice(i, 1);
  applyClient();
}

function applyClient() {
  void saveGuiConfig({ client: { ...client.value } });
}

/** 服务器地址：自动进服与 MOTD 显示共用一个地址（两个配置字段保持一致） */
const serverAddr = computed({
  get: () => client.value.motdServer,
  set: (v: string) => {
    client.value.motdServer = v;
    client.value.autoJoinServer = v;
    applyClient();
  },
});

// ---- 游戏启动设置 ----
const javaList = ref<JavaInfoDto[]>([]);
/** 全局启动参数（core RunArgObj）+ 游戏窗口设置（core WindowSettingObj），整体读写 */
const run = ref<RunArgSettingDto | null>(null);
const win = ref<WindowSettingDto | null>(null);
/** JVM 环境变量逐条编辑（键 / 值两框；落盘时合成回 run.jvmEnv，与 core splitn(2,'=') 语义一致） */
const envLines = ref<Array<{ key: string; value: string }>>([]);

/** 环境变量添加 / 删除 */
function addEnvLine() {
  envLines.value.push({ key: "", value: "" });
}
function removeEnvLine(idx: number) {
  envLines.value.splice(idx, 1);
}
/** 把逐条编辑的环境变量合成回 run.jvmEnv（保存前调用；只留键非空的行；值不变时不触发监听） */
function commitEnvLines() {
  if (!run.value) return;
  const merged = envLines.value
    .filter((l) => l.key.trim())
    .map((l) => `${l.key.trim()}=${l.value}`)
    .join("\n");
  if (run.value.jvmEnv !== merged) run.value.jvmEnv = merged;
}

/** 启动设置即改即存（防抖合并连续输入）；内存冲突时不落盘只提示 */
let launchSaveTimer: number | null = null;
/** 初次加载完成前不触发自动保存 */
let launchLoaded = false;

watch([run, win, envLines], () => {
  if (!launchLoaded) return;
  if (launchSaveTimer !== null) clearTimeout(launchSaveTimer);
  launchSaveTimer = window.setTimeout(() => {
    launchSaveTimer = null;
    void applyLaunch();
  }, 500);
}, { deep: true });

async function applyLaunch() {
  if (!run.value || !win.value) return;
  if (run.value.minMemory > run.value.maxMemory) {
    showToast(t("winSettings.memoryConflict"));
    return;
  }
  try {
    commitEnvLines();
    await commands.settings.saveLaunch(run.value, win.value!);
  } catch (e) {
    showToast(tErr(e));
  }
}
const scanning = ref(false);
/** 扫描文件夹进行中（同扫描一样弹窗提示） */
const scanningDir = ref(false);

/** 压缩包导入进度事件：更新进度条（命令返回后由 importingJava 收尾） */
let unlistenJavaProgress: UnlistenFn | null = null;

onMounted(async () => {
  if (!inTauri) return;
  const cfg = await loadGuiConfig();
  if (cfg?.client) client.value = { ...cfg.client };
  unlistenJavaProgress = await listen<JavaImportProgressDto>(SettingsJavaProgress, (e) => {
    importProgress.value = e.payload;
  });
  fontsLoading.value = true;
  fonts.value = await commands.settings.getSystemFonts().catch(() => []);
  fontsLoading.value = false;
  network.value = await commands.settings.getNetwork().catch(() => null);
  if (network.value) dnsLines.value = [...network.value.dns.https];
  const launch = await commands.settings.getLaunch().catch(() => null);
  if (launch) {
    javaList.value = launch.javaList;
    run.value = launch.run;
    win.value = launch.window;
    envLines.value = launch.run.jvmEnv
      .split("\n")
      .filter((l) => l.trim())
      .map((l) => {
        const i = l.indexOf("=");
        return i < 0 ? { key: l.trim(), value: "" } : { key: l.slice(0, i).trim(), value: l.slice(i + 1) };
      });
    launchLoaded = true;
  }
});

onUnmounted(() => {
  unlistenJavaProgress?.();
});

function onModeChange(v: string) {
  windowMode.value = v;
  setMultiWindow(v === "Multi");
}

function onLangChange(v: string) {
  setLocale(v === "en_us" ? "en_us" : "zh_cn");
}

function onSideChange(v: string) {
  side.value = v === "Right" ? "Right" : "Left";
  setSidebarSide(side.value);
}

function onThemeChange(v: string) {
  const th = v as Theme;
  themeValue.value = th;
  setTheme(th);
}

// ---- 背景图：本地选图走 <input type=file>，纯前端读成 dataURL（不落盘、不动后端） ----
const bgFileInput = ref<HTMLInputElement | null>(null);

/** 选择图片：Tauri 用系统文件对话框（拿到真实路径回填输入框，走统一加载管线），
 *  浏览器回退到文件输入 */
async function pickBgImage() {
  if (inTauri) {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const picked = await open({
      title: t("winSettings.bgPick"),
      multiple: false,
      filters: [{ name: "Image", extensions: ["png", "jpg", "jpeg", "webp"] }],
    });
    if (typeof picked === "string" && picked) {
      bgSourceInput.value = picked;
      await loadBgSource();
    }
    return;
  }
  bgFileInput.value?.click();
}

function onBgFile(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  input.value = ""; // 允许重复选同一张图
  if (!file) return;
  bgSourceInput.value = file.name;
  const reader = new FileReader();
  reader.onload = () => setBgImageFromOriginal(String(reader.result));
  reader.readAsDataURL(file);
}

// ---- 图片地址输入（本地文件路径或网址） ----
// 回显已设置的来源（打开窗口时不再是空的）；清除背景时输入框跟着清空
const bgSourceInput = ref("");
watch(bgSource, (v) => (bgSourceInput.value = v), { immediate: true });

async function loadBgSource() {
  const source = bgSourceInput.value.trim();
  if (!source) return;
  // 后端加载 → 缩放 → 落盘并记入配置（失败时 setBgImageFromOriginal 内部提示）；
  // 输入框里的地址保留不清空，方便看到当前用的是哪张图
  await setBgImageFromOriginal(source);
}

// ---- 原始大小：拖动只改草稿，点「应用」才触发后端重新缩放 ----
const bgSizeDraft = ref(bgNativeSize.value);

function applyBgSize() {
  void resizeBg(bgSizeDraft.value);
}

async function saveNetwork() {
  if (!network.value) return;
  try {
    await commands.settings.saveNetwork(network.value);
    showToast(t("winSettings.saved"));
  } catch (e) {
    showToast(tErr(e));
  }
}

/// 非代理网络设置即改即存（成功不弹提示，免得每次切换都打扰；失败才报）
async function applyNetwork() {
  if (!network.value) return;
  try {
    await commands.settings.saveNetwork(network.value);
  } catch (e) {
    showToast(tErr(e));
  }
}

// ---- Java ----
/** 按类型收起的分组（默认全展开） */
const collapsedTypes = ref<Record<string, boolean>>({});
/** Java 列表按发行类型（JDK / JRE）分组，保持首次出现顺序 */
const javaGroups = computed(() => {
  const groups: { type: string; items: JavaInfoDto[] }[] = [];
  const index = new Map<string, JavaInfoDto[]>();
  for (const j of javaList.value) {
    let items = index.get(j.javaType);
    if (!items) {
      items = [];
      index.set(j.javaType, items);
      groups.push({ type: j.javaType, items });
    }
    items.push(j);
  }
  return groups;
});

function toggleType(type: string) {
  collapsedTypes.value[type] = !collapsedTypes.value[type];
}
function typeOpen(type: string) {
  return !collapsedTypes.value[type];
}

/** 手动添加：名字 + 路径（main_add_java 落全局注册表） */
const newJavaName = ref("");
const newJavaPath = ref("");
const addingJava = ref(false);

async function refreshJava() {
  const launch = await commands.settings.getLaunch().catch(() => null);
  if (launch) javaList.value = launch.javaList;
}

async function browseJava() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    title: t("winSettings.javaAdd"),
    multiple: false,
    filters: [{ name: "Java", extensions: ["exe"] }],
  });
  if (typeof picked !== "string") return;
  newJavaPath.value = picked;
  // 让后端探测该 Java，自动回填识别到的名字（失败则用户手填）
  try {
    const name = await commands.settings.detectJava(picked);
    if (name) newJavaName.value = name;
  } catch {
    /* 探测失败：名字留给用户手填 */
  }
}

async function addJava() {
  const name = newJavaName.value.trim();
  const path = newJavaPath.value.trim();
  if (!name || !path) return;
  addingJava.value = true;
  try {
    await commands.settings.addJava(name, path);
    await refreshJava();
    newJavaName.value = "";
    newJavaPath.value = "";
    showToast(t("winSettings.javaAdded", { name }));
  } catch (e) {
    showToast(tErr(e));
  } finally {
    addingJava.value = false;
  }
}

async function scanJava() {
  scanning.value = true;
  try {
    javaList.value = await commands.settings.scanJava();
    showToast(t("winSettings.javaScanDone", { n: javaList.value.length }));
  } catch (e) {
    showToast(tErr(e));
  } finally {
    scanning.value = false;
  }
}

/** 删除全部：先弹确认 */
const confirmRemoveAll = ref(false);

async function removeAllJava() {
  try {
    await commands.settings.removeAllJava();
    await refreshJava();
  } catch (e) {
    showToast(tErr(e));
  } finally {
    confirmRemoveAll.value = false;
  }
}

/** 压缩包导入（zip / 7z 等，解包后注册其中的 Java；进度经 settings-java-progress 事件上报） */
const importingJava = ref(false);
const importProgress = ref<JavaImportProgressDto | null>(null);

const importPercent = computed(() => {
  const p = importProgress.value;
  if (!p || p.total <= 0) return 0;
  return Math.min(100, Math.round((p.now / p.total) * 100));
});

async function importJavaArchive() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    title: t("winSettings.javaImport"),
    multiple: false,
    filters: [{ name: "Archive", extensions: ["zip", "7z", "gz", "tar"] }],
  });
  if (typeof picked !== "string") return;
  importingJava.value = true;
  importProgress.value = null;
  try {
    // 名字不填，由后端从压缩包内容识别
    await commands.settings.importJava(null, picked);
    await refreshJava();
  } catch (e) {
    showToast(tErr(e));
  } finally {
    importingJava.value = false;
    importProgress.value = null;
  }
}

/** 扫描指定文件夹（含子目录） */
async function scanJavaDir() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    title: t("winSettings.javaScanDir"),
    multiple: false,
    directory: true,
  });
  if (typeof picked !== "string") return;
  scanningDir.value = true;
  try {
    const found = await commands.settings.scanJavaDir(picked);
    await refreshJava();
    // 后端返回该文件夹下识别到的单个 Java，null 表示没扫到
    if (found) showToast(t("winSettings.javaScanDirDone", { name: found.name }));
    else showToast(t("winSettings.javaScanDirNone"));
  } catch (e) {
    showToast(tErr(e));
  } finally {
    scanningDir.value = false;
  }
}

/** 下载 Java：打开独立下载窗口（发行类型 / 主版本 / 系统 / 架构筛选） */
function downloadJava() {
  openWindow("java_download");
}

async function removeJava(name: string) {
  try {
    await commands.settings.removeJava(name);
    await refreshJava();
  } catch (e) {
    showToast(tErr(e));
  }
}
</script>

<template>
  <WindowFrame :title="t('features.settings')" body-fill @close="$emit('close')">
    <div class="settings-layout">
      <!-- 左侧标签导航 -->
      <nav class="tab-rail">
        <button
          v-for="item in TABS"
          :key="item.id"
          class="tab-item"
          :class="{ active: tab === item.id }"
          @click="tab = item.id"
        >
          <span class="tab-icon">
            <!-- 调色板：界面 -->
            <svg v-if="item.icon === 'palette'" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="13.5" cy="6.5" r=".5" /><circle cx="17.5" cy="10.5" r=".5" /><circle cx="8.5" cy="7.5" r=".5" /><circle cx="6.5" cy="12.5" r=".5" />
              <path d="M12 2C6.5 2 2 6.5 2 12s4.5 10 10 10c.926 0 1.648-.746 1.648-1.688 0-.437-.18-.835-.437-1.125-.29-.289-.438-.652-.438-1.125a1.64 1.64 0 0 1 1.668-1.668h1.996c3.051 0 5.555-2.503 5.555-5.554C21.965 6.012 17.461 2 12 2z" />
            </svg>
            <!-- 人像：皮肤与头像 -->
            <svg v-else-if="item.icon === 'user'" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
              <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
              <circle cx="12" cy="7" r="4" />
            </svg>
            <!-- 下载箭头：网络与下载 -->
            <svg v-else-if="item.icon === 'download'" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <path d="m7 10 5 5 5-5" />
              <path d="M12 15V3" />
            </svg>
            <!-- 播放：游戏启动 -->
            <svg v-else-if="item.icon === 'play'" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="m6 4 14 8-14 8V4z" />
            </svg>
            <!-- 咖啡杯：Java -->
            <svg v-else-if="item.icon === 'coffee'" viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M17 8h1a4 4 0 1 1 0 8h-1" />
              <path d="M3 8h14v9a4 4 0 0 1-4 4H7a4 4 0 0 1-4-4Z" />
              <line x1="6" x2="6" y1="2" y2="4" /><line x1="10" x2="10" y1="2" y2="4" /><line x1="14" x2="14" y1="2" y2="4" />
            </svg>
            <!-- 齿轮：客户端设置 -->
            <svg v-else viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z" />
              <circle cx="12" cy="12" r="3" />
            </svg>
          </span>
          <span class="tab-text">
            <span class="tab-title">{{ t(`winSettings.tab.${item.id}`) }}</span>
            <span class="tab-desc">{{ t(`winSettings.tabDesc.${item.id}`) }}</span>
          </span>
        </button>
      </nav>

      <!-- 右侧内容区：每个标签一个面板，独立滚动 -->
      <div class="tab-content">
        <!-- 当前页大标题 -->
        <header class="content-head">
          <h2 class="content-title">{{ t(`winSettings.tab.${tab}`) }}</h2>
          <p class="content-sub">{{ t(`winSettings.tabDesc.${tab}`) }}</p>
        </header>

        <!-- ================= 界面（含窗口设置） ================= -->
        <section v-if="tab === 'ui'" class="panel">
          <h3 class="group-title">{{ t("winSettings.secGeneral") }}</h3>

          <!-- 语言 / 字体：两个下拉并排一行 -->
          <div class="general-row">
            <div class="general-col">
              <label class="field-label" style="margin-top: 0">{{ t("winSettings.language") }}</label>
              <select class="field-select lang-select" :value="locale" @change="onLangChange(($event.target as HTMLSelectElement).value)">
                <option value="zh_cn">简体中文</option>
                <option value="en_us">English</option>
              </select>
            </div>
            <div class="general-col">
              <label class="field-label" style="margin-top: 0">{{ t("winSettings.font") }}</label>
              <div class="font-row">
                <select v-model="fontPick" class="field-select font-select" @change="setFontFamily(fontPick)">
                  <option value="">{{ t("winSettings.fontDefault") }}</option>
                  <option v-for="f in fonts" :key="f" :value="f">{{ f }}</option>
                </select>
              </div>
            </div>
          </div>
          <p class="field-desc" style="margin-top: 8px">{{ t("winSettings.fontDesc") }}</p>
          <p v-if="fontsLoading" class="hint">{{ t("winSettings.fontLoading") }}</p>

          <!-- 界面动画开关（无底色卡片，与周边文字排版对齐） -->
          <div class="switch-row" style="margin-top: 14px">
            <div class="switch-text">
              <span class="switch-label">{{ t("winSettings.animations") }}</span>
              <span class="switch-state">{{ t("winSettings.animationsDesc") }}</span>
            </div>
            <BaseSwitch :model-value="animations" @update:model-value="setAnimations" />
          </div>

          <h3 class="group-title">{{ t("winSettings.secTheme") }}</h3>

          <!-- 主题：跟随系统 / 浅色 / 深色 -->
          <label class="field-label">{{ t("winSettings.theme") }}</label>
          <SegmentedTabs
            :model-value="themeValue"
            :options="[
              { value: 'System', label: t('winSettings.themeSystem') },
              { value: 'Light', label: t('winSettings.themeLight') },
              { value: 'Dark', label: t('winSettings.themeDark') },
            ]"
            @update:model-value="onThemeChange"
          />

          <!-- 强调色 -->
          <label class="field-label" style="margin-top: 16px">{{ t("winSettings.accent") }}</label>
          <p class="field-desc">{{ t("winSettings.accentDesc") }}</p>
          <div class="accent-list">
            <button
              v-for="a in ACCENTS"
              :key="a.id"
              class="accent-swatch"
              :class="{ active: accent === a.id }"
              :style="{ background: a.color }"
              :title="t(`winSettings.accent.${a.id}`)"
              @click="setAccent(a.id)"
            >
              <svg
                v-if="accent === a.id"
                viewBox="0 0 24 24"
                width="16"
                height="16"
                fill="none"
                :stroke="a.check"
                stroke-width="3.5"
                stroke-linecap="round"
                stroke-linejoin="round"
              >
                <path d="m5 13 4 4L19 7" />
              </svg>
            </button>

            <!-- 自定义：彩虹色块，点击弹出系统取色器 -->
            <label
              class="accent-swatch accent-custom"
              :class="{ active: accent === 'custom' }"
              :title="t('winSettings.accent.custom')"
            >
              <svg
                v-if="accent === 'custom'"
                viewBox="0 0 24 24"
                width="16"
                height="16"
                fill="none"
                stroke="#fff"
                stroke-width="3.5"
                stroke-linecap="round"
                stroke-linejoin="round"
                style="filter: drop-shadow(0 0 2px rgba(0, 0, 0, 0.6))"
              >
                <path d="m5 13 4 4L19 7" />
              </svg>
              <input type="color" :value="customAccent" @input="setCustomAccent(($event.target as HTMLInputElement).value)" />
            </label>
          </div>

          <h3 class="group-title">{{ t("winSettings.secWindow") }}</h3>

          <!-- 两态设置用 Switch 开关（无底色卡片，与界面动画行一致） -->
          <div class="switch-row">
            <div class="switch-text">
              <span class="switch-label">{{ t("winSettings.windowMode") }}</span>
              <span class="switch-state">{{ windowMode === "Multi" ? t("winSettings.multi") : t("winSettings.single") }}</span>
            </div>
            <BaseSwitch :model-value="windowMode === 'Multi'" @update:model-value="(v) => onModeChange(v ? 'Multi' : 'Single')" />
          </div>
          <p v-if="inTauri" class="hint">{{ t("winSettings.windowModeRestart") }}</p>

          <!-- 侧栏是主窗口的东西：单独一组 -->
          <h3 class="group-title">{{ t("winSettings.secMainWindow") }}</h3>
          <label class="field-label">{{ t("winSettings.sidebar") }}</label>
          <SegmentedTabs
            :model-value="side"
            :options="[
              { value: 'Left', label: t('winSettings.sidebarLeft') },
              { value: 'Right', label: t('winSettings.sidebarRight') },
            ]"
            @update:model-value="(v) => onSideChange(v as SidebarSide)"
          />
          <p class="field-desc" style="margin-top: 8px">{{ t("winSettings.sidebarDesc") }}</p>

          <!-- 背景图：本地选图 + 不透明度 / 模糊 / 缩放（前端预览） -->
          <h3 class="group-title">{{ t("winSettings.bgImage") }}</h3>
          <p class="field-desc">{{ t("winSettings.bgImageDesc") }}</p>
          <div class="bg-buttons">
            <!-- 图片地址：本地文件路径或网址 -->
            <input
              v-model="bgSourceInput"
              class="field-input bg-url-input"
              :placeholder="t('winSettings.bgUrlPlaceholder')"
              @keydown.enter="loadBgSource"
            />
            <BaseButton size="sm" variant="accent" :disabled="bgLoading" @click="loadBgSource">
              {{ bgLoading ? t("winSettings.bgLoading") : t("winSettings.bgLoad") }}
            </BaseButton>
            <BaseButton v-if="bgImage" size="sm" variant="danger" :disabled="bgLoading" @click="setBgImage('')">
              {{ t("winSettings.bgClear") }}
            </BaseButton>
            <BaseButton size="sm" variant="accent" :disabled="bgLoading" @click="pickBgImage">{{ t("winSettings.bgPick") }}</BaseButton>
          </div>
          <input ref="bgFileInput" class="bg-file" type="file" accept="image/*" @change="onBgFile" />

          <template v-if="bgImage">
            <div class="range-row row-card">
              <span class="range-label">{{ t("winSettings.bgOpacity") }}</span>
              <input
                class="range"
                type="range"
                min="5"
                max="100"
                :value="bgOpacity"
                @input="setBgOpacity(Number(($event.target as HTMLInputElement).value))"
              />
              <span class="range-value">{{ bgOpacity }}%</span>
            </div>
            <div class="range-row row-card">
              <span class="range-label">{{ t("winSettings.bgBlur") }}</span>
              <input
                class="range"
                type="range"
                min="0"
                max="40"
                :value="bgBlur"
                @input="setBgBlur(Number(($event.target as HTMLInputElement).value))"
              />
              <span class="range-value">{{ bgBlur }}px</span>
            </div>
            <!-- 原始大小：后端把图片分辨率缩放到原图的百分之多少（点「应用」才生效，
                 且只改分辨率，不改变图在窗口里的显示大小） -->
            <div class="range-row row-card">
              <span class="range-label">{{ t("winSettings.bgNativeSize") }}</span>
              <input
                class="range"
                type="range"
                min="10"
                max="100"
                :value="bgSizeDraft"
                @input="bgSizeDraft = Number(($event.target as HTMLInputElement).value)"
              />
              <span class="range-value">{{ bgSizeDraft }}%</span>
              <BaseButton
                size="sm"
                variant="accent"
                :disabled="bgLoading || bgSizeDraft === bgNativeSize"
                @click="applyBgSize"
              >{{ bgLoading ? t("winSettings.bgLoading") : t("winSettings.bgApply") }}</BaseButton>
            </div>
          </template>
        </section>

        <!-- ================= 皮肤与头像 ================= -->
        <section v-else-if="tab === 'skin'" class="panel">
          <!-- 头像显示模式 -->
          <label class="field-label">{{ t("winSettings.headDisplay") }}</label>
          <p class="field-desc">{{ t("winSettings.headDisplayDesc") }}</p>
          <SegmentedTabs
            :model-value="headType"
            :options="[
              { value: 'Head2DA', label: t('winSettings.headType2DA') },
              { value: 'Head2DB', label: t('winSettings.headType2DB') },
              { value: 'Head3DA', label: t('winSettings.headType3DA') },
              { value: 'Head3DC', label: t('winSettings.headType3DC') },
              { value: 'Head3DB', label: t('winSettings.headType3DB') },
            ]"
            @update:model-value="(v) => setHeadConfig(v as HeadType)"
          />
          <template v-if="headType === 'Head3DB'">
            <div class="grid-2" style="margin-top: 12px">
              <div>
                <label class="field-label">{{ t("winSettings.rotX") }}</label>
                <NumberStepper v-model="headX" :min="-90" :max="90" @update:model-value="(v) => setHeadConfig(headType, v, headY)" />
              </div>
              <div>
                <label class="field-label">{{ t("winSettings.rotY") }}</label>
                <NumberStepper v-model="headY" :min="-180" :max="180" @update:model-value="(v) => setHeadConfig(headType, headX, v)" />
              </div>
            </div>
          </template>

          <!-- 皮肤显示模式：2D TypeA / 2D TypeB / 3D -->
          <label class="field-label" style="margin-top: 20px">{{ t("winSettings.skinDisplay") }}</label>
          <p class="field-desc">{{ t("winSettings.skinDisplayDesc") }}</p>
          <SegmentedTabs
            :model-value="skinDisplay"
            :options="[
              { value: 'Skin2DA', label: t('winSettings.skin2da') },
              { value: 'Skin2DB', label: t('winSettings.skin2db') },
              { value: 'Skin3D', label: t('winSettings.skin3d') },
              { value: 'Skin3DD', label: t('winSettings.skin3dd') },
            ]"
            @update:model-value="(v) => setSkinDisplay(v as SkinDisplay)"
          />

          <!-- 样例预览：内置纤细皮肤按当前头像 / 皮肤配置实时渲染（配置变化经 URL 版本号重取） -->
          <div class="sample-row">
            <div class="sample-box">
              <img v-if="imageBase" class="sample-img" :src="headPreviewUrl" alt="" />
              <span class="sample-label">{{ t("winSettings.headSample") }}</span>
            </div>
            <div class="sample-box">
              <img v-if="imageBase" class="sample-img skin" :src="skinPreviewUrl" alt="" />
              <span class="sample-label">{{ t("winSettings.skinSample") }}</span>
            </div>
          </div>
        </section>

        <!-- ================= 网络与下载 ================= -->
        <section v-else-if="tab === 'network'" class="panel">
          <template v-if="network">
            <!-- 下载 -->
            <h3 class="group-title">{{ t("winSettings.secDownload") }}</h3>
            <div class="grid-2 dl-grid">
              <div>
                <label class="field-label">{{ t("winSettings.downloadSource") }}</label>
                <SegmentedTabs
                  :model-value="network.source"
                  :options="[
                    { value: 'Offical', label: t('winSettings.sourceOffical') },
                    { value: 'Bmclapi', label: t('winSettings.sourceBmclapi') },
                  ]"
                  @update:model-value="(v) => { network!.source = v; applyNetwork(); }"
                />
              </div>
              <div>
                <label class="field-label">{{ t("winSettings.downloadThread") }}</label>
                <NumberStepper
                  :model-value="network.downloadThread"
                  :min="1"
                  :max="64"
                  @update:model-value="(v) => { network!.downloadThread = v; applyNetwork(); }"
                />
              </div>
            </div>

            <!-- 下载校验 / 自动下载：开关行，即改即存 -->
            <div class="grid-2" style="margin-top: 14px">
              <div class="switch-row">
                <span>{{ t("winSettings.checkFile") }}</span>
                <BaseSwitch
                  :model-value="network.checkFile"
                  @update:model-value="(v) => { network!.checkFile = v; applyNetwork(); }"
                />
              </div>
              <div class="switch-row">
                <span>{{ t("winSettings.autoDownload") }}</span>
                <BaseSwitch
                  :model-value="network.autoDownload"
                  @update:model-value="(v) => { network!.autoDownload = v; applyNetwork(); }"
                />
              </div>
            </div>

            <!-- 代理 -->
            <h3 class="group-title">{{ t("winSettings.secProxy") }}</h3>
            <div class="grid-2">
              <div>
                <label class="field-label">{{ t("winSettings.proxyWork") }}</label>
                <SegmentedTabs
                  :model-value="network.workProxy"
                  :options="[
                    { value: 'Auto', label: t('winSettings.proxyAuto') },
                    { value: 'None', label: t('winSettings.proxyNone') },
                    { value: 'User', label: t('winSettings.proxyUser') },
                  ]"
                  @update:model-value="(v) => (network!.workProxy = v)"
                />
              </div>
              <div>
                <label class="field-label">{{ t("winSettings.proxyLogin") }}</label>
                <SegmentedTabs
                  :model-value="network.loginProxy"
                  :options="[
                    { value: 'Auto', label: t('winSettings.proxyAuto') },
                    { value: 'None', label: t('winSettings.proxyNone') },
                    { value: 'User', label: t('winSettings.proxyUser') },
                  ]"
                  @update:model-value="(v) => (network!.loginProxy = v)"
                />
              </div>
            </div>

            <!-- 任一路走手动代理才显示代理详情 -->
            <template v-if="network.workProxy === 'User' || network.loginProxy === 'User'">
              <div class="grid-2" style="margin-top: 14px">
                <div>
                  <label class="field-label">{{ t("winSettings.proxyType") }}</label>
                  <SegmentedTabs
                    :model-value="network.workProxyType"
                    :options="[
                      { value: 'Http', label: 'HTTP' },
                      { value: 'Sock4', label: 'SOCKS4' },
                      { value: 'Sock5', label: 'SOCKS5' },
                    ]"
                    @update:model-value="(v) => { network!.workProxyType = v; network!.loginProxyType = v; }"
                  />
                </div>
              </div>
              <div class="grid-2" style="margin-top: 12px">
                <div>
                  <label class="field-label">{{ t("winSettings.proxyIp") }}</label>
                  <input v-model="network.proxyIp" class="field-input" spellcheck="false" />
                </div>
                <div>
                  <label class="field-label">{{ t("winSettings.proxyPort") }}</label>
                  <input
                    :value="network.proxyPort"
                    class="field-input"
                    type="number"
                    @change="network!.proxyPort = Number(($event.target as HTMLInputElement).value) || 0"
                  />
                </div>
                <div>
                  <label class="field-label">{{ t("winSettings.proxyUsername") }}</label>
                  <input v-model="network.proxyUser" class="field-input" spellcheck="false" />
                </div>
                <div>
                  <label class="field-label">{{ t("winSettings.proxyPassword") }}</label>
                  <input v-model="network.proxyPassword" class="field-input" type="password" />
                </div>
              </div>
            </template>

            <!-- 代理字段较多，保留显式保存按钮（其余网络设置即改即存） -->
            <div class="save-row right">
              <BaseButton variant="accent" size="sm" @click="saveNetwork">{{ t("winSettings.save") }}</BaseButton>
            </div>

            <!-- 自定义 DNS（DoH） -->
            <h3 class="group-title">{{ t("winSettings.dns") }}</h3>
            <div class="switch-list">
              <div class="switch-row">
                <span>{{ t("winSettings.dnsEnable") }}</span>
                <BaseSwitch
                  :model-value="network.dns.enable"
                  @update:model-value="(v) => { network!.dns.enable = v; applyNetwork(); }"
                />
              </div>
              <div class="switch-row" :class="{ dim: !network.dns.enable }">
                <span>{{ t("winSettings.dnsProxy") }}</span>
                <BaseSwitch
                  :model-value="network.dns.httpProxy"
                  :disabled="!network.dns.enable"
                  @update:model-value="(v) => { network!.dns.httpProxy = v; applyNetwork(); }"
                />
              </div>
            </div>
            <template v-if="network.dns.enable">
              <label class="field-label" style="margin-top: 10px">{{ t("winSettings.dnsHttps") }}</label>
              <!-- 编辑中只改本地草稿，失焦 / 回车才落盘 -->
              <div class="dns-lines">
                <div v-for="(line, i) in dnsLines" :key="i" class="line-row">
                  <input
                    class="field-input grow"
                    :value="line"
                    spellcheck="false"
                    @input="setDnsLine(i, ($event.target as HTMLInputElement).value)"
                    @change="commitDns(); applyNetwork();"
                    @keydown.enter="($event.target as HTMLInputElement).blur()"
                  />
                  <button class="line-del" title="✕" @click="removeDnsLine(i)">✕</button>
                </div>
                <button class="line-add" @click="addDnsLine">＋ {{ t("args.addLine") }}</button>
              </div>
            </template>

            <!-- 游戏文件检查：关闭"检查xx"后对应的 SHA1 校验一并禁用（没得查自然不用校验） -->
            <h3 class="group-title">{{ t("winSettings.gameCheck") }}</h3>
            <div class="grid-2">
              <div class="switch-list">
                <div class="switch-row">
                  <span>{{ t("winSettings.checkCore") }}</span>
                  <BaseSwitch
                  :model-value="network!.check.core"
                  @update:model-value="(v) => { network!.check.core = v; applyNetwork(); }" />
                </div>
                <div class="switch-row">
                  <span>{{ t("winSettings.checkLib") }}</span>
                  <BaseSwitch
                  :model-value="network!.check.lib"
                  @update:model-value="(v) => { network!.check.lib = v; applyNetwork(); }" />
                </div>
                <div class="switch-row">
                  <span>{{ t("winSettings.checkAssets") }}</span>
                  <BaseSwitch
                  :model-value="network!.check.assets"
                  @update:model-value="(v) => { network!.check.assets = v; applyNetwork(); }" />
                </div>
                <div class="switch-row">
                  <span>{{ t("winSettings.checkMod") }}</span>
                  <BaseSwitch
                  :model-value="network!.check.gameMod"
                  @update:model-value="(v) => { network!.check.gameMod = v; applyNetwork(); }" />
                </div>
              </div>
              <div class="switch-list">
                <div class="switch-row" :class="{ dim: !network.check.core }">
                  <span>{{ t("winSettings.checkCoreSha1") }}</span>
                  <BaseSwitch
                  :model-value="network!.check.coreSha1"
                  :disabled="!network.check.core"
                  @update:model-value="(v) => { network!.check.coreSha1 = v; applyNetwork(); }" />
                </div>
                <div class="switch-row" :class="{ dim: !network.check.lib }">
                  <span>{{ t("winSettings.checkLibSha1") }}</span>
                  <BaseSwitch
                  :model-value="network!.check.libSha1"
                  :disabled="!network.check.lib"
                  @update:model-value="(v) => { network!.check.libSha1 = v; applyNetwork(); }" />
                </div>
                <div class="switch-row" :class="{ dim: !network.check.assets }">
                  <span>{{ t("winSettings.checkAssetsSha1") }}</span>
                  <BaseSwitch
                  :model-value="network!.check.assetsSha1"
                  :disabled="!network.check.assets"
                  @update:model-value="(v) => { network!.check.assetsSha1 = v; applyNetwork(); }" />
                </div>
                <div class="switch-row" :class="{ dim: !network.check.gameMod }">
                  <span>{{ t("winSettings.checkModSha1") }}</span>
                  <BaseSwitch
                  :model-value="network!.check.modSha1"
                  :disabled="!network.check.gameMod"
                  @update:model-value="(v) => { network!.check.modSha1 = v; applyNetwork(); }" />
                </div>
              </div>
            </div>
          </template>
          <p v-else class="field-desc">{{ t("winSettings.tauriOnly") }}</p>
        </section>

        <!-- ================= 游戏启动 ================= -->
        <section v-else-if="tab === 'launch'" class="panel">
          <template v-if="run && win">
            <!-- 游戏窗口 -->
            <h3 class="group-title">{{ t("winSettings.secGameWindow") }}</h3>
            <div class="switch-row">
              <span>{{ t("winSettings.fullScreen") }}</span>
              <BaseSwitch v-model="win!.fullScreen" />
            </div>
            <div class="grid-2" style="margin-top: 10px">
              <div>
                <label class="field-label">{{ t("winSettings.width") }}</label>
                <NumberStepper v-model="win!.width" :min="100" :max="65535" />
              </div>
              <div>
                <label class="field-label">{{ t("winSettings.height") }}</label>
                <NumberStepper v-model="win!.height" :min="100" :max="65535" />
              </div>
            </div>

            <!-- 内存 -->
            <h3 class="group-title">{{ t("winSettings.secMemory") }}</h3>
            <div class="grid-2">
              <div>
                <label class="field-label">{{ t("winSettings.minMemory") }}</label>
                <NumberStepper v-model="run!.minMemory" :min="256" :max="65536" :step="256" />
              </div>
              <div>
                <label class="field-label">{{ t("winSettings.maxMemory") }}</label>
                <NumberStepper v-model="run!.maxMemory" :min="256" :max="65536" :step="256" />
              </div>
            </div>

            <!-- JVM -->
            <h3 class="group-title">{{ t("winSettings.secJvm") }}</h3>
            <div class="grid-2">
              <div>
                <label class="field-label">{{ t("winSettings.gcMode") }}</label>
                <SegmentedTabs
                  v-model="run!.gcMode"
                  :options="[
                    { value: 'Auto', label: t('winSettings.gcAuto') },
                    { value: 'G1GC', label: t('winSettings.gcG1gc') },
                    { value: 'ZGC', label: t('winSettings.gcZgc') },
                    { value: 'None', label: t('winSettings.gcNone') },
                  ]"
                />
              </div>
              <div class="switch-list">
                <div class="switch-row">
                  <span>{{ t("winSettings.colorasm") }}</span>
                  <BaseSwitch v-model="run!.colorasm" />
                </div>
                <div class="switch-row">
                  <span>{{ t("winSettings.removeJvmArg") }}</span>
                  <BaseSwitch v-model="run!.removeJvmArg" />
                </div>
              </div>
            </div>

            <label class="field-label" style="margin-top: 14px">{{ t("winSettings.jvmEnv") }}</label>
            <div class="dns-lines">
              <div v-for="(_, i) in envLines" :key="i" class="line-row">
                <input
                  v-model="envLines[i].key"
                  class="field-input grow"
                  spellcheck="false"
                  :placeholder="t('winSettings.envKey')"
                />
                <input
                  v-model="envLines[i].value"
                  class="field-input grow"
                  spellcheck="false"
                  :placeholder="t('winSettings.envValue')"
                />
                <button class="line-del" title="✕" @click="removeEnvLine(i)">✕</button>
              </div>
              <button class="line-add" @click="addEnvLine">＋ {{ t("args.addLine") }}</button>
            </div>

            <label class="field-label" style="margin-top: 10px">{{ t("winSettings.jvmArgs") }}</label>
            <textarea v-model="run!.jvmArgs" class="field-input args-input" spellcheck="false" />

            <!-- 游戏参数 -->
            <h3 class="group-title">{{ t("winSettings.secGameArgs") }}</h3>
            <div class="switch-row">
              <span>{{ t("winSettings.removeGameArg") }}</span>
              <BaseSwitch v-model="run!.removeGameArg" />
            </div>
            <label class="field-label" style="margin-top: 10px">{{ t("winSettings.gameArgs") }}</label>
            <textarea v-model="run!.gameArgs" class="field-input args-input" spellcheck="false" />

            <!-- 启动命令 -->
            <h3 class="group-title">{{ t("winSettings.secLaunchCmd") }}</h3>
            <div class="switch-list">
              <div class="switch-row">
                <span>{{ t("winSettings.preLaunch") }}</span>
                <BaseSwitch v-model="run!.launchPreRun" />
              </div>
              <div class="switch-row" :class="{ dim: !run.launchPreRun }">
                <span>{{ t("winSettings.preSameTime") }}</span>
                <BaseSwitch v-model="run!.preRunWithGame" :disabled="!run.launchPreRun" />
              </div>
              <div class="switch-row">
                <span>{{ t("winSettings.postLaunch") }}</span>
                <BaseSwitch v-model="run!.launchPostRun" />
              </div>
            </div>
            <template v-if="run.launchPreRun">
              <label class="field-label" style="margin-top: 10px">{{ t("winSettings.preCmd") }}</label>
              <input v-model="run!.preRunArg" class="field-input" spellcheck="false" />
            </template>
            <template v-if="run.launchPostRun">
              <label class="field-label" style="margin-top: 10px">{{ t("winSettings.postCmd") }}</label>
              <input v-model="run!.postRunArg" class="field-input" spellcheck="false" />
            </template>

          </template>
          <p v-else class="field-desc">{{ t("winSettings.tauriOnly") }}</p>
        </section>

        <!-- ================= Java ================= -->
        <section v-else-if="tab === 'java'" class="panel">
          <template v-if="inTauri">
            <!-- 手动添加：输入名字和路径（放最上面） -->
            <h3 class="group-title" style="margin-top: 0">{{ t("winSettings.javaAdd") }}</h3>
            <div class="java-add-row">
              <input v-model="newJavaName" class="field-input java-add-name" :placeholder="t('winSettings.javaName')" spellcheck="false" />
              <input v-model="newJavaPath" class="field-input grow" :placeholder="t('winSettings.javaPath')" spellcheck="false" />
              <BaseButton size="sm" @click="browseJava">{{ t("args.browse") }}</BaseButton>
              <BaseButton
                size="sm" variant="accent"
                :disabled="addingJava || !newJavaName.trim() || !newJavaPath.trim()"
                @click="addJava"
              >
                {{ t("winSettings.javaAdd") }}
              </BaseButton>
            </div>
            <div class="save-row java-add-actions">
              <BaseButton size="sm" :disabled="scanning || scanningDir" @click="scanJava">
                {{ scanning ? t("winSettings.javaScanning") : t("winSettings.javaScan") }}
              </BaseButton>
              <BaseButton size="sm" :disabled="scanning || scanningDir" @click="scanJavaDir">
                {{ t("winSettings.javaScanDir") }}
              </BaseButton>
              <BaseButton size="sm" @click="importJavaArchive">{{ t("winSettings.javaImport") }}</BaseButton>
              <BaseButton size="sm" @click="downloadJava">
                {{ t("winSettings.javaDownload") }}
              </BaseButton>
              <BaseButton size="sm" variant="danger" :disabled="javaList.length === 0" @click="confirmRemoveAll = true">
                {{ t("winSettings.javaRemoveAll") }}
              </BaseButton>
            </div>

            <!-- 压缩包导入进度条（解包 / 识别阶段经 settings-java-progress 事件推进） -->
            <div v-if="importingJava || importProgress" class="java-progress">
              <span class="java-progress-label">{{ t("winSettings.javaImporting") }}</span>
              <div class="java-progress-track">
                <div class="java-progress-fill" :style="{ width: importPercent + '%' }" />
              </div>
              <span class="java-progress-text">{{ importProgress ? `${importProgress.now}/${importProgress.total}` : "" }}</span>
              <span v-if="importProgress?.subText" class="java-progress-sub" :title="importProgress.subText">{{ importProgress.subText }}</span>
            </div>

            <!-- 删除全部确认 -->
            <BaseModal
              v-if="confirmRemoveAll"
              :title="t('winSettings.javaRemoveAll')"
              :overlay-close="false"
              below-titlebar
              @close="confirmRemoveAll = false"
            >
              <p class="field-desc">{{ t("winSettings.javaRemoveAllConfirm") }}</p>
              <div class="save-row">
                <BaseButton size="sm" @click="confirmRemoveAll = false">{{ t("actions.cancel") }}</BaseButton>
                <BaseButton size="sm" variant="danger" @click="removeAllJava">{{ t("actions.confirm") }}</BaseButton>
              </div>
            </BaseModal>

            <!-- 扫描 Java 加载弹窗：扫描期间显示，完成自动消失 -->
            <BaseModal
              v-if="scanning || scanningDir"
              :width="300"
              :closable="false"
              :overlay-close="false"
            >
              <div class="loading-modal">
                <span class="loading-spin" />
                <span>{{ t("winSettings.javaScanning") }}</span>
              </div>
            </BaseModal>

            <div v-if="javaList.length === 0" class="field-desc">{{ t("winSettings.javaNone") }}</div>
            <!-- 按发行类型（JDK / JRE）分组：类型行展开后，该类型的 Java 以卡片网格平铺 -->
            <div v-for="g in javaGroups" :key="g.type" class="java-type">
              <button class="java-type-head" @click="toggleType(g.type)">
                <span class="chev" :class="{ up: typeOpen(g.type) }">▾</span>
                <span class="java-type-name">{{ g.type }}</span>
                <span class="java-count">{{ g.items.length }}</span>
              </button>
              <CollapsePanel :open="typeOpen(g.type)">
                <div class="java-grid">
                  <div v-for="j in g.items" :key="j.name" class="java-card">
                    <div class="java-card-top">
                      <span class="java-name" :title="j.name">{{ j.name }}</span>
                      <BaseButton size="sm" variant="danger" @click="removeJava(j.name)">
                        {{ t("winSettings.javaRemove") }}
                      </BaseButton>
                    </div>
                    <span class="java-meta">{{ j.version }} · {{ j.arch }}</span>
                    <span class="java-path" :title="j.path">{{ j.path }}</span>
                  </div>
                </div>
              </CollapsePanel>
            </div>
          </template>
          <p v-else class="field-desc">{{ t("winSettings.tauriOnly") }}</p>
        </section>

        <!-- ================= 客户端设置 ================= -->
        <section v-else-if="tab === 'client'" class="panel">
          <h3 class="group-title">{{ t("winSettings.secServers") }}</h3>

          <!-- 服务器地址：自动进服与 MOTD 显示共用 -->
          <label class="field-label">{{ t("winSettings.serverAddress") }}</label>
          <input
            v-model="serverAddr"
            class="field-input"
            spellcheck="false"
            placeholder="mc.example.com:25565"
            :title="t('winSettings.serverAddressHint')"
          />
          <p class="field-desc" style="margin-top: 8px">{{ t("winSettings.serverAddressHint") }}</p>

          <!-- 自动进服：启动时自动进入上面配置的服务器 -->
          <div class="switch-row" style="margin-top: 12px">
            <div class="switch-text">
              <span class="switch-label">{{ t("winSettings.autoJoin") }}</span>
              <span class="switch-state">{{ t("winSettings.autoJoinDesc") }}</span>
            </div>
            <BaseSwitch v-model="client.autoJoin" @update:model-value="applyClient" />
          </div>

          <!-- MOTD 卡片显示与刷新间隔 -->
          <div class="switch-row" style="margin-top: 8px">
            <div class="switch-text">
              <span class="switch-label">{{ t("winSettings.motdCard") }}</span>
              <span class="switch-state">{{ t("winSettings.motdCardDesc") }}</span>
            </div>
            <BaseSwitch v-model="client.motdCard" @update:model-value="applyClient" />
          </div>
          <div class="switch-row" :class="{ dim: !client.motdCard }" style="margin-top: 8px">
            <div class="switch-text">
              <span class="switch-label">{{ t("winSettings.motdInterval") }}</span>
              <span class="switch-state">{{ t("winSettings.motdIntervalDesc") }}</span>
            </div>
            <NumberStepper
              v-model="client.motdInterval"
              :min="5"
              :max="600"
              @update:model-value="applyClient"
            />
          </div>

          <h3 class="group-title">{{ t("winSettings.secLoginLock") }}</h3>
          <p class="field-desc">{{ t("winSettings.loginLockDesc") }}</p>
          <!-- 总开关：关闭时锁定列表不生效 -->
          <div class="switch-row">
            <div class="switch-text">
              <span class="switch-label">{{ t("winSettings.loginLockOn") }}</span>
              <span class="switch-state">{{ t("winSettings.loginLockOnDesc") }}</span>
            </div>
            <BaseSwitch v-model="client.loginLockOn" @update:model-value="applyClient" />
          </div>
          <!-- 已锁定的条目列表（总开关打开时才可编辑） -->
          <template v-if="client.loginLockOn">
          <div v-if="client.loginLock.length" class="switch-list">
            <div v-for="(e, i) in client.loginLock" :key="e.ty" class="switch-row lock-item">
              <span class="lock-item-text">
                {{ loginTypeLabel(e.ty) }}<template v-if="e.name"> — {{ e.name }}</template><template v-if="e.server"> — {{ e.server }}</template>
              </span>
              <BaseButton size="sm" variant="danger" @click="removeLock(i)">
                {{ t("winSettings.loginLockRemove") }}
              </BaseButton>
            </div>
          </div>
          <!-- 添加条目：选类型；外置登录 / 自定义皮肤站 / 统一通行证必须填服务器信息，可重复添加不同地址 -->
          <div class="lock-add">
            <select v-model="addLockType" class="field-select lock-add-type">
              <option v-for="ty in addLockOptions" :key="ty" :value="ty">
                {{ loginTypeLabel(ty) }}
              </option>
            </select>
            <input
              v-if="addLockHasServer"
              v-model="addLockName"
              class="field-input lock-add-name"
              spellcheck="false"
              :placeholder="t('winSettings.lockModelName')"
            />
            <input
              v-if="addLockHasServer"
              v-model="addLockServer"
              class="field-input lock-add-server"
              :class="{ 'lock-server-error': lockServerError }"
              spellcheck="false"
              :placeholder="lockServerPlaceholder"
              @input="lockServerError = ''"
            />
            <BaseButton
              size="sm"
              variant="accent"
              :disabled="!addLockOptions.length"
              @click="addLock"
            >
              {{ t("winSettings.loginLockAdd") }}
            </BaseButton>
          </div>
          <p v-if="lockServerError" class="lock-server-hint">{{ lockServerError }}</p>
          </template>

          <!-- 游戏标题（游戏窗口标题栏的自定义文字，全局默认值） -->
          <template v-if="win">
            <h3 class="group-title">{{ t("winSettings.secGameTitle") }}</h3>
            <div class="switch-list">
              <div class="switch-row">
                <span>{{ t("winSettings.editTitle") }}</span>
                <BaseSwitch v-model="win.editTitle" />
              </div>
              <div class="switch-row">
                <span>{{ t("winSettings.randomTitle") }}</span>
                <BaseSwitch v-model="win.randomTitle" />
              </div>
              <div class="switch-row">
                <span>{{ t("winSettings.cycleTitle") }}</span>
                <BaseSwitch v-model="win.cycleTitle" />
              </div>
            </div>
            <template v-if="win.editTitle">
              <label class="field-label" style="margin-top: 10px">{{ t("winSettings.gameTitle") }}</label>
              <input v-model="win.gameTitle" class="field-input" spellcheck="false" />
            </template>
            <template v-if="win.cycleTitle">
              <label class="field-label" style="margin-top: 10px">{{ t("winSettings.titleDelay") }}</label>
              <NumberStepper v-model="win.titleDelay" :min="100" :max="600000" :step="100" />
            </template>
          </template>
        </section>
      </div>
    </div>
  </WindowFrame>
</template>

<style scoped>
/* 左标签 + 右内容：占满窗口内容区，右侧独立滚动。
   用负外边距抵消 WindowFrame .frame-body 的 22px/26px 内边距，
   让标签导航和滚动条都顶到窗口边缘（滚动条贴右） */
.settings-layout {
  display: flex;
  gap: 0;
  height: calc(100% + 44px);
  min-height: 0;
  margin: -22px -26px;
}

.tab-rail {
  width: 176px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 14px 10px 14px 16px;
  border-right: 1px solid var(--border);
  background: var(--bg-side);
}

.tab-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border: none;
  border-radius: 10px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
  text-align: left;
  transition: background 0.12s, color 0.12s;
}

.tab-item:hover {
  background: var(--bg-card);
  color: var(--text);
}

.tab-item.active {
  background: var(--accent-soft);
  color: var(--accent);
}

.tab-icon {
  flex-shrink: 0;
  display: flex;
}

.tab-text {
  display: flex;
  flex-direction: column;
  line-height: 1.3;
  min-width: 0;
}

.tab-title {
  font-size: 13px;
  font-weight: 600;
}

.tab-desc {
  font-size: 10.5px;
  color: var(--text-dim);
  opacity: 0.8;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.tab-content {
  flex: 1;
  min-width: 0;
  overflow-y: auto;
  padding: 18px 22px 24px;
}

.panel {
  padding-bottom: 8px;
}

/* 当前页大标题 */
.content-head {
  margin-bottom: 6px;
}

.content-title {
  font-size: 19px;
  font-weight: 800;
}

.content-sub {
  font-size: 12px;
  color: var(--text-dim);
  margin-top: 3px;
}

/* 内容区的分组标题：左侧强调色竖条 + 底部分隔线 */
.group-title {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13.5px;
  font-weight: 700;
  margin: 22px 0 12px;
  padding-bottom: 9px;
  border-bottom: 1px solid var(--border);
}

.group-title::before {
  content: "";
  width: 3px;
  height: 13px;
  border-radius: 2px;
  background: var(--accent);
}

.panel > .group-title:first-of-type {
  margin-top: 10px;
}

/* 设置行卡片：开关 / 滑杆条目有底色有圆角，不再裸排在页面上 */
.row-card {
  background: var(--bg-side);
  border: 1px solid var(--border);
  border-radius: 11px;
  padding: 13px 16px;
}

.field-desc {
  font-size: 12px;
  color: var(--text-dim);
  margin: -4px 0 10px;
  line-height: 1.6;
}

/* 样例预览：头像与皮肤并排一行，内置纤细皮肤按当前配置渲染 */
.sample-row {
  display: flex;
  align-items: stretch;
  gap: 12px;
  margin-top: 16px;
  flex-wrap: wrap;
}

.sample-box {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  background: var(--bg-side);
  border: 1px solid var(--border);
  border-radius: 10px;
}

.sample-img {
  height: 100px;
  width: auto;
  image-rendering: pixelated;
}

/* 皮肤全身图比头像高一些 */
.sample-img.skin {
  height: 200px;
}

.sample-label {
  font-size: 11.5px;
  color: var(--text-dim);
}

.accent-list {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
}

.accent-swatch {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: 2px solid var(--border);
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: transform 0.12s, border-color 0.12s, box-shadow 0.12s;
}

.accent-swatch:hover {
  transform: scale(1.12);
}

.accent-swatch.active {
  border-color: var(--text);
  box-shadow: 0 0 0 3px var(--bg-card), 0 0 0 5px var(--text-dim);
}

/* 自定义强调色：彩虹色块，点击弹出系统取色器（input 只当取色入口用，不占位） */
.accent-custom {
  background: conic-gradient(#ff5f56, #facc15, #34d399, #22d3ee, #4f8cff, #a78bfa, #e879f9, #ff5f56);
  position: relative;
}

.accent-custom input {
  position: absolute;
  inset: 0;
  opacity: 0;
  cursor: pointer;
}

.hint {
  font-size: 12px;
  color: var(--accent);
  margin-top: 10px;
}

.font-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.font-select {
  flex: 1;
}

/* 语言 / 字体两个下拉并排一行，各占一半 */
.general-row {
  display: flex;
  gap: 14px;
}

.general-col {
  flex: 1;
  min-width: 0;
}

/* 下载源页签与线程数步进器统一高度（SegmentedTabs 自然高度 ≈35px） */
.dl-grid .seg-tabs,
.dl-grid .stepper {
  height: 35px;
}

/* 两态设置行：左侧标题 + 当前值，右侧 Switch */
.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.switch-text {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.switch-label {
  font-size: 12.5px;
  font-weight: 600;
}

.switch-state {
  font-size: 11.5px;
  color: var(--text-dim);
}

/* 背景图 */
/* 选图 / 清除 / 地址输入 / 加载 一行：输入框占满剩余空间 */
.bg-buttons {
  display: flex;
  /* 按钮与 42px 输入框等高（search-btn 行同款：stretch 撑高，覆盖 BaseButton 定高） */
  align-items: stretch;
  gap: 10px;
}

.bg-buttons .ui-btn {
  height: auto;
}

.bg-url-input {
  flex: 1;
  min-width: 0;
}

.bg-file {
  display: none;
}

.range-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
}

.range-label {
  width: 72px;
  flex-shrink: 0;
  font-size: 12.5px;
  color: var(--text-dim);
}

.range {
  flex: 1;
  accent-color: var(--accent);
}

.range-value {
  width: 52px;
  flex-shrink: 0;
  text-align: right;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  color: var(--text-dim);
}

.grid-2 {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px 16px;
}

/* 开关行（下载校验 / DNS / 游戏文件检查）：文字居左、BaseSwitch 居右 */
.switch-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.switch-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  font-size: 12.5px;
  user-select: none;
}

.switch-row.dim {
  opacity: 0.55;
}

/* 登录方式锁定：条目列表 + 添加行 */
.lock-item-text {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.lock-add {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 10px;
}

.lock-add-type {
  width: 160px;
  flex-shrink: 0;
}

.lock-add-name {
  width: 150px;
  flex-shrink: 0;
}

.lock-add-server {
  flex: 1;
  min-width: 0;
}

/* 服务器信息缺失 / 重复：红框（含聚焦态，覆盖默认的 accent 边框） */
.lock-add-server.lock-server-error,
.lock-add-server.lock-server-error:focus {
  border-color: var(--red);
}

.lock-server-hint {
  color: var(--red);
  font-size: 12px;
  margin: 6px 0 0;
}

.save-row {
  display: flex;
  gap: 10px;
  margin-top: 16px;
}

/* 代理保存按钮靠右放，矮版（size=sm），不占一整行视觉重心 */
.save-row.right {
  justify-content: flex-end;
}

.args-input {
  min-height: 56px;
  resize: vertical;
  font-family: "Cascadia Code", Consolas, monospace;
  font-size: 12px;
}

/* DoH 逐行编辑（与 LaunchArgsPanel 的 JVM 参数行编辑同款） */
.dns-lines {
  display: flex;
  flex-direction: column;
  gap: 8px;
  margin-top: 6px;
}

.line-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.grow {
  flex: 1;
  min-width: 120px;
}

.line-del {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-raised);
  color: var(--text-dim);
  cursor: pointer;
  flex-shrink: 0;
}

.line-del:hover {
  color: var(--red);
  border-color: var(--red);
  background: rgba(255, 95, 86, 0.1);
}

.line-add {
  align-self: flex-start;
  display: inline-flex;
  align-items: center;
  height: 28px;
  padding: 0 22px;
  border-radius: 9px;
  border: 1px dashed var(--border);
  background: var(--bg-raised);
  color: var(--text-dim);
  cursor: pointer;
  font-family: inherit;
}

.line-add:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

/* Java 页：按类型分组的行，展开后卡片网格平铺 */
.java-type {
  margin-bottom: 4px;
}

.java-type-head {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 8px 0;
  border: none;
  background: transparent;
  color: var(--text);
  cursor: pointer;
  font-family: inherit;
}

.java-type-name {
  font-size: 13.5px;
  font-weight: 600;
}

.java-count {
  font-size: 11px;
  color: var(--text-dim);
  background: var(--bg-hover);
  border-radius: 999px;
  padding: 1px 8px;
}

.chev {
  font-size: 11px;
  color: var(--text-dim);
  transition: transform 0.22s ease;
}

.chev.up {
  transform: rotate(180deg);
}

.java-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: 10px;
  padding: 4px 0 8px;
}

.java-card {
  display: flex;
  flex-direction: column;
  gap: 4px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-raised);
  min-width: 0;
}

.java-card-top {
  display: flex;
  align-items: center;
  gap: 8px;
  justify-content: space-between;
}

.java-name {
  font-weight: 600;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.java-meta {
  font-size: 11.5px;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.java-path {
  font-size: 11px;
  color: var(--text-dim);
  font-family: "Cascadia Code", Consolas, monospace;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.java-add-row {
  display: flex;
  /* 按钮与 42px 输入框等高（stretch 撑高，覆盖 BaseButton 定高） */
  align-items: stretch;
  gap: 8px;
}

.java-add-row .ui-btn {
  height: auto;
}

/* 添加区在最上面：按钮行与下方分组拉开距离 */
.java-add-actions {
  margin-bottom: 12px;
}

/* 压缩包导入进度条 */
.java-progress {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  font-size: 12px;
}

.java-progress-label {
  flex: none;
  color: var(--text-dim);
}

.java-progress-track {
  flex: 1;
  height: 6px;
  border-radius: 3px;
  background: var(--bg-raised);
  overflow: hidden;
}

.java-progress-fill {
  height: 100%;
  border-radius: 3px;
  background: var(--accent);
  transition: width 0.2s ease;
}

.java-progress-text {
  flex: none;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.java-progress-sub {
  flex: none;
  max-width: 220px;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* 扫描 Java 加载弹窗内容：转圈 + 提示文字 */
.loading-modal {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 12px;
  color: var(--text-dim);
  font-size: 14px;
}

.loading-spin {
  width: 20px;
  height: 20px;
  border: 2.5px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: settings-loading-spin 0.8s linear infinite;
}

@keyframes settings-loading-spin {
  to {
    transform: rotate(360deg);
  }
}

.java-add-name {
  width: 160px;
  flex: none;
}

.java-name {
  font-weight: 600;
  font-size: 13px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.java-meta {
  flex: 1;
  font-size: 11.5px;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
