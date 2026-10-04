<script setup lang="ts">
// 数据包子页：先选存档（下拉），再列该存档里的数据包，可启用 / 禁用 / 删除
import { t } from "../../../lib/i18n";
import { deleteDatapack, toggleDatapack } from "../../../lib/api";
import ContentHead from "./ContentHead.vue";
import ResourceRow from "./ResourceRow.vue";
import SaveTabs from "./SaveTabs.vue";
import type { useResourceData } from "../composables/useResourceData";
import type { useResourceOps } from "../composables/useResourceOps";
import type { DataPackItemDto } from "../../../lib/bindings";

const props = defineProps<{
  data: ReturnType<typeof useResourceData>;
  ops: ReturnType<typeof useResourceOps>;
}>();

const { saves, datapacks, dpSave, instanceUuid, loading } = props.data;
const { busy, act, askDelete, openFolder } = props.ops;

function toggle(item: DataPackItemDto) {
  act(async () => {
    await toggleDatapack(instanceUuid.value, dpSave.value, item.name);
  });
}

function remove(item: DataPackItemDto) {
  askDelete(item.file, () => deleteDatapack(instanceUuid.value, dpSave.value, item.name));
}

/** 启用状态（三态：开着 / 关着 / 没登记进 level.dat） */
function state(item: DataPackItemDto): string {
  if (item.enable === true) return t("resource.dpOn");
  if (item.enable === false) return t("resource.dpOff");
  return t("resource.dpNone");
}

/** 显示名：去掉 NBT 登记名里的 `file/` 前缀 */
function name(item: DataPackItemDto): string {
  return item.name.startsWith("file/") ? item.name.slice(5) : item.file || item.name;
}

/** 副标题：描述 · 包格式 */
function sub(item: DataPackItemDto): string {
  const format = t("resource.packFormat", { format: item.packFormat });
  return [item.description, format].filter(Boolean).join(" · ");
}
</script>

<template>
  <ContentHead :data="data">
    <SaveTabs :data="data" />
  </ContentHead>

  <div class="item-list">
    <select v-model="dpSave" class="dp-select">
      <option value="" disabled>{{ t("resource.selectSave") }}</option>
      <option v-for="s in saves" :key="s.dir" :value="s.dir">
        {{ s.levelName || s.dir }}
      </option>
    </select>

    <template v-if="dpSave">
      <ResourceRow
        v-for="item in datapacks"
        :key="item.name"
        letter="D"
        :name="name(item)"
      >
        <template #badges>
          <span class="badge" :class="{ 'badge-red': item.enable === false }">
            {{ state(item) }}
          </span>
        </template>
        <template #sub>{{ sub(item) }}</template>
        <template #actions>
          <button class="mini-btn" :disabled="busy" @click="toggle(item)">
            {{ item.enable === false ? t("resource.enable") : t("resource.disable") }}
          </button>
          <button class="mini-btn" :disabled="busy" @click="openFolder('datapacks', null, dpSave)">
            {{ t("resource.openFolder") }}
          </button>
          <button class="mini-btn danger" :disabled="busy" @click="remove(item)">
            {{ t("resource.delete") }}
          </button>
        </template>
      </ResourceRow>
      <div v-if="!datapacks.length && !loading" class="empty-tip">{{ t("resource.empty") }}</div>
    </template>
    <div v-else class="empty-tip">{{ t("resource.selectSave") }}</div>
  </div>
</template>
