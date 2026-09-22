<script setup lang="ts">
import { computed, onUnmounted, ref, watch } from "vue";
import { getImageBaseUrl, onInstanceChange } from "../lib/api";

const props = withDefaults(
  defineProps<{
    name: string;
    uuid: string;
    size?: number;
  }>(),
  { size: 48 },
);

// 根据 uuid 稳定取色，保证同一实例图标一致（图标缺失 / 加载失败时的回退）
const gradients = [
  "linear-gradient(135deg, #3f8cff, #5f6cff)",
  "linear-gradient(135deg, #34d399, #22d3ee)",
  "linear-gradient(135deg, #f59e0b, #ef4444)",
  "linear-gradient(135deg, #a855f7, #ec4899)",
  "linear-gradient(135deg, #14b8a6, #3b82f6)",
  "linear-gradient(135deg, #f97316, #f43f5e)",
  "linear-gradient(135deg, #84cc16, #22c55e)",
  "linear-gradient(135deg, #06b6d4, #6366f1)",
];

const palette = computed(() => {
  let h = 0;
  for (const c of props.uuid) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return gradients[h % gradients.length];
});

const char = computed(() => props.name.trim().charAt(0).toUpperCase() || "M");

// ---------- 真实图标加载 ----------

// mcml-image 协议前缀（浏览器预览无 IPC 时取不到，走回退渐变）
const base = ref("");
const failed = ref(false);
/** 缓存破坏参数（图标被更换后刷新） */
const ver = ref(0);

const url = computed(() =>
  base.value ? `${base.value}/instance/${props.uuid}?v=${ver.value}` : "",
);

// onUnmounted 必须在 setup 同步期注册，await 后再调会失去组件实例，
// 所以注销函数先存下来，由同步注册的钩子代为调用
let unlistenChange: (() => void) | null = null;
onUnmounted(() => unlistenChange?.());

(async () => {
  try {
    base.value = await getImageBaseUrl();
  } catch {
    base.value = "";
  }

  // 实例图标被更换（edit）后刷新；事件无 uuid，低频操作统一刷新即可
  unlistenChange = await onInstanceChange((type) => {
    if (type !== "edit") return;
    ver.value = Date.now();
    failed.value = false;
  });
})();

watch(
  () => props.uuid,
  () => {
    failed.value = false;
  },
);
</script>

<template>
  <div
    class="inst-icon"
    :style="{
      width: size + 'px',
      height: size + 'px',
      background: palette,
      fontSize: Math.round(size * 0.42) + 'px',
      borderRadius: Math.round(size * 0.24) + 'px',
    }"
  >
    <img
      v-if="url && !failed"
      class="inst-icon-img"
      :src="url"
      alt=""
      @error="failed = true"
    />
    <span v-if="failed || !url">{{ char }}</span>
  </div>
</template>

<style scoped>
.inst-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: 700;
  flex-shrink: 0;
  box-shadow: 0 2px 6px rgba(0, 0, 0, 0.15);
  border: 1px solid rgba(255, 255, 255, 0.14);
  user-select: none;
  overflow: hidden;
  position: relative;
}

.inst-icon-img {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  object-fit: cover;
  /* 实例图标多为低分辨率像素图，平滑缩放会发糊 */
  image-rendering: pixelated;
}

/* 简单的高光装饰 */
.inst-icon::after {
  content: "";
  position: absolute;
  inset: 0;
  background: linear-gradient(180deg, rgba(255, 255, 255, 0.22), rgba(255, 255, 255, 0) 45%);
  pointer-events: none;
}
</style>
