<script setup lang="ts">
// 存档列表：备份、打开目录、删除（数据包是同一分类下的子页，见 DatapackPane）
import { t } from "../../../lib/i18n";
import { showToast } from "../../../lib/toast";
import { backupSave, deleteSave } from "../../../lib/api";
import ContentHead from "./ContentHead.vue";
import ResourceRow from "./ResourceRow.vue";
import SaveTabs from "./SaveTabs.vue";
import type { useResourceData } from "../composables/useResourceData";
import type { useResourceOps } from "../composables/useResourceOps";
import type { SaveItemDto } from "../../../lib/bindings";

const props = defineProps<{
  data: ReturnType<typeof useResourceData>;
  ops: ReturnType<typeof useResourceOps>;
}>();

const { saves, instanceUuid, loading } = props.data;
const { busy, act, askDelete, openFolder } = props.ops;

function remove(item: SaveItemDto) {
  askDelete(item.levelName || item.dir, () => deleteSave(instanceUuid.value, item.dir));
}

/** 备份存档：后端返回备份出来的文件名 */
function backup(item: SaveItemDto) {
  act(async () => {
    const name = await backupSave(instanceUuid.value, item.dir);
    showToast(t("resource.backupOk", { name }));
  });
}

/** 上次游玩时间（Unix 毫秒 → 本地时间，未知显示 --） */
function formatTime(ms: number): string {
  if (!ms) return "--";
  return new Date(ms).toLocaleString();
}
</script>

<template>
  <ContentHead :data="data">
    <SaveTabs :data="data" />
  </ContentHead>

  <div v-if="loading" class="empty-tip">{{ t("resource.loading") }}</div>
  <div v-else class="item-list">
    <ResourceRow
      v-for="item in saves"
      :key="item.dir"
      :icon="item.icon"
      letter="S"
      :name="item.levelName || item.dir"
    >
      <template #badges>
        <span v-if="item.broken" class="badge badge-red">{{ t("resource.broken") }}</span>
        <span v-if="item.hardCore" class="badge badge-red">Hardcore</span>
      </template>
      <template #sub>
        {{ item.dir }} · {{ t("resource.lastPlayed", { time: formatTime(item.lastPlayed) }) }}
      </template>
      <template #actions>
        <button class="mini-btn" :disabled="busy" @click="backup(item)">
          {{ t("resource.backup") }}
        </button>
        <button class="mini-btn" :disabled="busy" @click="openFolder('saves', item.dir)">
          {{ t("resource.openFolder") }}
        </button>
        <button class="mini-btn danger" :disabled="busy" @click="remove(item)">
          {{ t("resource.delete") }}
        </button>
      </template>
    </ResourceRow>
    <div v-if="!saves.length" class="empty-tip">{{ t("resource.empty") }}</div>
  </div>
</template>
