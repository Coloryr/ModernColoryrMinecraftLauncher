<script setup lang="ts">
import { ref } from "vue";
import WindowFrame from "../components/ui/WindowFrame.vue";
import BaseButton from "../components/ui/BaseButton.vue";
import { t } from "../lib/i18n";

const uploading = ref(false);
const uploaded = ref(false);

async function uploadSkin() {
  uploading.value = true;
  await new Promise((r) => setTimeout(r, 600));
  uploading.value = false;
  uploaded.value = true;
}
</script>

<template>
  <WindowFrame :title="t('features.skin')" @close="$emit('close')">
    <div class="skin-layout">
      <!-- 3D 预览占位 -->
      <div class="preview">
        <div class="preview-stage">
          <div class="preview-figure">
            <div class="head"></div>
            <div class="body"></div>
            <div class="arm left"></div>
            <div class="arm right"></div>
            <div class="leg left"></div>
            <div class="leg right"></div>
          </div>
          <p class="preview-tip">{{ t("winSkin.tip") }}</p>
        </div>
      </div>

      <!-- 皮肤信息 -->
      <div class="skin-info">
        <h2 class="info-title">{{ t("winSkin.current") }}</h2>
        <p class="info-line">{{ t("winSkin.name") }}</p>
        <p class="info-line">{{ t("winSkin.model") }}</p>
        <p class="info-line">{{ t("winSkin.source") }}</p>

        <div class="info-actions">
          <BaseButton variant="primary" :disabled="uploading" @click="uploadSkin">
            {{ uploading ? t("winSkin.uploading") : t("winSkin.upload") }}
          </BaseButton>
          <BaseButton variant="plain">{{ t("winSkin.reset") }}</BaseButton>
        </div>
        <p v-if="uploaded" class="ok-tip">{{ t("winSkin.applied") }}</p>

        <div class="divider"></div>
        <h2 class="info-title">{{ t("winSkin.descTitle") }}</h2>
        <p class="info-line dim">{{ t("winSkin.desc") }}</p>
      </div>
    </div>
  </WindowFrame>
</template>

<style scoped>
.skin-layout {
  display: flex;
  gap: 22px;
  flex-wrap: wrap;
}

.preview {
  flex: 1;
  min-width: 280px;
}

.preview-stage {
  height: 380px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  position: relative;
  overflow: hidden;
}

/* 简易人形占位 */
.preview-figure {
  position: relative;
  width: 90px;
  height: 200px;
}

.preview-figure .head {
  position: absolute;
  top: 0;
  left: 22px;
  width: 46px;
  height: 46px;
  background: #e8b98a;
  border-radius: 10px;
}

.preview-figure .body {
  position: absolute;
  top: 52px;
  left: 20px;
  width: 50px;
  height: 62px;
  background: #3f8cff;
  border-radius: 8px;
}

.preview-figure .arm {
  position: absolute;
  top: 52px;
  width: 16px;
  height: 58px;
  background: #3f8cff;
  border-radius: 6px;
}

.preview-figure .arm.left {
  left: 2px;
}

.preview-figure .arm.right {
  right: 2px;
}

.preview-figure .leg {
  position: absolute;
  top: 116px;
  width: 20px;
  height: 60px;
  background: #4a5568;
  border-radius: 6px;
}

.preview-figure .leg.left {
  left: 20px;
}

.preview-figure .leg.right {
  right: 20px;
}

.preview-tip {
  position: absolute;
  bottom: 16px;
  font-size: 12px;
  color: var(--text-dim);
}

.skin-info {
  width: 300px;
}

.info-title {
  font-size: 14px;
  font-weight: 700;
  margin-bottom: 10px;
}

.info-line {
  font-size: 13px;
  color: var(--text);
  margin-bottom: 8px;
}

.info-line.dim {
  color: var(--text-dim);
  line-height: 1.7;
}

.info-actions {
  display: flex;
  gap: 10px;
  margin-top: 14px;
}

.ok-tip {
  font-size: 12.5px;
  color: var(--green);
  margin-top: 10px;
}

.divider {
  height: 1px;
  background: var(--border);
  margin: 20px 0;
}
</style>
