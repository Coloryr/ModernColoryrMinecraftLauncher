<script setup lang="ts">
// 自定义主页面：把服主导入的整页 HTML 放进 iframe 顶替内置主页
//
// 故意不给 allow-same-origin：iframe 的 origin 保持 opaque，页面拿不到启动器页面的
// localStorage / cookie；postMessage 在 opaque origin 下照常可用。
// 页面里的 window.mml（invoke / on / ready）由 Rust 注入的 custom_home_bridge.js 提供，
// 父窗口侧的处理在 lib/customHomeBridge.ts。
import { onMounted, onUnmounted, ref } from "vue";
import { attachCustomHomeBridge } from "../lib/customHomeBridge";

defineProps<{ entryUrl: string }>();

const frame = ref<HTMLIFrameElement | null>(null);
let detach: (() => void) | null = null;

onMounted(() => {
  if (frame.value) detach = attachCustomHomeBridge(frame.value);
});

onUnmounted(() => {
  detach?.();
  detach = null;
});
</script>

<template>
  <iframe ref="frame" class="custom-home-frame" :src="entryUrl"
    sandbox="allow-scripts allow-forms allow-popups allow-modals"></iframe>
</template>

<style scoped>
/* 铺满内容区：父容器都是「确定高度的纵向 flex」（.custom-home-page / .custom-home-fill），
   高度靠 flex:1 吃满，height:100% 只是兜底 */
.custom-home-frame {
  flex: 1;
  width: 100%;
  height: 100%;
  min-height: 0;
  border: 0;
  display: block;
  background: var(--bg);
}
</style>
