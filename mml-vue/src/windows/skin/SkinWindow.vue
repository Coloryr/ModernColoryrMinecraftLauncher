<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import AccountSelector from "../../components/AccountSelector.vue";
import { t } from "../../lib/i18n";
import { accounts, currentAccount, loadAccounts, typeLabelKey } from "../../lib/accountStore";
import type { AccountStoreDto } from "../../lib/bindings";
import { getImageBaseUrl } from "../../lib/api";
import * as skinview3d from "skinview3d";

/** 展示模式 */
type Mode = "2d" | "3d";

/** 皮肤类型（auto = 自动检测；old = 1.7旧版；new = 1.8新版经典；slim = 纤细） */
type SkinTypeOpt = "auto" | "old" | "new" | "slim";

const mode = ref<Mode>("3d");
/** 是否渲染披风 */
const showCape = ref(true);
/** 是否播放动画（仅3D） */
const playAnim = ref(true);

/** 皮肤类型选项（URI段值 + i18n键） */
const SKIN_TYPES: Array<{ value: SkinTypeOpt; labelKey: string }> = [
  { value: "auto", labelKey: "winSkin.skinAuto" },
  { value: "new", labelKey: "winSkin.skinClassic" },
  { value: "slim", labelKey: "winSkin.skinSlim" },
  { value: "old", labelKey: "winSkin.skinOld" },
];

const skinType = ref<SkinTypeOpt>("auto");
const base = ref("");
const account = ref<AccountStoreDto | null>(null);
/** 皮肤获取失败（离线账户 / 下载失败） */
const noSkin = ref(false);

const skin2dUrl = computed(() =>
  account.value && base.value
    ? `${base.value}/skin2d/${account.value.authType}/${account.value.uuid}/${skinType.value}`
    : "",
);
const skinrawUrl = computed(() =>
  account.value && base.value
    ? `${base.value}/skinraw/${account.value.authType}/${account.value.uuid}`
    : "",
);
const caperawUrl = computed(() =>
  account.value && base.value
    ? `${base.value}/caperaw/${account.value.authType}/${account.value.uuid}`
    : "",
);
const cape2dUrl = computed(() =>
  account.value && base.value
    ? `${base.value}/cape2d/${account.value.authType}/${account.value.uuid}`
    : "",
);

// ---- 3D 渲染（skinview3d） ----
const stage = ref<HTMLDivElement | null>(null);
const canvas = ref<HTMLCanvasElement | null>(null);
let viewer: skinview3d.SkinViewer | null = null;

function destroyViewer() {
  viewer?.dispose();
  viewer = null;
}

/** 皮肤类型 → skinview3d 模型（1.7旧版没有纤细臂，走自动检测） */
const modelOpt = computed(() => {
  switch (skinType.value) {
    case "slim":
      return "slim" as const;
    case "new":
      return "default" as const;
    default:
      return "auto-detect" as const;
  }
});

function setupViewer() {
  destroyViewer();
  noSkin.value = false;
  if (mode.value !== "3d" || !canvas.value || !stage.value || !skinrawUrl.value) return;

  viewer = new skinview3d.SkinViewer({
    canvas: canvas.value,
    width: stage.value.clientWidth,
    height: stage.value.clientHeight,
    model: modelOpt.value,
    zoom: 0.85,
  });
  viewer.autoRotate = true;
  viewer.autoRotateSpeed = 1.2;
  viewer.animation = playAnim.value ? new skinview3d.WalkingAnimation() : null;

  viewer.loadSkin(skinrawUrl.value).catch(() => {
    noSkin.value = true;
  });
  if (showCape.value) {
    // 没有披风的账户加载失败是正常的，静默忽略
    viewer.loadCape(caperawUrl.value).catch(() => {});
  }
}

watch([mode, skinrawUrl, modelOpt], () => setupViewer());

// 披风开关：不重建观察器，直接挂载/卸载
watch(showCape, () => {
  if (!viewer || mode.value !== "3d") return;
  if (showCape.value && caperawUrl.value) {
    viewer.loadCape(caperawUrl.value).catch(() => {});
  } else {
    viewer.loadCape(null);
  }
});

// 动画开关：切动画类型 / 清空
watch(playAnim, () => {
  if (!viewer) return;
  viewer.animation = playAnim.value ? new skinview3d.WalkingAnimation() : null;
});

// 2D 图片换源时清除上一次的失败标记
watch(skin2dUrl, () => {
  noSkin.value = false;
});

