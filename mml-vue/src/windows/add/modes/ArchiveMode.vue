<script setup lang="ts">
// 添加实例 · 模式二：导入压缩包（路径框 + 文件树 + 压缩包类型）
import { nextTick, ref, watch } from "vue";
import { t } from "../../../lib/i18n";
import BaseButton from "../../../components/ui/BaseButton.vue";
import FileTreePanel from "../parts/FileTreePanel.vue";
import LoaderQueryProgress from "../../../components/LoaderQueryProgress.vue";
import type { FileNode } from "../../../lib/fileTree";

const props = defineProps<{
  path: string;
  tree: FileNode[];
  checked: Set<string>;
  expanded: Set<string>;
  packTypes: string[];
  packType: string;
  /** 正在读压缩包条目 / 识别整合包类型 */
  scanning: boolean;
  /** 校验失败：压缩包必填（高亮 + 自动聚焦） */
  invalid?: boolean;
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
  <label class="field-label">{{ t("add.archive") }} <span class="req">*</span></label>
  <div class="path-row">
    <input
      ref="pathInput"
      :value="path"
      class="field-input"
      :class="{ 'is-invalid': invalid }"
      :placeholder="t('add.archivePlaceholder')"
      spellcheck="false"
      autocomplete="off"
      @input="emit('update:path', ($event.target as HTMLInputElement).value)"
    />
    <BaseButton class="pick-btn" size="sm" variant="accent" @click="emit('pick')">{{ t("add.browse") }}</BaseButton>
  </div>

  <!-- 读取条目 / 识别类型中：不确定滚动进度条（与加载器查询、目录扫描同一套观感） -->
  <LoaderQueryProgress
    v-if="scanning"
    kind="versions"
    :visible="true"
    :step="0"
    :total="0"
    :label="t('add.archiveReading')"
  />

  <template v-if="tree.length">
    <FileTreePanel
      :tree="tree"
      :checked="checked"
      :expanded="expanded"
      @toggle-file="emit('toggle-file', $event)"
      @toggle-dir="(n: FileNode, on: boolean) => emit('toggle-dir', n, on)"
      @toggle-expand="emit('toggle-expand', $event)"
      @set-all="emit('set-all', $event)"
    />

    <label class="field-label">{{ t("add.packType") }}</label>
    <select
      :value="packType"
      class="field-select"
      @change="emit('update:packType', ($event.target as HTMLSelectElement).value)"
    >
      <option v-for="p in packTypes" :key="p" :value="p">{{ t(`add.pack.${p}`) }}</option>
    </select>
  </template>
  <!-- 还没选包时的空状态：说明这一步之后会发生什么 -->
  <p v-else class="field-hint">{{ t("add.archiveHint") }}</p>
</template>

<style scoped>
/* 卡片里第一条标签顶格，不留多余的 12px */
.field-label:first-child {
  margin-top: 0;
}
</style>
