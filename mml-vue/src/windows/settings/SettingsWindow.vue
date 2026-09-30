<script setup lang="ts">
// 启动器设置窗口：左侧标签导航 + 右侧内容区
// 标签：界面（含窗口设置）/ Java / 网络与下载 / 游戏启动 / 皮肤与头像 / 客户端设置
//
// 这里只做编排：状态与保存逻辑全在 composables/（每个标签一份），
// 排版在 parts/tabs/（每个标签一个组件），窗口级样式在 settings.css（非 scoped，子组件共用）。
// 本文件负责：标签切换（记忆上次所在页）、Esc 关窗、分组级「恢复默认」的派发与高亮。
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import SettingsRail from "./parts/SettingsRail.vue";
import SettingsSearch from "./parts/SettingsSearch.vue";
import UiTab from "./parts/tabs/UiTab.vue";
import JavaTab from "./parts/tabs/JavaTab.vue";
import NetworkTab from "./parts/tabs/NetworkTab.vue";
import LaunchTab from "./parts/tabs/LaunchTab.vue";
import SkinTab from "./parts/tabs/SkinTab.vue";
import ClientTab from "./parts/tabs/ClientTab.vue";
import { t } from "../../lib/i18n";
import { showToast } from "../../lib/toast";
import { useSettingsUi } from "./composables/useSettingsUi";
import { useSettingsSkin } from "./composables/useSettingsSkin";
import { useSettingsNetwork } from "./composables/useSettingsNetwork";
import { useSettingsLaunch } from "./composables/useSettingsLaunch";
import { useSettingsJava } from "./composables/useSettingsJava";
import { useSettingsClient } from "./composables/useSettingsClient";
import { isSettingsTab, SETTINGS_TABS, type SettingsTab } from "./types";
import type { SettingsHit } from "./search";
import "./settings.css";

const emit = defineEmits<{ (e: "close"): void }>();

// ================= 标签（记住上次所在页） =================

const TAB_KEY = "mml.settingsTab";

function storedTab(): SettingsTab {
  try {
    const raw = localStorage.getItem(TAB_KEY);
    return isSettingsTab(raw) ? raw : "ui";
  } catch {
    return "ui";
  }
}

const tab = ref<SettingsTab>(storedTab());

watch(tab, (v) => {
  try {
    localStorage.setItem(TAB_KEY, v);
  } catch {
    // 存储不可用不影响使用
  }
});

// ================= 各区域状态 =================

const ui = useSettingsUi();
const skin = useSettingsSkin();
const network = useSettingsNetwork();
const launch = useSettingsLaunch();
const java = useSettingsJava();
const client = useSettingsClient();

// 游戏标题那组显示在客户端设置页，但数据属于 window 设置：这里取出给 ClientTab
const { win: launchWin } = launch;

/** 支持"恢复默认"的分组 id（由各 composable 汇总，SettingsGroup 据此决定显不显示按钮） */
const resettableGroups = computed(() => [
  ...ui.resettableGroups,
  ...skin.resettableGroups,
  ...network.resettableGroups,
  ...launch.resettableGroups,
  ...java.resettableGroups,
  ...client.resettableGroups,
]);

/** 刚恢复默认 / 搜索命中的分组：短暂高亮 */
const flashGroup = ref("");
let flashTimer: number | null = null;

function flash(id: string) {
  flashGroup.value = id;
  if (flashTimer !== null) clearTimeout(flashTimer);
  flashTimer = window.setTimeout(() => {
    flashTimer = null;
    flashGroup.value = "";
  }, 1600);
}

/** 各区域的 composable（顺序即"恢复默认"的派发顺序） */
function handlers() {
  return [ui, skin, network, launch, java, client];
}

/** 分组级"恢复默认"：按注册顺序派发给各 composable，谁认领谁处理 */
async function onResetGroup(id: string) {
  for (const h of handlers()) {
    if (await h.resetGroup(id)) {
      flash(id);
      showToast(t("winSettings.resetDone"));
      return;
    }
  }
}

/** 全部恢复默认：逐个分组派发（已确认过一次，不再逐组提示） */
const confirmResetAll = ref(false);

async function onResetAll() {
  confirmResetAll.value = false;
  for (const id of resettableGroups.value) {
    for (const h of handlers()) {
      if (await h.resetGroup(id)) break;
    }
  }
  showToast(t("winSettings.resetDone"));
}

