<script setup lang="ts">
// 模组视图之二：表格（多列 + 点表头排序）
//
// 列按用户要求定：名称（带状态徽标）/ 版本 / 作者 / 文件名 / 简介。
// 排序只作用于**传进来的这一批**（分组模式下就是某个分组内部），在视图里本地算，不动后端。
import { computed, ref } from "vue";
import { t } from "../../../../lib/i18n";
import type { ModItemDto } from "../../../../lib/bindings";
import { modRowKey, type ModViewEmits, type ModViewProps } from "../../types";

const props = defineProps<ModViewProps>();
const emit = defineEmits<ModViewEmits>();

/** 可排序列：字段 + 文案键（简介也参与排序：按字典序，空值排前） */
type SortKey = "name" | "version" | "author" | "file" | "description";

const COLUMNS: Array<{ key: SortKey; labelKey: string }> = [
  { key: "name", labelKey: "resource.modName" },
  { key: "version", labelKey: "resource.modVersion" },
  { key: "author", labelKey: "resource.modAuthor" },
  { key: "file", labelKey: "resource.modFile" },
  { key: "description", labelKey: "resource.modDesc" },
];

const sortKey = ref<SortKey>("name");
const sortAsc = ref(true);

/** 点表头：同一列切换升降序，换列则从升序开始 */
function sortBy(key: SortKey) {
  if (sortKey.value === key) {
    sortAsc.value = !sortAsc.value;
    return;
  }
  sortKey.value = key;
  sortAsc.value = true;
}

/** 排序后的行（显示名缺失时用文件名，与列表视图口径一致） */
const rows = computed(() => {
  const key = sortKey.value;
  const dir = sortAsc.value ? 1 : -1;
  const value = (item: ModItemDto): string => {
    if (key === "name") return (item.name || item.file).toLowerCase();
    return (item[key] || "").toLowerCase();
  };
  return [...props.items].sort((a, b) => value(a).localeCompare(value(b)) * dir);
});
</script>

<template>
  <div class="mod-table">
    <div class="mod-tr mod-th">
      <button
        v-for="col in COLUMNS"
        :key="col.key"
        class="mod-th-cell"
        @click="sortBy(col.key)"
      >
        {{ t(col.labelKey) }}
        <span v-if="sortKey === col.key" class="mod-sort">{{ sortAsc ? "▲" : "▼" }}</span>
      </button>
      <span class="mod-th-cell mod-th-actions">{{ t("resource.modActions") }}</span>
    </div>

    <div
      v-for="item in rows"
      :key="modRowKey(item)"
      class="mod-tr"
      :class="{ 'mod-dragging': !!item.sha1 && draggingKey === item.sha1 }"
      @pointerdown="emit('drag-start', { event: $event, item })"
    >
      <span class="mod-cell mod-name-cell">
        <span class="mod-name" :title="item.name || item.file">{{ item.name || item.file }}</span>
        <span v-if="item.disable" class="badge badge-dim">{{ t("resource.modDisabled") }}</span>
        <span v-if="item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
        <span v-if="item.core" class="badge">{{ t("resource.modCore") }}</span>
        <span v-if="item.jarInJar.length" class="badge">{{ t("resource.modBuiltin") }}</span>
      </span>
      <span class="mod-cell" :title="item.version">{{ item.version }}</span>
      <span class="mod-cell" :title="item.author">{{ item.author }}</span>
      <span class="mod-cell" :title="item.file">{{ item.file }}</span>
      <span class="mod-cell mod-desc" :title="item.description">{{ item.description }}</span>
      <span class="mod-cell mod-actions" @pointerdown.stop>
        <button class="mini-btn" :disabled="busy" @click.stop="emit('toggle', item)">
          {{ item.disable ? t("resource.enable") : t("resource.disable") }}
        </button>
        <button class="mini-btn" :disabled="busy" @click.stop="emit('open-folder', item)">
          {{ t("resource.openFolder") }}
        </button>
        <button class="mini-btn danger" :disabled="busy" @click.stop="emit('remove', item)">
          {{ t("resource.delete") }}
        </button>
      </span>
    </div>

    <div v-if="!rows.length" class="empty-tip">{{ t("resource.empty") }}</div>
  </div>
</template>
