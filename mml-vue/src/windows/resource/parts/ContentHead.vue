<script setup lang="ts">
// 内容区顶栏：左边是标题（存档分类放子页切换），右边是该分类的操作按钮 + 刷新
//
// 刷新按钮在这里统一渲染（每个分类都要有，且逻辑完全一样），
// 各分类自己的按钮（清空截图 / 添加服务器）走 actions 插槽塞在它前面。
import { t } from "../../../lib/i18n";
import type { useResourceData } from "../composables/useResourceData";

const props = defineProps<{ data: ReturnType<typeof useResourceData> }>();
const { loading, reloadCurrent } = props.data;
</script>

<template>
  <div class="content-head">
    <div><slot /></div>
    <div class="head-actions">
      <slot name="actions" />
      <button class="mini-btn" :disabled="loading" @click="reloadCurrent">
        {{ t("resource.refresh") }}
      </button>
    </div>
  </div>
</template>
