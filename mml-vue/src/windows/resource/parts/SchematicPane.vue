<script setup lang="ts">
// 结构文件列表：打开目录、删除
import { t } from "../../../lib/i18n";
import { deleteSchematic } from "../../../lib/api";
import ContentHead from "./ContentHead.vue";
import ListSkeleton from "./ListSkeleton.vue";
import ResourceRow from "./ResourceRow.vue";
import type { useResourceData } from "../composables/useResourceData";
import type { useResourceOps } from "../composables/useResourceOps";
import type { SchematicItemDto } from "../../../lib/bindings";

const props = defineProps<{
  data: ReturnType<typeof useResourceData>;
  ops: ReturnType<typeof useResourceOps>;
}>();

const { schematics, instanceUuid, loading } = props.data;
const { busy, askDelete, openFolder } = props.ops;

function remove(item: SchematicItemDto) {
  askDelete(item.name || item.file, () => deleteSchematic(instanceUuid.value, item.file));
}

/** 副标题：作者 · 尺寸 · 方块数 */
function sub(item: SchematicItemDto): string {
  return [
    item.author,
    t("resource.dims", { w: item.width, h: item.height, l: item.length }),
    t("resource.blockCount", { count: item.blockCount }),
  ]
    .filter(Boolean)
    .join(" · ");
}
</script>

<template>
  <ContentHead :data="data">
    <h3 class="head-title">{{ t("resource.schematics") }}</h3>
  </ContentHead>

  <div v-if="loading" class="item-list">
    <ListSkeleton />
  </div>
  <div v-else class="item-list">
    <ResourceRow
      v-for="item in schematics"
      :key="item.file"
      :name="item.name || item.file"
    >
      <template #badges>
        <span class="badge badge-dim">{{ item.typeName }}</span>
        <span v-if="item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
      </template>
      <template #sub>{{ sub(item) }}</template>
      <template #actions>
        <button class="mini-btn" :disabled="busy" @click="openFolder('schematics', item.file)">
          {{ t("resource.openFolder") }}
        </button>
        <button class="mini-btn danger" :disabled="busy" @click="remove(item)">
          {{ t("resource.delete") }}
        </button>
      </template>
    </ResourceRow>
    <div v-if="!schematics.length" class="empty-tip">{{ t("resource.empty") }}</div>
  </div>
</template>
