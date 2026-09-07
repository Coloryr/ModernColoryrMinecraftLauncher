<script setup lang="ts">
// 启动画面：初始化中显示 splash，初始化失败显示错误页（splashError），
// 引导表单（数据目录 / 玩家名）用于重新初始化
import { t } from "../../lib/i18n";
import BaseButton from "./BaseButton.vue";

withDefaults(
  defineProps<{
    splashVisible: boolean;
    bootFailed: boolean;
    splashError: string;
    initLoading: boolean;
    initError: string;
    localDir: string;
    playerName: string;
  }>(),
  {
    splashVisible: true,
    bootFailed: false,
    splashError: "",
    initLoading: false,
    initError: "",
    localDir: "",
    playerName: "",
  },
);

const emit = defineEmits<{
  (e: "update:localDir", v: string): void;
  (e: "update:playerName", v: string): void;
  (e: "retry"): void;
  (e: "retryBoot"): void;
}>();
</script>

<template>
  <!-- 启动画面（初始化中，由 closeSplash() 关闭） -->
  <div v-if="splashVisible" class="splash">
    <div class="splash-logo">MC</div>
    <div class="splash-name">{{ t("app.name") }}</div>
    <div class="splash-spinner"></div>
    <div class="splash-text">{{ t("init.splash") }}</div>
  </div>

  <!-- 初始化失败错误页（closeSplash(error) 后显示） -->
  <div v-else-if="splashError" class="boot-error">
    <div class="boot-error-card">
      <div class="boot-error-icon">!</div>
      <h1>{{ t("init.failed") }}</h1>
      <p class="boot-error-text">{{ splashError }}</p>
      <BaseButton variant="primary" size="lg" block @click="emit('retryBoot')">
        {{ t("init.retry") }}
      </BaseButton>
    </div>
  </div>

  <!-- 初始化引导（仅初始化失败时出现） -->
  <div v-else-if="bootFailed" class="setup">
    <div class="setup-card">
      <div class="setup-logo">MCML</div>
      <h1>{{ t("app.name") }}</h1>
      <p class="setup-sub">{{ t("init.title") }}</p>

      <label class="field-label">{{ t("init.dataDir") }}</label>
      <input
        :value="localDir"
        class="field-input"
        :placeholder="t('init.dataDirPlaceholder')"
        spellcheck="false"
        @input="emit('update:localDir', ($event.target as HTMLInputElement).value)"
      />

      <label class="field-label">{{ t("init.playerName") }}</label>
      <input
        :value="playerName"
        class="field-input"
        :placeholder="t('init.playerNamePlaceholder')"
        spellcheck="false"
        @input="emit('update:playerName', ($event.target as HTMLInputElement).value)"
      />

      <p v-if="initError" class="error-text">{{ initError }}</p>

      <BaseButton variant="primary" size="lg" block :disabled="initLoading" @click="emit('retry')">
        {{ initLoading ? t("init.initializing") : t("init.button") }}
      </BaseButton>
    </div>
  </div>
</template>

<style scoped>
.splash {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 14px;
  background: linear-gradient(160deg, var(--bg-side) 0%, var(--bg) 100%);
}

.splash-logo {
  width: 72px;
  height: 72px;
  border-radius: 18px;
  background: linear-gradient(135deg, #4f8cff, #7c5cff);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 800;
  font-size: 30px;
  color: #fff;
  box-shadow: 0 8px 24px rgba(79, 140, 255, 0.35);
}

.splash-name {
  font-size: 20px;
  font-weight: 700;
  color: var(--text);
}

.splash-spinner {
  width: 26px;
  height: 26px;
  border-radius: 50%;
  border: 3px solid var(--border);
  border-top-color: var(--accent);
  animation: splash-rotate 0.9s linear infinite;
  margin-top: 8px;
}

@keyframes splash-rotate {
  to {
    transform: rotate(360deg);
  }
}

.splash-text {
  font-size: 13px;
  color: var(--text-dim);
}

/* 初始化失败错误页 */
.boot-error {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(160deg, var(--bg-side) 0%, var(--bg) 100%);
}

.boot-error-card {
  width: 440px;
  padding: 36px;
  border-radius: 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
  align-items: center;
}

.boot-error-icon {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  background: rgba(255, 99, 99, 0.14);
  color: #ff6363;
  font-size: 30px;
  font-weight: 800;
  display: flex;
  align-items: center;
  justify-content: center;
}

.boot-error-card h1 {
  font-size: 18px;
  margin: 14px 0 10px;
  color: var(--text);
}

.boot-error-text {
  width: 100%;
  margin-bottom: 22px;
  padding: 12px 14px;
  border-radius: 8px;
  background: var(--bg-side);
  border: 1px solid var(--border);
  color: var(--text-dim);
  font-size: 13px;
  word-break: break-all;
  max-height: 180px;
  overflow-y: auto;
}

/* 初始化界面 */
.setup {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(160deg, var(--bg-side) 0%, var(--bg) 100%);
}

.setup-card {
  width: 400px;
  padding: 36px;
  border-radius: 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  box-shadow: var(--shadow-lg);
}

.setup-logo {
  font-size: 34px;
  font-weight: 800;
  letter-spacing: 2px;
  background: linear-gradient(120deg, #4f8cff, #7c5cff);
  -webkit-background-clip: text;
  background-clip: text;
  color: transparent;
  text-align: center;
}

.setup-card h1 {
  font-size: 20px;
  text-align: center;
  margin: 10px 0 4px;
}

.setup-sub {
  color: var(--text-dim);
  font-size: 13px;
  text-align: center;
  margin-bottom: 22px;
}
</style>
