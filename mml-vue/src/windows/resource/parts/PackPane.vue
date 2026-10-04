<script setup lang="ts">
// 材质包列表：打开目录、删除
import { t } from "../../../lib/i18n";
import { deleteResourcepack } from "../../../lib/api";
import ContentHead from "./ContentHead.vue";
import ResourceRow from "./ResourceRow.vue";
import type { useResourceData } from "../composables/useResourceData";
import type { useResourceOps } from "../composables/useResourceOps";
import type { PackItemDto } from "../../../lib/bindings";

const props = defineProps<{
  data: ReturnType<typeof useResourceData>;
  ops: ReturnType<typeof useResourceOps>;
}>();

const { packs, instanceUuid, loading } = props.data;
const { busy, askDelete, openFolder } = props.ops;

function remove(item: PackItemDto) {
  askDelete(item.file, () => deleteResourcepack(instanceUuid.value, item.file));
}

/** 副标题：描述 · 包格式（描述可能为空） */
function sub(item: PackItemDto): string {
  const format = t("resource.packFormat", { format: item.packFormat });
  return [item.description, format].filter(Boolean).join(" · ");
}
</script>

<template>
  <ContentHead :data="data">
    <h3 class="head-title">{{ t("resource.resourcepacks") }}</h3>
  </ContentHead>

  <div v-if="loading" class="empty-tip">{{ t("resource.loading") }}</div>
  <div v-else class="item-list">
    <ResourceRow
      v-for="item in packs"
      :key="item.file"
      :icon="item.icon"
      letter="P"
      :name="item.file"
    >
      <template #badges>
        <span v-if="item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
      </template>
      <template #sub>{{ sub(item) }}</template>
      <template #actions>
        <button class="mini-btn" :disabled="busy" @click="openFolder('resourcepacks', item.file)">
          {{ t("resource.openFolder") }}
        </button>
        <button class="mini-btn danger" :disabled="busy" @click="remove(item)">
          {{ t("resource.delete") }}
        </button>
      </template>
    </ResourceRow>
    <div v-if="!packs.length" class="empty-tip">{{ t("resource.empty") }}</div>
  </div>
</template>
