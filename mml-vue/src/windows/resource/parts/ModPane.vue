<script setup lang="ts">
// 模组分类：三种展示方式（列表 / 表格 / JiJ 树）+ 自定义分组
//
// - 顶栏（ContentHead）：标题 + 视图切换 + 「分组」开关 + 新建分组 + 刷新
// - 分组打开：每个自建分组一块，末尾跟一块「未分组」；把模组行拖到某块上即归到该组
// - 分组关闭：选中的视图直接铺平（拖拽也随之关闭 —— 没有落点可言）
//
// 数据：模组列表在 useResourceData，分组在 useModGroups（自己的 JSON），
// 拖拽在 useModDrag（三种视图共用同一套落点判定）。
import { computed, onMounted, ref } from "vue";
import { t } from "../../../lib/i18n";
import SegmentedTabs from "../../../components/ui/SegmentedTabs.vue";
import BaseButton from "../../../components/ui/BaseButton.vue";
import BaseModal from "../../../components/ui/BaseModal.vue";
import ContentHead from "./ContentHead.vue";
import ModList from "./mod/ModList.vue";
import ModTable from "./mod/ModTable.vue";
import ModTree from "./mod/ModTree.vue";
import { deleteMod, disableMod, enableMod } from "../../../lib/api";
import { useModDrag } from "../composables/useModDrag";
import { useModGroups } from "../composables/useModGroups";
import type { useResourceData } from "../composables/useResourceData";
import type { useResourceOps } from "../composables/useResourceOps";
import type { useResourceView } from "../composables/useResourceView";
import type { ModItemDto } from "../../../lib/bindings";
import type { ModView } from "../types";

const props = defineProps<{
  data: ReturnType<typeof useResourceData>;
  ops: ReturnType<typeof useResourceOps>;
  view: ReturnType<typeof useResourceView>;
}>();

const { mods, instanceUuid, loading } = props.data;
const { busy, act, askDelete, askConfirm, openFolder } = props.ops;
const { modView, setModView } = props.view;

const groups = useModGroups(props.data);

// ---------- 拖拽归组 ----------

const { draggingKey, dropGroup, onRowPointerDown, consumeSuppressClick } = useModDrag({
  onDrop: (group, keys) => void groups.setGroup(group, keys),
});

/** 行按下：坏 jar 没有 SHA1，不可归组 */
function onDragStart(payload: { event: PointerEvent; item: ModItemDto }) {
  if (!payload.item.sha1) return;
  onRowPointerDown(payload.event, payload.item.sha1);
}

// ---------- 搜索 ----------

/** 搜索词（**不持久化**：这是一次性的筛选，不是视图偏好，重开窗口该是干净的） */
const keyword = ref("");
const query = computed(() => keyword.value.trim().toLowerCase());

/** 命中判定：名字 / modid / 文件名 / 作者 / 版本 / 简介，任一包含即命中 */
function hit(item: ModItemDto, q: string): boolean {
  return [item.name, item.modId, item.file, item.author, item.version, item.description].some(
    (value) => value.toLowerCase().includes(q),
  );
}

/** 当前要显示的模组（搜索词为空时就是全部） */
const matched = computed(() => {
  const q = query.value;
  return q ? mods.value.filter((item) => hit(item, q)) : mods.value;
});

/** 搜了但一个都没命中 */
const noMatch = computed(() => query.value !== "" && matched.value.length === 0);

// ---------- 分组块 ----------

/**
 * 内置的**状态分组**键
 *
 * 它们不是用户分组：不可改名 / 删除，也不能往里拖（归属由"启用 / 禁用"操作决定，
 * 拖进去等于改状态，本轮不做）。用户自建的分组排在它们前面。
 */
const STATE_ON = "\u0001on";
const STATE_OFF = "\u0001off";
const STATE_FAIL = "\u0001fail";

/** 分组视图的块：自建分组（可拖入）+ 三个状态分组（只读，空的收起来） */
const sections = computed(() => {
  const of = groups.groupOfKey.value;
  // 只在自建分组里的模组：剩下的才按状态分桶（同一个模组不会在两处出现）
  const rest = matched.value.filter((item) => !item.sha1 || !of.has(item.sha1));

  const custom = groups.groups.value.map((group) => ({
    key: group.name,
    label: group.name,
    items: matched.value.filter((item) => !!item.sha1 && of.get(item.sha1) === group.name),
    /** 自建分组：可拖入，头上有改名 / 删除 */
    custom: true,
  }));

  const states = [
    {
      key: STATE_ON,
      label: t("resource.groupOn"),
      items: rest.filter((item) => !item.fail && !item.disable),
      custom: false,
    },
    {
      key: STATE_OFF,
      label: t("resource.groupOff"),
      items: rest.filter((item) => !item.fail && item.disable),
      custom: false,
    },
    {
      key: STATE_FAIL,
      label: t("resource.groupFail"),
      items: rest.filter((item) => item.fail),
      custom: false,
    },
  ];

  // 搜索时把没命中的自建分组收起来；状态分组本来就只显示非空的
  const list = query.value ? custom.filter((sec) => sec.items.length > 0) : custom;
  return [...list, ...states.filter((sec) => sec.items.length > 0)];
});

/** 一个分组的启用 / 禁用计数（用户要的"按启用禁用"看这一眼就够） */
function counts(items: ModItemDto[]): string {
  const off = items.filter((item) => item.disable).length;
  return t("resource.groupCounts", { n: items.length, on: items.length - off, off });
}

