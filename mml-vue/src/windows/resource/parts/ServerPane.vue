<script setup lang="ts">
// 服务器列表（servers.dat）：添加 / 编辑 / 删除、打开目录 + **每行的 MOTD 状态**
//
// 添加/编辑的表单状态属于本分类，就放在这一层（草稿 + 保存中标记），
// 弹窗排版在 ServerFormModal。
//
// MOTD 也是这一层：进分类 / 刷新列表时**并发查全部服务器**，结果填进对应那一行。
// 并发是必须的 —— 连不上的服要等到超时，串行的话后面的行全被它堵住。
import { ref, watch } from "vue";
import { t, tErr } from "../../../lib/i18n";
import { showToast } from "../../../lib/toast";
import { addServer, deleteServer, getMotd, updateServer } from "../../../lib/api";
import { faviconOf, motdSegStyle } from "../../../lib/motd";
import ContentHead from "./ContentHead.vue";
import ListSkeleton from "./ListSkeleton.vue";
import ResourceRow from "./ResourceRow.vue";
import ServerFormModal from "./ServerFormModal.vue";
import type { useResourceData } from "../composables/useResourceData";
import type { useResourceOps } from "../composables/useResourceOps";
import type { ServerFormDraft } from "../types";
import type { MotdDto, ServerItemDto } from "../../../lib/bindings";

const props = defineProps<{
  data: ReturnType<typeof useResourceData>;
  ops: ReturnType<typeof useResourceOps>;
}>();

const { servers, instanceUuid, loading, reloadCurrent } = props.data;
const { busy, askDelete, openFolder } = props.ops;

/** 表单草稿（null = 没开弹窗） */
const form = ref<ServerFormDraft | null>(null);
const saving = ref(false);

/** 一行的身份键：名字 + 地址（servers.dat 允许重名，地址才是真的定位） */
function keyOf(item: ServerItemDto): string {
  return `${item.name}|${item.ip}`;
}

/**
 * MOTD 查询结果：`keyOf(item)` → 结果
 *
 * 值可能是 `null`（查过但失败），而"键不存在"表示**还没回来** ——
 * 两者在界面上不一样（"查询中…" vs "离线"），所以不能都塞成 null。
 */
const motds = ref(new Map<string, MotdDto | null>());

/** 正在查的键集合（用于"查询中…"） */
const pending = ref(new Set<string>());

/** 并发查全部服务器（进分类 / 刷新列表时跑一遍） */
async function queryAll() {
  const list = servers.value;
  motds.value = new Map();
  if (!list.length) {
    pending.value = new Set();
    return;
  }
  pending.value = new Set(list.map(keyOf));

  await Promise.all(
    list.map(async (item) => {
      const key = keyOf(item);
      try {
        motds.value.set(key, await getMotd(item.ip));
      } catch {
        // 失败也要落一条 null：否则这一行会永远停在"查询中…"
        motds.value.set(key, null);
      } finally {
        const next = new Set(pending.value);
        next.delete(key);
        pending.value = next;
      }
    }),
  );
}

// 列表一换（首次加载、点刷新、增删改之后重拉）就重查一遍
watch(servers, () => void queryAll(), { immediate: true });

/** 这一行的 MOTD（没查到 / 还在查时返回 null） */
function motdOf(item: ServerItemDto): MotdDto | null {
  return motds.value.get(keyOf(item)) ?? null;
}

function isLoading(item: ServerItemDto): boolean {
  return pending.value.has(keyOf(item));
}

/** 查到且服务器在线 */
function isOnline(item: ServerItemDto): boolean {
  return motdOf(item)?.state === "ok";
}

/**
 * 行图标：优先用 `servers.dat` 里存的那个（玩家自己设的），
 * 没存就用 MOTD 带回的服务器图标 —— 真实服务器列表就是这么显示的
 */
function iconOf(item: ServerItemDto): string {
  return item.icon || faviconOf(motdOf(item)) || "";
}

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
    <template #actions>
      <button class="mini-btn" :disabled="busy" @click="openForm(null)">
        {{ t("resource.serverAdd") }}
      </button>
    </template>
  </ContentHead>

  <div v-if="loading" class="item-list">
    <ListSkeleton />
  </div>
  <div v-else class="item-list">
    <ResourceRow
      v-for="item in servers"
      :key="keyOf(item)"
      :icon="iconOf(item)"
      :name="item.name"
    >
      <template #badges>
        <span v-if="item.acceptTextures" class="badge badge-dim">
          {{ t("resource.acceptTextures") }}
        </span>
        <!-- 在线人数：查到了才显示（查不到时那一行下面会写"离线"） -->
        <span v-if="isOnline(item)" class="badge">
          {{
            t("server.players", {
              now: motdOf(item)?.playersOnline ?? 0,
              max: motdOf(item)?.playersMax ?? 0,
            })
          }}
        </span>
      </template>

      <!--
        副标题：MOTD（彩色段）优先，后面跟版本 / 延迟；查不到就退回地址 + 一句状态。
        地址**始终留在最后** —— 它是这一行的定位信息，不能因为查到了就不显示
      -->
      <template #sub>
        <template v-if="isOnline(item)">
          <span
            v-for="(seg, i) in motdOf(item)?.segments ?? []"
            :key="i"
            :style="motdSegStyle(seg)"
          >{{ seg.text }}</span>
          <span class="sep">·</span>
          <span>{{ motdOf(item)?.version || t("server.unknown") }}</span>
          <span class="sep">·</span>
          <span>{{ t("server.ping", { ms: motdOf(item)?.ping ?? 0 }) }}</span>
          <span class="sep">·</span>
        </template>
        <span>{{ item.ip }}</span>
        <template v-if="isLoading(item)">
          <span class="sep">·</span><span>{{ t("server.refreshing") }}</span>
        </template>
        <template v-else-if="!isOnline(item)">
          <span class="sep">·</span>
          <span>{{ motdOf(item)?.message || t("server.offline") }}</span>
        </template>
      </template>

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

<!-- 副标题里的分隔号：与模组行的 `·` 同一套观感（行内间距靠 CSS，不靠空格） -->
<style scoped>
.sep {
  margin: 0 5px;
  color: var(--text-dim);
  opacity: 0.7;
}
</style>
