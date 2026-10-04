<script setup lang="ts">
// 服务器列表（servers.dat）：添加 / 编辑 / 删除、打开目录
//
// 添加/编辑的表单状态属于本分类，就放在这一层（草稿 + 保存中标记），
// 弹窗排版在 ServerFormModal。
import { ref } from "vue";
import { t, tErr } from "../../../lib/i18n";
import { showToast } from "../../../lib/toast";
import { addServer, deleteServer, updateServer } from "../../../lib/api";
import ContentHead from "./ContentHead.vue";
import ResourceRow from "./ResourceRow.vue";
import ServerFormModal from "./ServerFormModal.vue";
import type { useResourceData } from "../composables/useResourceData";
import type { useResourceOps } from "../composables/useResourceOps";
import type { ServerFormDraft } from "../types";
import type { ServerItemDto } from "../../../lib/bindings";

const props = defineProps<{
  data: ReturnType<typeof useResourceData>;
  ops: ReturnType<typeof useResourceOps>;
}>();

const { servers, instanceUuid, loading, reloadCurrent } = props.data;
const { busy, askDelete, openFolder } = props.ops;

/** 表单草稿（null = 没开弹窗） */
const form = ref<ServerFormDraft | null>(null);
const saving = ref(false);

function openForm(item: ServerItemDto | null) {
  form.value = item
    ? {
        edit: true,
        name: item.name,
        ip: item.ip,
        acceptTextures: item.acceptTextures,
        origName: item.name,
        origIp: item.ip,
      }
    : { edit: false, name: "", ip: "", acceptTextures: false, origName: "", origIp: "" };
}

async function save() {
  const draft = form.value;
  if (!draft || saving.value) return;
  const name = draft.name.trim();
  const ip = draft.ip.trim();
  if (!name || !ip) {
    showToast(t("err.nameIp"));
    return;
  }
  saving.value = true;
  try {
    if (draft.edit) {
      await updateServer(
        instanceUuid.value,
        draft.origName,
        draft.origIp,
        name,
        ip,
        draft.acceptTextures,
      );
    } else {
      await addServer(instanceUuid.value, name, ip);
    }
    form.value = null;
    await reloadCurrent();
  } catch (e) {
    // 失败提示后输入内容留着，用户改完再存
    showToast(tErr(e));
  } finally {
    saving.value = false;
  }
}

function remove(item: ServerItemDto) {
  askDelete(item.name || item.ip, () => deleteServer(instanceUuid.value, item.name, item.ip));
}
</script>

<template>
  <ContentHead :data="data">
    <h3 class="head-title">{{ t("resource.servers") }}</h3>
    <template #actions>
      <button class="mini-btn" :disabled="busy" @click="openForm(null)">
        {{ t("resource.serverAdd") }}
      </button>
    </template>
  </ContentHead>

  <div v-if="loading" class="empty-tip">{{ t("resource.loading") }}</div>
  <div v-else class="item-list">
    <ResourceRow
      v-for="item in servers"
      :key="`${item.name}|${item.ip}`"
      :icon="item.icon"
      letter="S"
      :name="item.name"
    >
      <template #badges>
        <span v-if="item.acceptTextures" class="badge badge-dim">
          {{ t("resource.acceptTextures") }}
        </span>
      </template>
      <template #sub>{{ item.ip }}</template>
      <template #actions>
        <button class="mini-btn" :disabled="busy" @click="openForm(item)">
          {{ t("resource.serverEdit") }}
        </button>
        <button class="mini-btn" :disabled="busy" @click="openFolder('servers', null)">
          {{ t("resource.openFolder") }}
        </button>
        <button class="mini-btn danger" :disabled="busy" @click="remove(item)">
          {{ t("resource.delete") }}
        </button>
      </template>
    </ResourceRow>
    <div v-if="!servers.length" class="empty-tip">{{ t("resource.empty") }}</div>
  </div>

  <ServerFormModal
    v-if="form"
    :form="form"
    :busy="saving"
    @submit="save"
    @close="form = null"
  />
</template>
