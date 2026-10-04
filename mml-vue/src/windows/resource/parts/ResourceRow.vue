<script setup lang="ts">
// 资源列表的通用行：图标（图 / 首字母兜底）+ 名称 + 徽标 + 副标题 + 右侧操作
//
// 七个分类的行长得一模一样，差别只在徽标、副标题与操作按钮上，所以那三处走插槽；
// 行的样式在窗口级 resource.css 里（非 scoped，拆开的子组件共用，见那里的头部说明）。
withDefaults(
  defineProps<{
    /** 图标地址（没有时退回 letter 占位） */
    icon?: string | null;
    /** 没有图标时显示的占位字（M / P / S / D…） */
    letter?: string;
    name?: string;
  }>(),
  { icon: "", letter: "?", name: "" },
);
</script>

<template>
  <div class="item-row">
    <img v-if="icon" class="item-icon" :src="icon" alt="" />
    <span v-else class="item-icon item-icon-empty">{{ letter }}</span>
    <div class="item-main">
      <div class="item-name-line">
        <span class="item-name">{{ name }}</span>
        <slot name="badges" />
      </div>
      <span class="item-sub"><slot name="sub" /></span>
    </div>
    <div class="item-actions">
      <slot name="actions" />
    </div>
  </div>
</template>
