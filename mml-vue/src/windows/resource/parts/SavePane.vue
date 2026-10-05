<script setup lang="ts">
// 存档列表：备份、打开目录、删除（数据包是同一分类下的子页，见 DatapackPane）
import { t } from "../../../lib/i18n";
import { stripFormatting } from "../../../lib/formatting";
import { showToast } from "../../../lib/toast";
import { backupSave, deleteSave } from "../../../lib/api";
import ContentHead from "./ContentHead.vue";
import FormattedText from "../../../components/ui/FormattedText.vue";
import ListSkeleton from "./ListSkeleton.vue";
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
  // 确认框里用**去掉格式码**的名字：那里是纯文本，`§a` 只会显示成乱码
  askDelete(stripFormatting(item.levelName || item.dir), () =>
    deleteSave(instanceUuid.value, item.dir),
  );
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

  <div v-if="loading" class="item-list">
    <ListSkeleton />
  </div>
  <div v-else class="item-list">
    <ResourceRow
      v-for="item in saves"
      :key="item.dir"
      :icon="item.icon"
      :name="item.levelName || item.dir"
    >
      <!-- 存档名可以带 `§` 格式码（地图作者常用来上色），过一遍显示期解析 -->
      <template #name>
        <FormattedText :text="item.levelName || item.dir" />
      </template>
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