// ================= 设置项搜索 =================

/** 命中搜索项：切到它的标签页，滚到分组锚点，短暂高亮 */
async function onSearchSelect(hit: SettingsHit) {
  tab.value = hit.tab;
  // 目标标签页是 v-if 挂上来的：等它渲染完再找锚点
  await nextTick();
  const el = document.getElementById(`set-${hit.group}`);
  el?.scrollIntoView({ block: "start", behavior: "smooth" });
  flash(hit.group);
}

// ================= 初始化 =================

onMounted(async () => {
  document.addEventListener("keydown", onKeyDown);
  // 各区域的加载互不依赖，一起发（失败的各自留空）
  await Promise.all([
    network.load(),
    launch.load(),
    java.load(),
    client.load(),
    client.refreshLockInstances(),
    client.refreshCustomHome(),
  ]);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onKeyDown);
  if (flashTimer !== null) clearTimeout(flashTimer);
});

/** 切到客户端设置页时刷新实例列表 / 自定义主页状态（设置窗口开着时它们可能已被改动） */
watch(tab, (v) => {
  if (v !== "client") return;
  void client.refreshLockInstances();
  void client.refreshCustomHome();
});

// ================= Esc 关窗 =================

function onKeyDown(e: KeyboardEvent) {
  if (e.key !== "Escape") return;
  // 有弹窗时先让弹窗处理（Java 页的确认框自己听 Esc），别把整扇窗关掉
  if (document.querySelector(".modal-mask")) return;
  emit("close");
}
</script>

<template>
  <WindowFrame :title="t('features.settings')" body-fill @close="emit('close')">
    <div class="settings-layout">
      <SettingsRail v-model="tab" :tabs="SETTINGS_TABS" />

      <!-- 右侧内容区：每个标签一个面板，独立滚动 -->
      <div class="tab-content">
        <!-- 当前页大标题 + 全局恢复默认 + 设置项搜索（跨标签搜，命中跳转并高亮） -->
        <header class="set-content-head">
          <div>
            <h2 class="set-content-title">{{ t(`winSettings.tab.${tab}`) }}</h2>
            <p class="set-content-sub">{{ t(`winSettings.tabDesc.${tab}`) }}</p>
          </div>
          <div class="set-head-actions">
            <BaseButton size="sm" v-tip="t('winSettings.resetAllHint')" @click="confirmResetAll = true">
              {{ t("winSettings.resetAll") }}
            </BaseButton>
            <SettingsSearch @select="onSearchSelect" />
          </div>
        </header>

        <section class="set-panel">
          <UiTab
            v-if="tab === 'ui'"
            :settings="ui"
            :resettable="resettableGroups"
            :flash-group="flashGroup"
            @reset="onResetGroup"
          />
          <JavaTab
            v-else-if="tab === 'java'"
            :settings="java"
            :resettable="resettableGroups"
            :flash-group="flashGroup"
            @reset="onResetGroup"
          />
          <NetworkTab
            v-else-if="tab === 'network'"
            :settings="network"
            :resettable="resettableGroups"
            :flash-group="flashGroup"
            @reset="onResetGroup"
          />
          <LaunchTab
            v-else-if="tab === 'launch'"
            :settings="launch"
            :resettable="resettableGroups"
            :flash-group="flashGroup"
            @reset="onResetGroup"
          />
          <SkinTab
            v-else-if="tab === 'skin'"
            :settings="skin"
            :resettable="resettableGroups"
            :flash-group="flashGroup"
            @reset="onResetGroup"
          />
          <ClientTab
            v-else
            :settings="client"
            :win="launchWin"
            :resettable="resettableGroups"
            :flash-group="flashGroup"
            @reset="onResetGroup"
          />
        </section>
      </div>
    </div>

    <!-- 全部恢复默认：影响面大，先确认 -->
    <BaseModal
      v-if="confirmResetAll"
      :title="t('winSettings.resetAllTitle')"
      @close="confirmResetAll = false"
    >
      <p class="modal-text">{{ t("winSettings.resetAllConfirm") }}</p>
      <div class="modal-actions">
        <BaseButton @click="confirmResetAll = false">{{ t("actions.cancel") }}</BaseButton>
        <BaseButton variant="danger" @click="onResetAll">{{ t("winSettings.resetAll") }}</BaseButton>
      </div>
    </BaseModal>
  </WindowFrame>
</template>
