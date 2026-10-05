<script setup lang="ts">
// 材质包列表：打开目录、删除
import { t } from "../../../lib/i18n";
import { deleteResourcepack } from "../../../lib/api";
import ContentHead from "./ContentHead.vue";
import FormattedText from "../../../components/ui/FormattedText.vue";
import ListSkeleton from "./ListSkeleton.vue";
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

/** 包格式那段文案（与描述分开渲染：描述要过格式码，这一句不要） */
function formatLabel(item: PackItemDto): string {
  return t("resource.packFormat", { format: item.packFormat });
}
</script>

<template>
  <ContentHead :data="data">
    <h3 class="head-title">{{ t("resource.resourcepacks") }}</h3>
  </ContentHead>

  <div v-if="loading" class="item-list">
    <ListSkeleton />
  </div>
  <div v-else class="item-list">
    <ResourceRow
      v-for="item in packs"
      :key="item.file"
      :icon="item.icon"
      :name="item.file"
    >
      <template #badges>
        <span v-if="item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
      </template>
      <!--
        描述要过格式码（整合包作者常用 `§a` 这类上色），包格式那句不要 ——
        所以两段分开渲染、中间自己接分隔符，不能先拼成一个字符串再整体格式化
      -->
      <template #sub>
        <FormattedText v-if="item.description" :text="item.description" />
        <template v-if="item.description"> · </template>{{ formatLabel(item) }}
      </template>
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
