<script setup lang="ts">
// 带加载占位的图片：加载中显示流光动画，失败显示灰底；父级把尺寸类挂在组件上即可
import { ref, watch } from "vue";

const props = defineProps<{
  src: string;
  alt?: string;
  title?: string;
}>();

const loaded = ref(false);
const failed = ref(false);

// 换图（如版本筛选刷新截图）时回到占位状态
watch(
  () => props.src,
  () => {
    loaded.value = false;
    failed.value = false;
  },
);
</script>

<template>
  <span class="async-img">
    <img
      v-if="!failed"
      :src="src"
      :alt="alt ?? ''"
      :title="title"
      loading="lazy"
      :class="{ show: loaded }"
      @load="loaded = true"
      @error="failed = true"
    />
    <span v-if="!loaded && !failed" class="async-shimmer"></span>
  </span>
</template>

<style scoped>
.async-img {
  position: relative;
  display: block;
  overflow: hidden;
}

.async-img img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  opacity: 0;
  transition: opacity 0.2s ease;
}

.async-img img.show {
  opacity: 1;
}

/* 流光占位：图盖上来之前一直在动 */
.async-shimmer {
  position: absolute;
  inset: 0;
  background: linear-gradient(100deg, var(--bg-hover) 40%, var(--bg-card) 50%, var(--bg-hover) 60%);
  background-size: 200% 100%;
  animation: async-shine 1.2s linear infinite;
}

@keyframes async-shine {
  to {
    background-position: -200% 0;
  }
}
</style>