// ---------- 分组增删改 ----------

/** 分组名弹窗：新建（原名空）/ 重命名（带原名） */
const groupForm = ref<{ orig: string; name: string } | null>(null);
const groupBusy = ref(false);

function openGroupForm(orig = "") {
  groupForm.value = { orig, name: orig };
}

async function saveGroupForm() {
  const form = groupForm.value;
  if (!form || groupBusy.value) return;
  const name = form.name.trim();
  if (!name) return;
  groupBusy.value = true;
  try {
    const ok = form.orig ? await groups.rename(form.orig, name) : await groups.add(name);
    if (ok) groupForm.value = null;
  } finally {
    groupBusy.value = false;
  }
}

function removeGroup(name: string) {
  askConfirm(
    t("resource.groupDelete"),
    t("resource.groupDeleteConfirm", { name }),
    () => groups.remove(name),
    // 分组数据不在资源列表里：删完不用重拉列表（模组那一次要重解析 jar，很慢）
    { reload: false },
  );
}

// ---------- 模组操作（三个视图共用） ----------

function toggle(item: ModItemDto) {
  if (consumeSuppressClick()) return;
  act(async () => {
    if (item.disable) {
      await enableMod(instanceUuid.value, item.uuid);
    } else {
      await disableMod(instanceUuid.value, item.uuid);
    }
  });
}

function remove(item: ModItemDto) {
  askDelete(item.name || item.file, () => deleteMod(instanceUuid.value, item.uuid));
}

onMounted(() => void groups.load());
</script>

<template>
  <ContentHead :data="data">
    <h3 class="head-title">{{ t("resource.mods") }}</h3>
    <template #actions>
      <input
        v-model="keyword"
        class="mod-search"
        :placeholder="t('resource.searchPlaceholder')"
        spellcheck="false"
        autocomplete="off"
      />
      <SegmentedTabs
        :model-value="modView"
        :options="[
          { value: 'list', label: t('resource.viewList') },
          { value: 'table', label: t('resource.viewTable') },
          { value: 'tree', label: t('resource.viewTree') },
        ]"
        @update:model-value="setModView($event as ModView)"
      />
      <button class="mini-btn" :disabled="busy" @click="openGroupForm()">
        {{ t("resource.groupAdd") }}
      </button>
    </template>
  </ContentHead>

  <div v-if="loading" class="empty-tip">{{ t("resource.loading") }}</div>
  <div v-else-if="noMatch" class="empty-tip">{{ t("resource.searchEmpty") }}</div>

  <!-- 分组常开：自建分组（可拖入）+ 状态分组（已启用 / 已禁用 / 识别失败，只读） -->
  <div v-else class="item-list mod-groups">
    <section
      v-for="sec in sections"
      :key="sec.key"
      class="mod-group"
      :data-mod-group="sec.custom ? sec.key : undefined"
      :class="{ 'drop-target': sec.custom && dropGroup === sec.key }"
    >
      <div class="mod-group-head">
        <span class="mod-group-name">{{ sec.label }}</span>
        <span class="mod-group-count">{{ counts(sec.items) }}</span>
        <span v-if="sec.custom" class="group-acts">
          <button class="group-act" :disabled="busy" @click="openGroupForm(sec.key)">
            {{ t("resource.groupRename") }}
          </button>
          <button class="group-act danger" :disabled="busy" @click="removeGroup(sec.key)">
            {{ t("resource.groupDelete") }}
          </button>
        </span>
      </div>

      <div class="mod-group-body">
        <p v-if="!sec.items.length" class="empty-tip">
          {{ t("resource.groupEmptyHint") }}
        </p>
        <ModList
          v-else-if="modView === 'list'"
          :items="sec.items"
          :busy="busy"
          :dragging-key="draggingKey"
          @toggle="toggle"
          @remove="remove"
          @open-folder="openFolder('mods', $event.file)"
          @drag-start="onDragStart"
        />
        <ModTable
          v-else-if="modView === 'table'"
          :items="sec.items"
          :busy="busy"
          :dragging-key="draggingKey"
          @toggle="toggle"
          @remove="remove"
          @open-folder="openFolder('mods', $event.file)"
          @drag-start="onDragStart"
        />
        <ModTree
          v-else
          :items="sec.items"
          :busy="busy"
          :dragging-key="draggingKey"
          @toggle="toggle"
          @remove="remove"
          @open-folder="openFolder('mods', $event.file)"
          @drag-start="onDragStart"
        />
      </div>
    </section>
  </div>

  <!-- 新建 / 重命名分组 -->
  <BaseModal
    v-if="groupForm"
    :title="groupForm.orig ? t('resource.groupRenameTitle') : t('resource.groupAddTitle')"
    :closable="false"
    @close="groupForm = null"
  >
    <label class="field-label">{{ t("resource.groupName") }}</label>
    <input
      v-model="groupForm.name"
      class="field-input"
      :placeholder="t('resource.groupPlaceholder')"
      spellcheck="false"
      @keydown.enter="saveGroupForm"
    />
    <div class="modal-actions">
      <BaseButton :disabled="groupBusy" @click="groupForm = null">
        {{ t("resource.cancel") }}
      </BaseButton>
      <BaseButton variant="primary" :disabled="groupBusy" @click="saveGroupForm">
        {{ t("resource.save") }}
      </BaseButton>
    </div>
  </BaseModal>
</template>
