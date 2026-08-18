<script setup lang="ts">
import { ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import { t, locale, setLocale } from "../../lib/i18n";
import { multiWindow, setMultiWindow, openWindow, isTauri } from "../windowManager";
import { sidebarSide, setSidebarSide } from "../../lib/settings";
import { ACCENTS, accent, setAccent } from "../../lib/theme";
import BaseButton from "../../components/ui/BaseButton.vue";

const inTauri = isTauri();
const windowMode = ref(multiWindow.value ? "multi" : "single");
const side = ref(sidebarSide.value);

function onModeChange(v: string) {
  windowMode.value = v;
  setMultiWindow(v === "multi");
}

function onLangChange(v: string) {
  setLocale(v === "en-US" ? "en-US" : "zh-CN");
}

function onSideChange(v: string) {
  side.value = v === "right" ? "right" : "left";
  setSidebarSide(side.value);
}

function testWindow() {
  openWindow("skin");
}
</script>

<template>
  <WindowFrame :title="t('features.settings')" @close="$emit('close')">
    <section class="card">
      <h2 class="card-title">{{ t("winSettings.language") }}</h2>
      <SegmentedTabs
        :model-value="locale"
        :options="[
          { value: 'zh-CN', label: '简体中文' },
          { value: 'en-US', label: 'English' },
        ]"
        @update:model-value="onLangChange"
      />
    </section>

    <section class="card">
      <h2 class="card-title">{{ t("winSettings.accent") }}</h2>
      <p class="card-desc">{{ t("winSettings.accentDesc") }}</p>
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
      </div>
    </section>

    <section class="card">
      <h2 class="card-title">{{ t("winSettings.windowMode") }}</h2>
      <template v-if="inTauri">
        <p class="card-desc">{{ t("winSettings.tauriFixed") }}</p>
        <p class="hint">{{ t("winSettings.currentMulti") }}</p>
      </template>
      <template v-else>
        <p class="card-desc">{{ t("winSettings.windowModeDesc") }}</p>
        <SegmentedTabs
          :model-value="windowMode"
          :options="[
            { value: 'single', label: t('winSettings.single') },
            { value: 'multi', label: t('winSettings.multi') },
          ]"
          @update:model-value="onModeChange"
        />
        <p class="hint">
          {{
            windowMode === "multi"
              ? t("winSettings.currentMulti")
              : t("winSettings.currentSingle")
          }}
        </p>
      </template>
      <div class="test-row">
        <BaseButton size="sm" variant="accent" @click="testWindow">测试新窗口</BaseButton>
      </div>
    </section>

    <section class="card">
      <h2 class="card-title">{{ t("winSettings.sidebar") }}</h2>
      <p class="card-desc">{{ t("winSettings.sidebarDesc") }}</p>
      <SegmentedTabs
        :model-value="side"
        :options="[
          { value: 'left', label: t('winSettings.sidebarLeft') },
          { value: 'right', label: t('winSettings.sidebarRight') },
        ]"
        @update:model-value="onSideChange"
      />
    </section>

    <section class="card">
      <h2 class="card-title">{{ t("winSettings.downloadSource") }}</h2>
      <p class="card-desc">{{ t("winSettings.downloadSourceDesc") }}</p>
    </section>

    <section class="card">
      <h2 class="card-title">{{ t("winSettings.network") }}</h2>
      <p class="card-desc">{{ t("winSettings.networkDesc") }}</p>
    </section>

    <section class="card">
      <h2 class="card-title">{{ t("winSettings.defaultJava") }}</h2>
      <p class="card-desc">{{ t("winSettings.defaultJavaDesc") }}</p>
    </section>

    <section class="card">
      <h2 class="card-title">{{ t("winSettings.interface") }}</h2>
      <p class="card-desc">{{ t("winSettings.interfaceDesc") }}</p>
    </section>
  </WindowFrame>
</template>

<style scoped>
.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 18px 20px;
  margin-bottom: 14px;
}

.card-title {
  font-size: 14px;
  font-weight: 700;
  margin-bottom: 6px;
}

.card-desc {
  font-size: 12.5px;
  color: var(--text-dim);
  margin-bottom: 12px;
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

.hint {
  font-size: 12px;
  color: var(--accent);
  margin-top: 10px;
}

.test-row {
  margin-top: 12px;
}
</style>
