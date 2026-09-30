<script setup lang="ts">
// 设置窗口 · 游戏启动标签（游戏窗口 / 内存 / JVM / 游戏参数 / 启动命令）
//
// 内存冲突（最小 > 最大）时把两个步进器标红并给出提示，保存也会被拦住（见 useSettingsLaunch）；
// 冲突出现时把焦点落到"要改小"的最小内存上（NumberStepper 的输入框在组件内部，取 $el 里的 input）。
import { nextTick, ref, watch } from "vue";
import { t } from "../../../../lib/i18n";
import BaseSwitch from "../../../../components/ui/BaseSwitch.vue";
import NumberStepper from "../../../../components/ui/NumberStepper.vue";
import SegmentedTabs from "../../../../components/ui/SegmentedTabs.vue";
import SettingsGroup from "../SettingsGroup.vue";
import SaveStateText from "../SaveStateText.vue";
import type { useSettingsLaunch } from "../../composables/useSettingsLaunch";

const props = defineProps<{
  settings: ReturnType<typeof useSettingsLaunch>;
  resettable: string[];
  flashGroup?: string;
}>();

const emit = defineEmits<{ (e: "reset", id: string): void }>();

const {
  run,
  win,
  envLines,
  gcOptions,
  memoryConflict,
  commitEnvLines,
  addEnvLine,
  removeEnvLine,
  saveState,
} = props.settings;

/** 内存冲突时聚焦最小内存（要改的就是它） */
const minMemoryRef = ref<InstanceType<typeof NumberStepper> | null>(null);
watch(memoryConflict, async (bad) => {
  if (!bad) return;
  await nextTick();
  const root = minMemoryRef.value?.$el as HTMLElement | undefined;
  root?.querySelector("input")?.focus();
});
</script>

<template>
  <template v-if="run && win">
    <SettingsGroup
      id="gameWindow"
      title-key="winSettings.secGameWindow"
      :resettable="resettable.includes('gameWindow')"
      :flash="flashGroup === 'gameWindow'"
      @reset="emit('reset', $event)"
    >
      <div class="switch-row">
        <span>{{ t("winSettings.fullScreen") }}</span>
        <BaseSwitch v-model="win!.fullScreen" />
      </div>
      <div class="grid-2 mt-10">
        <div>
          <label class="field-label">{{ t("winSettings.width") }}</label>
          <NumberStepper v-model="win!.width" :min="100" :max="65535" />
        </div>
        <div>
          <label class="field-label">{{ t("winSettings.height") }}</label>
          <NumberStepper v-model="win!.height" :min="100" :max="65535" />
        </div>
      </div>
    </SettingsGroup>

    <SettingsGroup
      id="memory"
      title-key="winSettings.secMemory"
      :resettable="resettable.includes('memory')"
      :flash="flashGroup === 'memory'"
      @reset="emit('reset', $event)"
    >
      <div class="grid-2">
        <div :class="{ 'field-invalid': memoryConflict }">
          <label class="field-label no-top">{{ t("winSettings.minMemory") }}</label>
          <NumberStepper ref="minMemoryRef" v-model="run!.minMemory" :min="256" :max="65536" :step="256" />
        </div>
        <div :class="{ 'field-invalid': memoryConflict }">
          <label class="field-label no-top">{{ t("winSettings.maxMemory") }}</label>
          <NumberStepper v-model="run!.maxMemory" :min="256" :max="65536" :step="256" />
        </div>
      </div>
      <!-- 冲突时明确说清"没保存"，避免用户以为改上了 -->
      <p v-if="memoryConflict" class="lock-server-hint">{{ t("winSettings.memoryConflict") }}</p>
      <div class="set-save-row right">
        <SaveStateText :state="saveState" />
      </div>
    </SettingsGroup>

    <SettingsGroup
      id="jvm"
      title-key="winSettings.secJvm"
      :resettable="resettable.includes('jvm')"
      :flash="flashGroup === 'jvm'"
      @reset="emit('reset', $event)"
    >
      <div class="grid-2">
        <div>
          <label class="field-label no-top">{{ t("winSettings.gcMode") }}</label>
          <SegmentedTabs v-model="run!.gcMode" :options="gcOptions" />
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

      <label class="field-label mt-14">{{ t("winSettings.jvmEnv") }}</label>
      <div class="dns-lines">
        <div v-for="(_, i) in envLines" :key="i" class="line-row">
          <input
            v-model="envLines[i].key"
            class="field-input grow"
            spellcheck="false"
            autocomplete="off"
            :placeholder="t('winSettings.envKey')"
            @change="commitEnvLines"
          />
          <input
            v-model="envLines[i].value"
            class="field-input grow"
            spellcheck="false"
            autocomplete="off"
            :placeholder="t('winSettings.envValue')"
          />
          <button class="line-del" aria-label="remove" v-tip="t('args.removeLine')" @click="removeEnvLine(i)">✕</button>
        </div>
        <button class="line-add" @click="addEnvLine">＋ {{ t("args.addLine") }}</button>
      </div>

      <label class="field-label mt-10">{{ t("winSettings.jvmArgs") }}</label>
      <textarea v-model="run!.jvmArgs" class="field-input args-input" spellcheck="false" />
    </SettingsGroup>

    <SettingsGroup
      id="gameArgs"
      title-key="winSettings.secGameArgs"
      :resettable="resettable.includes('gameArgs')"
      :flash="flashGroup === 'gameArgs'"
      @reset="emit('reset', $event)"
    >
      <div class="switch-row">
        <span>{{ t("winSettings.removeGameArg") }}</span>
        <BaseSwitch v-model="run!.removeGameArg" />
      </div>
      <label class="field-label mt-10">{{ t("winSettings.gameArgs") }}</label>
      <textarea v-model="run!.gameArgs" class="field-input args-input" spellcheck="false" />
    </SettingsGroup>

    <SettingsGroup
      id="launchCmd"
      title-key="winSettings.secLaunchCmd"
      :resettable="resettable.includes('launchCmd')"
      :flash="flashGroup === 'launchCmd'"
      @reset="emit('reset', $event)"
    >
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
        <label class="field-label mt-10">{{ t("winSettings.preCmd") }}</label>
        <input v-model="run!.preRunArg" class="field-input" spellcheck="false" autocomplete="off" />
      </template>
      <template v-if="run.launchPostRun">
        <label class="field-label mt-10">{{ t("winSettings.postCmd") }}</label>
        <input v-model="run!.postRunArg" class="field-input" spellcheck="false" autocomplete="off" />
      </template>
    </SettingsGroup>
  </template>
  <p v-else class="field-desc">{{ t("winSettings.tauriOnly") }}</p>
</template>
