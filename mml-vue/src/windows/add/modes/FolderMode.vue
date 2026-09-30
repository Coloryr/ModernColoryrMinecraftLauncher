<script setup lang="ts">
// 添加实例 · 模式三：添加文件夹（路径框 + 内容树，懒加载）
import { nextTick, ref, watch } from "vue";
import { t } from "../../../lib/i18n";
import BaseButton from "../../../components/ui/BaseButton.vue";
import FileTreePanel from "../parts/FileTreePanel.vue";
import type { FileNode } from "../../../lib/fileTree";

const props = defineProps<{
  path: string;
  tree: FileNode[];
  checked: Set<string>;
  expanded: Set<string>;
  /** 校验失败：文件夹必填（高亮 + 自动聚焦） */
  invalid?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:path", v: string): void;
  (e: "pick"): void;
  (e: "toggle-file", key: string): void;
  (e: "toggle-dir", node: FileNode, on: boolean): void;
  (e: "toggle-expand", key: string): void;
  (e: "lazy-load", node: FileNode): void;
  (e: "set-all", on: boolean): void;
}>();

const pathInput = ref<HTMLInputElement | null>(null);

// 必填校验失败：聚焦路径框（高亮见模板的 is-invalid）
watch(
  () => props.invalid,
  async (invalid) => {
    if (!invalid) return;
    await nextTick();
    pathInput.value?.focus();
  },
);
</script>

<template>
  <label class="field-label">{{ t("add.folder") }} <span class="req">*</span></label>
  <div class="path-row">
    <input
      ref="pathInput"
      :value="path"
      class="field-input"
      :class="{ 'is-invalid': invalid }"
      :placeholder="t('add.folderPlaceholder')"
      spellcheck="false"
      autocomplete="off"
      @input="emit('update:path', ($event.target as HTMLInputElement).value)"
    />
    <BaseButton size="sm" variant="accent" @click="emit('pick')">{{ t("add.browse") }}</BaseButton>
  </div>

  <FileTreePanel
    v-if="tree.length"
    :tree="tree"
    :checked="checked"
    :expanded="expanded"
    @toggle-file="emit('toggle-file', $event)"
    @toggle-dir="(n: FileNode, on: boolean) => emit('toggle-dir', n, on)"
    @toggle-expand="emit('toggle-expand', $event)"
    @lazy-load="emit('lazy-load', $event)"
    @set-all="emit('set-all', $event)"
  />
  <!-- 还没选文件夹时的空状态 -->
  <p v-else class="field-hint">{{ t("add.folderHint") }}</p>
</template>

<style scoped>
/* 卡片里第一条标签顶格，不留多余的 12px */
.field-label:first-child {
  margin-top: 0;
}
</style>
