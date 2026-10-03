<script setup lang="ts">
// 小图标：原来用文字字形（＋ ✓ ✕ ‹ › ▾ − ×）当图标的地方统一走这里
//
// 为什么不留着字形：
// - 字形形状 / 粗细 / 位置随字体走：中文字体把 ＋ 画在 em 框偏上，盒子居中了笔画仍显偏上
//   （AccountSelector 里已经踩过一次，见那里的注释）；
// - ▾ 这类三角按字号画得太小，粗细还随字号变形；
// - 字号一改图标就变形，而 SVG 的尺寸由 size 定死，线宽由 weight 调。
//
// 统一 24 视窗 + stroke=currentColor（跟随所在处的字体色）+ 圆头圆角，
// 与仓库里其它内联图标（86 处 stroke="currentColor"）同一套画法。
type GlyphName =
  | "plus"
  | "minus"
  | "check"
  | "close"
  | "chevron-left"
  | "chevron-right"
  | "chevron-down"
  | "play"
  | "gear"
  | "document"
  | "news"
  | "check-square"
  | "image"
  | "download";

withDefaults(defineProps<{ name: GlyphName; size?: number; weight?: number }>(), {
  size: 14,
  weight: 2,
});
</script>

<template>
  <svg
    class="glyph"
    :width="size"
    :height="size"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    :stroke-width="weight"
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
  >
    <path v-if="name === 'plus'" d="M12 5v14M5 12h14" />
    <path v-else-if="name === 'minus'" d="M5 12h14" />
    <path v-else-if="name === 'check'" d="m5 13 4 4L19 7" />
    <path v-else-if="name === 'close'" d="M6 6l12 12M18 6L6 18" />
    <path v-else-if="name === 'chevron-left'" d="m15 6-6 6 6 6" />
    <path v-else-if="name === 'chevron-right'" d="m9 6 6 6-6 6" />
    <path v-else-if="name === 'chevron-down'" d="m6 9 6 6 6-6" />
    <path v-else-if="name === 'play'" d="M7.5 5.2v13.6L19 12 7.5 5.2z" />
    <!-- 齿轮与顶栏功能按钮上那个是同一个画法（见 MainTopbar 的 gear） -->
    <g v-else-if="name === 'gear'">
      <circle cx="12" cy="12" r="3.2" />
      <path
        d="M19.4 15a1.7 1.7 0 0 0 .34 1.87l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.7 1.7 0 0 0-1.87-.34 1.7 1.7 0 0 0-1.03 1.56V21a2 2 0 1 1-4 0v-.09a1.7 1.7 0 0 0-1.03-1.56 1.7 1.7 0 0 0-1.87.34l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.7 1.7 0 0 0 .34-1.87 1.7 1.7 0 0 0-1.56-1.03H3a2 2 0 1 1 0-4h.09a1.7 1.7 0 0 0 1.56-1.03 1.7 1.7 0 0 0-.34-1.87l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.7 1.7 0 0 0 1.87.34h.01a1.7 1.7 0 0 0 1.03-1.56V3a2 2 0 1 1 4 0v.09a1.7 1.7 0 0 0 1.03 1.56 1.7 1.7 0 0 0 1.87-.34l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.7 1.7 0 0 0-.34 1.87v.01a1.7 1.7 0 0 0 1.56 1.03H21a2 2 0 1 1 0 4h-.09a1.7 1.7 0 0 0-1.56 1.03z"
      />
    </g>
    <!-- 文档（实例日志）：纸张轮廓与 FileTree 的文件图标同款，另加两行文字 -->
    <g v-else-if="name === 'document'">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8l-6-6z" />
      <path d="M14 2v6h6" />
      <path d="M8 13h8M8 17h5" />
    </g>
    <g v-else-if="name === 'news'">
      <path d="M4 6a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v13H6a2 2 0 0 1-2-2V6z" />
      <path d="M17 9h2a1 1 0 0 1 1 1v6a2 2 0 0 1-2 2" />
      <path d="M7 8h7M7 12h7M7 16h4" />
    </g>
    <g v-else-if="name === 'check-square'">
      <rect x="3.5" y="3.5" width="17" height="17" rx="4" />
      <path d="m8.5 12.4 2.6 2.6 4.6-5.4" />
    </g>
    <!-- 无图占位：与 lib/imageFallback.ts 里那张破图占位同一套画法 -->
    <g v-else-if="name === 'image'">
      <rect x="3" y="3" width="18" height="18" rx="3" />
      <circle cx="8.5" cy="9" r="1.5" />
      <path d="m21 15-5-5L5 21" />
    </g>
    <g v-else-if="name === 'download'">
      <path d="M12 3v12" />
      <path d="m7 10 5 5 5-5" />
      <path d="M4 20h16" />
    </g>
  </svg>
</template>

<style scoped>
.glyph {
  /* 与文字同行时（如"＋ 添加一行"）按文字中线对齐，别骑在基线上；
     当 flex 子项时 vertical-align 不参与，由父级的 align-items 居中 */
  display: inline-block;
  vertical-align: -0.14em;
  flex-shrink: 0;
}
</style>
