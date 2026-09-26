<script setup lang="ts">
// 启动器设置窗口：左侧标签导航 + 右侧内容区
// 标签：界面（含窗口设置）/ 皮肤与头像 / 网络与下载 / 游戏启动
import { computed, onMounted, ref, watch } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import NumberStepper from "../../components/ui/NumberStepper.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseSwitch from "../../components/ui/BaseSwitch.vue";
import { t, locale, setLocale, tErr } from "../../lib/i18n";
import { commands, type JavaInfoDto, type NetworkSettingDto } from "../../lib/bindings";
import { multiWindow, setMultiWindow, isTauri } from "../windowManager";
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

const inTauri = isTauri();

// ================= 标签导航 =================

type SettingsTab = "ui" | "skin" | "network" | "launch";

const TABS: Array<{ id: SettingsTab; icon: string }> = [
  { id: "ui", icon: "palette" },
  { id: "skin", icon: "user" },
  { id: "network", icon: "download" },
  { id: "launch", icon: "play" },
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
/** DoH 地址列表 ⇄ 每行一个的文本 */
const dnsHttpsText = computed({
  get: () => network.value?.dns.https.join("\n") ?? "",
  set: (v: string) => {
    if (network.value) {
      network.value.dns.https = v
        .split("\n")
        .map((s) => s.trim())
        .filter((s) => s.length > 0);
    }
  },
});

// ---- 游戏启动设置 ----
const javaList = ref<JavaInfoDto[]>([]);
const minMemory = ref(512);
const maxMemory = ref(4096);
const jvmArgs = ref("");
const gameArgs = ref("");
const scanning = ref(false);

onMounted(async () => {
  if (!inTauri) return;
  fontsLoading.value = true;
  fonts.value = await commands.settings.getSystemFonts().catch(() => []);
  fontsLoading.value = false;
  network.value = await commands.settings.getNetwork().catch(() => null);
  const launch = await commands.settings.getLaunch().catch(() => null);
  if (launch) {
    javaList.value = launch.javaList;
    minMemory.value = launch.minMemory;
    maxMemory.value = launch.maxMemory;
    jvmArgs.value = launch.jvmArgs;
    gameArgs.value = launch.gameArgs;
  }
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

async function saveLaunch() {
  if (minMemory.value > maxMemory.value) {
    showToast(t("winSettings.memoryConflict"));
    return;
  }
  try {
    await commands.settings.saveLaunch(minMemory.value, maxMemory.value, jvmArgs.value, gameArgs.value);
    showToast(t("winSettings.saved"));
  } catch (e) {
    showToast(tErr(e));
  }
}

// ---- Java ----
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

async function addJava() {
  const { open } = await import("@tauri-apps/plugin-dialog");
  const picked = await open({
    title: t("winSettings.javaAdd"),
    multiple: false,
    filters: [{ name: "Java", extensions: ["exe"] }],
  });
  if (typeof picked !== "string") return;
  try {
    const info = await commands.settings.addJava(picked);
    javaList.value = [...javaList.value, info];
    showToast(t("winSettings.javaAdded", { name: info.name }));
  } catch (e) {
    showToast(tErr(e));
  }
}

async function removeJava(name: string) {
  try {
    javaList.value = await commands.settings.removeJava(name);
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
            <svg v-else viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="m6 4 14 8-14 8V4z" />
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
          <!-- 皮肤显示模式：2D TypeA / 2D TypeB / 3D -->
          <label class="field-label">{{ t("winSettings.skinDisplay") }}</label>
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

          <!-- 头像显示模式 -->
          <label class="field-label" style="margin-top: 20px">{{ t("winSettings.headDisplay") }}</label>
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
            <div class="save-row">
              <BaseButton variant="accent" @click="saveNetwork">{{ t("winSettings.save") }}</BaseButton>
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
              <textarea v-model="dnsHttpsText" class="field-input args-input" spellcheck="false" @change="applyNetwork()" />
            </template>

            <!-- 游戏文件检查：关闭"检查xx"后对应的 SHA1 校验一并禁用（没得查自然不用校验） -->
            <h3 class="group-title">{{ t("winSettings.gameCheck") }}</h3>
            <div class="grid-2">
              <div class="switch-list">
                <div class="switch-row">
                  <span>{{ t("winSettings.checkCore") }}</span>
                  <BaseSwitch v-model="network.check.core" />
                </div>
                <div class="switch-row">
                  <span>{{ t("winSettings.checkLib") }}</span>
                  <BaseSwitch v-model="network.check.lib" />
                </div>
                <div class="switch-row">
                  <span>{{ t("winSettings.checkAssets") }}</span>
                  <BaseSwitch v-model="network.check.assets" />
                </div>
                <div class="switch-row">
                  <span>{{ t("winSettings.checkMod") }}</span>
                  <BaseSwitch v-model="network.check.gameMod" />
                </div>
              </div>
              <div class="switch-list">
                <div class="switch-row" :class="{ dim: !network.check.core }">
                  <span>{{ t("winSettings.checkCoreSha1") }}</span>
                  <BaseSwitch v-model="network.check.coreSha1" :disabled="!network.check.core" />
                </div>
                <div class="switch-row" :class="{ dim: !network.check.lib }">
                  <span>{{ t("winSettings.checkLibSha1") }}</span>
                  <BaseSwitch v-model="network.check.libSha1" :disabled="!network.check.lib" />
                </div>
                <div class="switch-row" :class="{ dim: !network.check.assets }">
                  <span>{{ t("winSettings.checkAssetsSha1") }}</span>
                  <BaseSwitch v-model="network.check.assetsSha1" :disabled="!network.check.assets" />
                </div>
                <div class="switch-row" :class="{ dim: !network.check.gameMod }">
                  <span>{{ t("winSettings.checkModSha1") }}</span>
                  <BaseSwitch v-model="network.check.modSha1" :disabled="!network.check.gameMod" />
                </div>
              </div>
            </div>

            <div class="save-row">
              <BaseButton variant="accent" @click="saveNetwork">{{ t("winSettings.save") }}</BaseButton>
            </div>
          </template>
          <p v-else class="field-desc">{{ t("winSettings.tauriOnly") }}</p>
        </section>

        <!-- ================= 游戏启动 ================= -->
        <section v-else class="panel">
          <template v-if="inTauri">
            <div class="grid-2">
              <div>
                <label class="field-label">{{ t("winSettings.minMemory") }}</label>
                <NumberStepper v-model="minMemory" :min="256" :max="65536" :step="256" />
              </div>
              <div>
                <label class="field-label">{{ t("winSettings.maxMemory") }}</label>
                <NumberStepper v-model="maxMemory" :min="256" :max="65536" :step="256" />
              </div>
            </div>

            <label class="field-label" style="margin-top: 14px">{{ t("winSettings.jvmArgs") }}</label>
            <textarea v-model="jvmArgs" class="field-input args-input" spellcheck="false" />

            <label class="field-label" style="margin-top: 14px">{{ t("winSettings.gameArgs") }}</label>
            <textarea v-model="gameArgs" class="field-input args-input" spellcheck="false" />

            <div class="save-row">
              <BaseButton variant="accent" @click="saveLaunch">{{ t("winSettings.save") }}</BaseButton>
            </div>

            <h3 class="group-title">{{ t("winSettings.javaList") }}</h3>
            <div v-if="javaList.length === 0" class="field-desc">{{ t("winSettings.javaNone") }}</div>
            <div v-else class="java-list">
              <div v-for="j in javaList" :key="j.name" class="java-row">
                <span class="java-name" :title="j.path">{{ j.name }}</span>
                <span class="java-meta">{{ j.version }} · {{ j.javaType }} · {{ j.arch }}</span>
                <BaseButton size="sm" variant="danger" @click="removeJava(j.name)">
                  {{ t("winSettings.javaRemove") }}
                </BaseButton>
              </div>
            </div>
            <div class="save-row">
              <BaseButton size="sm" :disabled="scanning" @click="scanJava">
                {{ scanning ? t("winSettings.javaScanning") : t("winSettings.javaScan") }}
              </BaseButton>
              <BaseButton size="sm" @click="addJava">{{ t("winSettings.javaAdd") }}</BaseButton>
            </div>
          </template>
          <p v-else class="field-desc">{{ t("winSettings.tauriOnly") }}</p>
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
  align-items: center;
  gap: 10px;
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

.save-row {
  display: flex;
  gap: 10px;
  margin-top: 16px;
}

.args-input {
  min-height: 56px;
  resize: vertical;
  font-family: "Cascadia Code", Consolas, monospace;
  font-size: 12px;
}

.java-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.java-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--bg-side);
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
