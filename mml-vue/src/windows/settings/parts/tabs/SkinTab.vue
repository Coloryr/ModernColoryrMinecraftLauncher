<script setup lang="ts">
// 设置窗口 · 皮肤与头像标签
import { t } from "../../../../lib/i18n";
import NumberStepper from "../../../../components/ui/NumberStepper.vue";
import SegmentedTabs from "../../../../components/ui/SegmentedTabs.vue";
import SettingsGroup from "../SettingsGroup.vue";
import type { useSettingsSkin } from "../../composables/useSettingsSkin";

const props = defineProps<{
  settings: ReturnType<typeof useSettingsSkin>;
  resettable: string[];
  flashGroup?: string;
}>();

const emit = defineEmits<{ (e: "reset", id: string): void }>();

const {
  imageBase,
  headPreviewUrl,
  skinPreviewUrl,
  headType,
  headX,
  headY,
  setHeadConfig,
  headTypeOptions,
  onHeadTypeChange,
  skinDisplay,
  skinDisplayOptions,
  onSkinDisplayChange,
  offsetAdjustable,
} = props.settings;
</script>

<template>
  <SettingsGroup
    id="skin"
    :resettable="resettable.includes('skin')"
    :flash="flashGroup === 'skin'"
    @reset="emit('reset', $event)"
  >
    <!-- 头像显示模式 -->
    <label class="field-label no-top">{{ t("winSettings.headDisplay") }}</label>
    <p class="field-desc">{{ t("winSettings.headDisplayDesc") }}</p>
    <SegmentedTabs
      :model-value="headType"
      :options="headTypeOptions"
      @update:model-value="onHeadTypeChange"
    />
    <div v-if="offsetAdjustable" class="grid-2 mt-12">
      <div>
        <label class="field-label">{{ t("winSettings.rotX") }}</label>
        <NumberStepper
          v-model="headX"
          :min="-90"
          :max="90"
          @update:model-value="(v) => setHeadConfig(headType, v, headY)"
        />
      </div>
      <div>
        <label class="field-label">{{ t("winSettings.rotY") }}</label>
        <NumberStepper
          v-model="headY"
          :min="-180"
          :max="180"
          @update:model-value="(v) => setHeadConfig(headType, headX, v)"
        />
      </div>
    </div>

    <!-- 皮肤显示模式：2D TypeA / 2D TypeB / 3D -->
    <label class="field-label mt-16">{{ t("winSettings.skinDisplay") }}</label>
    <p class="field-desc">{{ t("winSettings.skinDisplayDesc") }}</p>
    <SegmentedTabs
      :model-value="skinDisplay"
      :options="skinDisplayOptions"
      @update:model-value="onSkinDisplayChange"
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
  </SettingsGroup>
</template>