onMounted(async () => {
  await loadAccounts();
  account.value = currentAccount.value;
  base.value = await getImageBaseUrl();
  setupViewer();
});

onBeforeUnmount(destroyViewer);

function pickAccount(acc: AccountStoreDto) {
  account.value = acc;
}
</script>

<template>
  <WindowFrame :title="t('features.skin')" @close="$emit('close')">
    <div class="skin-layout">
      <!-- 预览区：2D / 3D -->
      <div class="preview">
        <div class="mode-toggle">
          <button
            class="mode-btn"
            :class="{ active: mode === '3d' }"
            @click="mode = '3d'"
          >
            {{ t("winSkin.mode3d") }}
          </button>
          <button
            class="mode-btn"
            :class="{ active: mode === '2d' }"
            @click="mode = '2d'"
          >
            {{ t("winSkin.mode2d") }}
          </button>
        </div>

        <div class="mode-toggle">
          <span class="mode-label">{{ t("winSkin.skinType") }}</span>
          <button
            v-for="opt in SKIN_TYPES"
            :key="opt.value"
            class="mode-btn"
            :class="{ active: skinType === opt.value }"
            @click="skinType = opt.value"
          >
            {{ t(opt.labelKey) }}
          </button>
        </div>

        <div class="mode-toggle">
          <label class="mode-label toggle-label">
            <input v-model="showCape" type="checkbox" />
            {{ t("winSkin.showCape") }}
          </label>
          <label v-if="mode === '3d'" class="mode-label toggle-label">
            <input v-model="playAnim" type="checkbox" />
            {{ t("winSkin.playAnim") }}
          </label>
        </div>

        <div ref="stage" class="preview-stage">
          <canvas v-show="mode === '3d'" ref="canvas" class="gl-canvas"></canvas>
          <template v-if="mode === '2d'">
            <img
              v-if="skin2dUrl"
              :src="skin2dUrl"
              class="flat-skin"
              @load="noSkin = false"
              @error="noSkin = true"
            />
            <img
              v-if="showCape && cape2dUrl"
              :src="cape2dUrl"
              class="flat-cape"
              alt=""
            />
          </template>
          <p v-if="noSkin" class="preview-tip">{{ t("winSkin.noSkin") }}</p>
        </div>
        <p v-if="mode === '3d' && !noSkin" class="stage-hint">{{ t("winSkin.hint") }}</p>
      </div>

      <!-- 信息面板 -->
      <div class="skin-info">
        <h2 class="info-title">{{ t("winSkin.account") }}</h2>
        <AccountSelector
          :account="account"
          :accounts="accounts"
          @update:account="pickAccount"
        />

        <div class="divider"></div>
        <p class="info-line">
          {{ account ? account.userName : t("account.noAccount") }}
        </p>
        <p v-if="account" class="info-line dim">
          {{ t(typeLabelKey(account.authType)) }} · {{ account.uuid }}
        </p>
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
  min-width: 300px;
}

.mode-toggle {
  display: inline-flex;
  gap: 4px;
  padding: 4px;
  margin-bottom: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
}

.mode-btn {
  padding: 5px 16px;
  font-size: 13px;
  color: var(--text-dim);
  background: transparent;
  border: 0;
  border-radius: 7px;
  cursor: pointer;
}

.mode-label {
  padding: 5px 10px;
  font-size: 13px;
  color: var(--text);
}

.toggle-label {
  display: flex;
  align-items: center;
  gap: 6px;
  cursor: pointer;
  user-select: none;
}

.mode-btn.active {
  color: var(--text);
  background: var(--border);
}

.preview-stage {
  height: 400px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  display: flex;
  align-items: center;
  justify-content: center;
  position: relative;
  overflow: hidden;
}

.gl-canvas {
  width: 100%;
  height: 100%;
}

.flat-skin {
  max-width: 70%;
  max-height: 100%;
  image-rendering: pixelated;
}

.flat-cape {
  max-width: 25%;
  max-height: 60%;
  image-rendering: pixelated;
}

.preview-tip {
  position: absolute;
  bottom: 16px;
  left: 0;
  right: 0;
  text-align: center;
  font-size: 12px;
  color: var(--text-dim);
}

.stage-hint {
  margin-top: 8px;
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
  word-break: break-all;
}

.info-line.dim {
  color: var(--text-dim);
  line-height: 1.7;
}

.divider {
  height: 1px;
  background: var(--border);
  margin: 20px 0;
}
</style>
