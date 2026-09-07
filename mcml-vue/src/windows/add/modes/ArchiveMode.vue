<script setup lang="ts">
// 添加实例 · 模式二：导入压缩包（路径框 + 文件树 + 压缩包类型）
import { t } from "../../../lib/i18n";
import BaseButton from "../../../components/ui/BaseButton.vue";
import FileTree from "../../../components/FileTree.vue";
import type { FileNode } from "../../../lib/fileTree";

defineProps<{
  path: string;
  tree: FileNode[];
  checked: Set<string>;
  expanded: Set<string>;
  packTypes: string[];
  packType: string;
}>();

const emit = defineEmits<{
  (e: "update:path", v: string): void;
  (e: "pick"): void;
  (e: "toggle-file", key: string): void;
  (e: "toggle-dir", node: FileNode, on: boolean): void;
  (e: "toggle-expand", key: string): void;
  (e: "set-all", on: boolean): void;
  (e: "update:packType", v: string): void;
}>();
</script>

<template>
  <label class="field-label">{{ t("add.archive") }} <span class="req">*</span></label>
  <div class="path-row">
    <input
      :value="path"
      class="field-input"
      :placeholder="t('add.archivePlaceholder')"
      spellcheck="false"
      @input="emit('update:path', ($event.target as HTMLInputElement).value)"
    />
    <BaseButton size="sm" variant="accent" @click="emit('pick')">{{ t("add.browse") }}</BaseButton>
  </div>

  <template v-if="tree.length">
    <div class="files-head">
      <span class="field-label files-label">{{ t("add.files") }}</span>
      <span class="files-actions">
        <button class="files-btn" @click="emit('set-all', true)">{{ t("add.filesAll") }}</button>
        <button class="files-btn" @click="emit('set-all', false)">{{ t("add.filesNone") }}</button>
      </span>
    </div>
    <div class="file-list">
      <FileTree
        :nodes="tree"
        :checked="checked"
        :expanded="expanded"
        @toggle-file="emit('toggle-file', $event)"
        @toggle-dir="(n: FileNode, on: boolean) => emit('toggle-dir', n, on)"
        @toggle-expand="emit('toggle-expand', $event)"
      />
    </div>

    <label class="field-label">{{ t("add.packType") }}</label>
    <select
      :value="packType"
      class="field-select"
      @change="emit('update:packType', ($event.target as HTMLSelectElement).value)"
    >
      <option v-for="p in packTypes" :key="p" :value="p">{{ t(`add.pack.${p}`) }}</option>
    </select>
  </template>
</template>

<style scoped>
.req {
  color: var(--red);
}

.path-row {
  display: flex;
  gap: 8px;
  align-items: center;
}

.files-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 4px 0 6px;
}

.files-label {
  margin: 0;
  font-weight: 600;
  color: var(--text);
}

.files-actions {
  display: flex;
  gap: 8px;
}

.files-btn {
  border: none;
  background: transparent;
  color: var(--accent);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  padding: 2px 4px;
}

.files-btn:hover {
  text-decoration: underline;
}

.file-list {
  max-height: 220px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 6px;
  background: var(--bg);
  margin-bottom: 8px;
}
</style>
