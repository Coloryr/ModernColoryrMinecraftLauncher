<script setup lang="ts">
// 模组分类：三种展示方式（列表 / 表格 / JiJ 树）+ 自定义分组
//
// - 顶栏（ContentHead）：标题 + 视图切换 + 「分组」开关 + 新建分组 + 刷新
// - 分组打开：每个自建分组一块，末尾跟一块「未分组」；把模组行拖到某块上即归到该组
// - 分组关闭：选中的视图直接铺平（拖拽也随之关闭 —— 没有落点可言）
//
// 数据：模组列表在 useResourceData，分组在 useModGroups（自己的 JSON），
// 拖拽在 useModDrag（三种视图共用同一套落点判定）。
import { computed, onMounted, ref, watch } from "vue";
import { t, tErr } from "../../../lib/i18n";
import { showToast } from "../../../lib/toast";
import SegmentedTabs from "../../../components/ui/SegmentedTabs.vue";
import BaseButton from "../../../components/ui/BaseButton.vue";
import BaseModal from "../../../components/ui/BaseModal.vue";
import GlyphIcon from "../../../components/ui/GlyphIcon.vue";
import ContentHead from "./ContentHead.vue";
import ListSkeleton from "./ListSkeleton.vue";
import ModList from "./mod/ModList.vue";
import ModTable from "./mod/ModTable.vue";
import { deleteMod, disableMod, enableMod, setModNote } from "../../../lib/api";
import { useModDrag, useReorderDrag, type ModDropTarget, type ModStateGroup } from "../composables/useModDrag";
import {
  useModGroups,
  STATE_GROUP_ID_OFF,
  STATE_GROUP_ID_ON,
  STATE_GROUP_ID_FAIL,
} from "../composables/useModGroups";
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

const { mods, instanceUuid, loading, modProgress } = props.data;
const { busy, act, askDelete, askConfirm, openFolder } = props.ops;
const { modView, setModView } = props.view;

const groups = useModGroups(props.data);

// ---------- 拖拽归组 / 改状态 ----------

const { draggingKey, dropTarget, onRowPointerDown, consumeSuppressClick } = useModDrag({
  onDrop: (target, keys) => void applyDrop(target, keys),
});

/**
 * 投放：自建分组 = 归组；状态分组 = 改启用状态
 *
 * 状态分组只装"不在任何自建分组里"的模组（见 [`sections`]）。所以拖过去光改状态是看不见的
 * —— 模组还留在原来那个自建分组里，那一块不会出现在状态分组下。要**先摘出分组**，
 * 投放才算真的生效。
 *
 * 反向的"状态分组 → 自建分组"由 [`afterGroupDrop`] 收尾（把模组改回启用），见那里的说明。
 *
 * 「识别失败」不是"可拖成的状态"（坏 jar 拖不动，状态也没法靠拖拽造出来），拖过去当没发生。
 */
async function applyDrop(target: ModDropTarget, keys: string[]) {
  if (target.kind === "group") {
    await groups.setGroup(target.group, keys);
    await afterGroupDrop(keys);
    return;
  }
  if (target.kind === "ungrouped") {
    await groups.setGroup(null, keys);
    return;
  }
  if (target.state === "fail") return;

  const wantDisable = target.state === "off";
  // 拖拽只带 SHA1（内容哈希），要调启用 / 禁用得换回那一条的 uuid
  const picked = pickByState(keys, wantDisable);

  // 先摘出分组：它不属于列表数据，单独提交（reload: false 那条路）
  await groups.setGroup(null, keys);
  await setState(picked, wantDisable);
}

/**
 * 拖进自建分组之后收尾：**把模组改成启用**
 *
 * 用户定的口径是"拖动模组修改分组 = 禁用和启用"，所以从「已禁用」拖到某个分组就是启用它
 * （与拖到「已启用」等价）。已经启用的不用碰 —— 启用 / 禁用会改文件名、uuid 跟着变，
 * 能不动就不动（也让"启动器里拖一下、外面只多了个分组"这种常见操作不产生额外写盘）。
 */
async function afterGroupDrop(keys: string[]) {
  await setState(pickByState(keys, false), false);
}

/**
 * 把若干模组改成目标启用状态
 *
 * `wantDisable = true` 禁用、`false` 启用；`picked` 里只放"当前状态与目标不同"的那些。
 */
async function setState(picked: ModItemDto[], wantDisable: boolean) {
  if (!picked.length) return;
  await act(async () => {
    for (const item of picked) {
      if (wantDisable) await disableMod(instanceUuid.value, item.uuid);
      else await enableMod(instanceUuid.value, item.uuid);
    }
  });
}

/** 从拖拽带过来的 SHA1 里挑出"当前状态与目标不同"的那些条目 */
function pickByState(keys: string[], wantDisable: boolean): ModItemDto[] {
  return keys
    .map((key) => bySha1.value.get(key))
    .filter((item): item is ModItemDto => !!item)
    .filter((item) => item.disable !== wantDisable);
}

/** 行按下：坏 jar 没有 SHA1，不可归组 / 改状态 */
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
 * 状态分组 → 固定 uuid / 文案键
 *
 * 状态分组与自建分组**共用一套 uuid 键**（见 useModGroups 的 `STATE_GROUP_ID_*`）：
 * 渲染身份、排序、折叠状态、投放判定全用它，不必再维护"两套键互相映射"。
 *
 * 它们不是用户分组：不可改名 / 删除。归属由"启用 / 禁用"决定：
 * - 拖**到**「已启用 / 已禁用」= 改状态 + 摘出自建分组（见 [`applyDrop`]）；
 * - 从状态分组里拖**到自建分组** = 只归组 + 启用（见 [`afterGroupDrop`]）。
 */
const STATE_KEYS: Record<ModStateGroup, { id: string; labelKey: string }> = {
  on: { id: STATE_GROUP_ID_ON, labelKey: "resource.groupOn" },
  off: { id: STATE_GROUP_ID_OFF, labelKey: "resource.groupOff" },
  fail: { id: STATE_GROUP_ID_FAIL, labelKey: "resource.groupFail" },
};

/** 分组视图里的一块：自建分组 或 状态分组 */
interface ModSection {
  /** 分组 uuid（自建与状态分组同一套键空间） */
  id: string;
  label: string;
  items: ModItemDto[];
  /** 自建分组：可改名 / 删除，落点 = 归组 */
  custom: boolean;
  /** 状态分组的类型；自建分组没有 */
  state?: ModStateGroup;
}

/**
 * 分组视图的块：**按用户拖出来的顺序**（顺序表见 useModGroups 的 `order`）
 *
 * - 自建分组：空的也留着（是个可以往里拖的落点）
 * - 「已启用 / 已禁用」：**空也显示**（用户点名）—— 它们是"改状态"的落点，空着更是个提示
 * - 「识别失败」：空就收起来（没有坏包时它只是个噪音；它也不接受投放）
 *
 * 顺序表里可能有已删分组的旧 uuid，所以以实到的为准（找不到的跳过）。
 */
const sections = computed<ModSection[]>(() => {
  const of = groups.groupOfKey.value;
  // 只在自建分组里的模组：剩下的才按状态分桶（同一个模组不会在两处出现）
  const rest = matched.value.filter((item) => !item.sha1 || !of.has(item.sha1));

  const byId = new Map<string, ModSection>();
  for (const group of groups.groups.value) {
    byId.set(group.uuid, {
      id: group.uuid,
      label: group.name,
      items: matched.value.filter((item) => !!item.sha1 && of.get(item.sha1) === group.uuid),
      custom: true,
    });
  }
  for (const state of ["on", "off", "fail"] as ModStateGroup[]) {
    const { id, labelKey } = STATE_KEYS[state];
    byId.set(id, {
      id,
      label: t(labelKey),
      items: rest.filter((item) => stateHit(item, state)),
      custom: false,
      state,
    });
  }

  const list: ModSection[] = [];
  for (const key of groups.order.value) {
    const sec = byId.get(key);
    if (!sec || !visible(sec)) continue;
    list.push(sec);
  }
  // 兜底：分组是刚建的、顺序表还没回来时，别把它漏掉（后端也会在下次下发时补上）
  for (const [key, sec] of byId) {
    if (!groups.order.value.includes(key) && visible(sec)) list.push(sec);
  }
  return list;
});

/** 一个模组是否属于某个状态分组 */
function stateHit(item: ModItemDto, state: ModStateGroup): boolean {
  if (state === "fail") return item.fail;
  if (item.fail) return false;
  return state === "off" ? item.disable : !item.disable;
}

/**
 * 这一块现在要不要显示
 *
 * 「识别失败」空着就收起来；自建分组与「已启用 / 已禁用」始终显示。
 * **搜索时例外**：搜索是对"看得见的东西"做过滤，此时空分组（含状态分组）一律收起 ——
 * 否则一搜索满屏都是空的「已启用 / 已禁用」，命中的那几条反而要找。
 */
function visible(sec: ModSection): boolean {
  if (query.value) return sec.items.length > 0;
  if (sec.custom) return true;
  return sec.state !== "fail" || sec.items.length > 0;
}

/**
 * 这一块当前是不是拖拽落点（拖着东西悬在它上面）
 *
 * 「识别失败」不接受投放 —— 不给高亮，松手也不做事，免得看着像"能放"。
 */
function isDropTarget(sec: ModSection): boolean {
  const target = dropTarget.value;
  if (!target) return false;
  if (target.kind === "state") {
    return !sec.custom && sec.state === target.state && target.state !== "fail";
  }
  return sec.custom && target.kind === "group" && target.group === sec.id;
}

/** SHA1 → 列表里那一条（拖拽只带内容哈希，改状态要用 uuid 定位） */
const bySha1 = computed(() => {
  const map = new Map<string, ModItemDto>();
  for (const item of mods.value) {
    if (item.sha1) map.set(item.sha1, item);
  }
  return map;
});

/**
 * 分组头上的计数
 *
 * 自建分组两种状态都有，所以报"启用 / 禁用"；**状态分组只报项数** ——
 * 在「已启用」里再写一行"禁用 0"、在「已禁用」里写"启用 0"都是纯噪音（用户点名）。
 */
function counts(sec: ModSection): string {
  if (sec.items.length === 0) return "";
  if (!sec.custom) return t("resource.groupCountItems", { n: sec.items.length });
  const off = sec.items.filter((item) => item.disable).length;
  return t("resource.groupCounts", { n: sec.items.length, on: sec.items.length - off, off });
}

// ---------- 分组折叠 ----------

/**
 * 分组块的收起 / 展开
 *
 * 状态存在**实例的 `guisetting.json`**（`Mod.GroupCollapsed`，见 useModGroups）：
 * 用户明确要求记住（"已启用默认不展开"），而且换实例各记各的。
 * 键就是分组 uuid —— 与顺序表同一套，不必再转换。
 */
function isOpen(id: string): boolean {
  return !groups.isCollapsed(id);
}

function toggleCollapse(id: string) {
  groups.toggleCollapsed(id);
}

// 拖到收起的块上自动展开：收起后整块只剩一条分组头，落点太小、不好瞄
watch(dropTarget, (target) => {
  const id =
    target?.kind === "group"
      ? target.group
      : target?.kind === "state"
        ? STATE_KEYS[target.state].id
        : "";
  if (id && !isOpen(id)) toggleCollapse(id);
});

// ---------- 分组增删改 ----------

// ---------- 分组头拖拽排序 ----------

/**
 * 块键 → 那块的 `<section>` 元素（量位置算插入点用）
 *
 * 用 `Map` 而不是 `querySelectorAll`：状态分组的渲染键带 `\u0001`，写进选择器要转义，
 * 而且块的渲染顺序就是 `sections` 的顺序，按 key 直接结对更稳。
 */
/**
 * 块的元素表（量位置算插入点用）
 *
 * 用 `Map` 而不是 `querySelectorAll`：块的渲染顺序就是 `sections` 的顺序，
 * 按 uuid 直接结对更稳（uuid 也不适合写进选择器）。
 */
const sectionEls = new Map<string, HTMLElement>();

function bindSection(id: string, el: Element | null) {
  if (el) sectionEls.set(id, el as HTMLElement);
  else sectionEls.delete(id);
}

/** 当前渲染的块 id（拖拽排序的坐标轴；与 DOM 里的顺序一致） */
const sectionIds = computed(() => sections.value.map((sec) => sec.id));

const {
  dragKey: draggingSection,
  insertAt: insertSectionAt,
  onPointerDown: onSectionPointerDown,
  consumeSuppressClick: consumeSectionClick,
} = useReorderDrag({
  keys: () => sectionIds.value,
  rectOf: (key) => sectionEls.get(key)?.getBoundingClientRect() ?? null,
  onDrop: (from, insertAt) => void moveSection(from, insertAt),
});

/**
 * 把某一块拖到第 `insertAt` 块之前
 *
 * 与左侧分类栏同一算法：摘掉被拖项后，目标下标在被拖项之后要减一。
 * 落盘的就是**分组 uuid**（状态分组与自建分组同一套键空间，不用转换）。
 */
async function moveSection(from: string, insertAt: number) {
  const ids = sectionIds.value;
  const index = ids.indexOf(from);
  if (index < 0) return;
  const next = [...ids];
  next.splice(index, 1);
  next.splice(Math.max(0, Math.min(next.length, insertAt > index ? insertAt - 1 : insertAt)), 0, from);

  // 只把"看得见的块"排了序，隐藏的块（空的识别失败等）不在里面 ——
  // 它们在顺序表里的位置由后端保留原样，这里按原顺序补回去
  const visible = new Set(ids);
  const hidden = groups.order.value.filter((id) => !visible.has(id));
  await groups.setOrder([...next, ...hidden]);
}

/** 插入线画不画在第 index 块之前 */
function showSectionLine(index: number): boolean {
  return draggingSection.value !== null && insertSectionAt.value === index;
}

/**
 * 点分组头（箭头 / 名字那一带）→ 折叠或展开
 *
 * 名字那一带同时是**拖拽把手**，所以要先问一句"刚才那一下是不是拖完松手的"：
 * 拖过就把这次 click 吞掉（否则排完序会顺手把分组折叠了）。
 * `consumeSuppressClick` 只在真的拖过时才返回 true（见 useDragGesture 的 `dragged`）。
 */
function onSectionToggle(id: string) {
  if (consumeSectionClick()) return;
  toggleCollapse(id);
}

// ---------- 分组增删改 ----------

/**
 * 分组名弹窗
 *
 * `id` 为**空串**表示"新建"；否则是"重命名某个分组"（带 uuid）。
 * 名字只在这一个字段里，所以弹窗不再需要同时记"原名"。
 */
const groupForm = ref<{ id: string; name: string } | null>(null);
const groupBusy = ref(false);

/** 打开弹窗：新建（不传 id）/ 重命名（传分组 uuid 与当前名字） */
function openGroupForm(id = "", name = "") {
  groupForm.value = { id, name };
}

async function saveGroupForm() {
  const form = groupForm.value;
  if (!form || groupBusy.value) return;
  const name = form.name.trim();
  if (!name) return;
  groupBusy.value = true;
  try {
    const ok = form.id ? await groups.rename(form.id, name) : await groups.add(name);
    if (ok) groupForm.value = null;
  } finally {
    groupBusy.value = false;
  }
}

/** 删除分组（传 uuid；确认框里显示的是分组名） */
function removeGroup(id: string, name: string) {
  askConfirm(
    t("resource.groupDelete"),
    t("resource.groupDeleteConfirm", { name }),
    () => groups.remove(id),
    // 分组数据不在资源列表里：删完不用重拉列表（模组那一次要重解析 jar，很慢）
    { reload: false },
  );
}

// ---------- 模组备注 ----------

/**
 * 备注弹窗的草稿（`item` 是列表里那一条的引用，存完直接改它）
 *
 * 备注存在 `guisetting.json` 的 `Mod.ModName`（与 ColorMC 互通），
 * **不在模组列表里**，所以存完**不能重拉列表** —— 重拉一次要重新解析每个 jar
 * 的元数据（数秒），用户只是写一句备注而已。改本地那一份就够。
 */
const noteForm = ref<{ item: ModItemDto; text: string } | null>(null);
const noteBusy = ref(false);

function openNote(item: ModItemDto) {
  noteForm.value = { item, text: item.note };
}

/** 保存（`text` 传空串 = 清掉备注） */
async function saveNote(text: string) {
  const form = noteForm.value;
  if (!form || noteBusy.value) return;
  const value = text.trim();
  noteBusy.value = true;
  try {
    await setModNote(instanceUuid.value, form.item.file, value);
    form.item.note = value;
    noteForm.value = null;
  } catch (e) {
    showToast(tErr(e));
  } finally {
    noteBusy.value = false;
  }
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
        ]"
        @update:model-value="setModView($event as ModView)"
      />
      <button class="mini-btn" :disabled="busy" @click="openGroupForm()">
        {{ t("resource.groupAdd") }}
      </button>
    </template>
  </ContentHead>

  <!--
    加载中：与「下载整合包」窗口同一套骨架行（呼吸动画，见 `styles/skeleton.css`）

    模组这一档是真慢 —— 每个 jar 都要解析元数据，几百个包要好几秒，
    所以给骨架 + **扫描进度 `x/x`**（进度事件由后端逐文件发出，见 useResourceData）
  -->
  <div v-if="loading" class="item-list">
    <ListSkeleton :done="modProgress.done" :total="modProgress.total" />
  </div>
  <div v-else-if="noMatch" class="empty-tip">{{ t("resource.searchEmpty") }}</div>

  <!--
    表格视图：**一张总表**（分组名就是第一列），不是每块一张小表
    —— 这样"哪个模组在哪个组"一眼看得到，也不必在每块上重复 15 列表头。

    每个分组仍是拖拽落点（data-* 属性挂在 <section> 上，落点判定与列表视图共用一套），
    只是这一块内部不再画分组头与模组行，而是把整批交给 ModTable 自己渲染。
  -->
  <div v-else-if="modView === 'table'" class="item-list mod-groups mod-groups-table">
    <template v-for="(sec, idx) in sections" :key="sec.id">
      <span v-if="showSectionLine(idx)" class="mod-group-insert" />

      <section
        :ref="(el) => bindSection(sec.id, el as Element | null)"
        class="mod-group mod-group-table"
        :data-mod-group="sec.custom ? sec.id : undefined"
        :data-state-group="sec.custom ? undefined : sec.state"
        :class="{
          'drop-target': isDropTarget(sec),
          'mod-group-dragging': draggingSection === sec.id,
        }"
      >
        <ModTable
          :items="sec.items"
          :busy="busy"
          :dragging-key="draggingKey"
          :group-label="sec.label"
          :group-key="sec.id"
          :group-open="isOpen(sec.id)"
          :custom="sec.custom"
          :dragging-group="draggingSection"
          :hide-header="idx > 0"
          @toggle="toggle"
          @remove="remove"
          @note="openNote"
          @toggle-group="onSectionToggle(sec.id)"
          @open-folder="openFolder('mods', $event.file)"
          @drag-start="onDragStart"
        />
      </section>
    </template>
  </div>

  <!-- 列表视图：按块渲染（自建分组可拖入归组 + 三个状态分组） -->
  <div v-else class="item-list mod-groups">
    <!-- 按块渲染（不用 section 上的 v-for）：插入线要画在"两块之间"，得有个兄弟节点 -->
    <template v-for="(sec, idx) in sections" :key="sec.id">
      <!-- 拖分组头时的插入线：松手后这一块会落到这里 -->
      <span v-if="showSectionLine(idx)" class="mod-group-insert" />

      <section
        :ref="(el) => bindSection(sec.id, el as Element | null)"
        class="mod-group"
        :data-mod-group="sec.custom ? sec.id : undefined"
        :data-state-group="sec.custom ? undefined : sec.state"
        :class="{
          'drop-target': isDropTarget(sec),
          'mod-group-dragging': draggingSection === sec.id,
        }"
      >
        <div class="mod-group-head">
          <!--
            分组头分两块，**两块都能点开 / 收起**：
            - 箭头：只管折叠，不参与拖拽（`@pointerdown.stop` 挡住把手的按下事件）；
            - 名字那一带：既是折叠的点击区，也是拖拽把手（按住拖动即调整分组顺序）。
              拖动结束时那一下 click 由 onSectionToggle 里的抑制标记吞掉。
          -->
          <button
            class="mod-group-toggle"
            :aria-label="t('resource.groupCollapse')"
            @pointerdown.stop
            @click.stop="toggleCollapse(sec.id)"
          >
            <GlyphIcon
              class="mod-group-caret"
              :class="{ collapsed: !isOpen(sec.id) }"
              name="chevron-down"
              :size="13"
              :weight="2.4"
            />
          </button>
          <span
            class="mod-group-handle"
            v-tip="t('resource.groupDragTip')"
            @pointerdown="onSectionPointerDown($event, sec.id)"
            @click="onSectionToggle(sec.id)"
          >
            <span class="mod-group-name">{{ sec.label }}</span>
          </span>
          <span class="mod-group-count">{{ counts(sec) }}</span>
          <span v-if="sec.custom" class="group-acts">
            <button class="group-act" :disabled="busy" @click="openGroupForm(sec.id, sec.label)">
              {{ t("resource.groupRename") }}
            </button>
            <button
              class="group-act danger"
              :disabled="busy"
              @click="removeGroup(sec.id, sec.label)"
            >
              {{ t("resource.groupDelete") }}
            </button>
          </span>
        </div>

        <div v-show="isOpen(sec.id)" class="mod-group-body">
        <p v-if="!sec.items.length" class="empty-tip">
          {{ t("resource.groupEmptyHint") }}
        </p>
        <ModList
          :items="sec.items"
          :busy="busy"
          :dragging-key="draggingKey"
          @toggle="toggle"
          @remove="remove"
          @note="openNote"
          @open-folder="openFolder('mods', $event.file)"
          @drag-start="onDragStart"
        />
        </div>
      </section>

      <!-- 尾部插入线：拖到最末一块之后 -->
      <span v-if="idx === sections.length - 1 && showSectionLine(sections.length)" class="mod-group-insert" />
    </template>
  </div>

  <!-- 新建 / 重命名分组 -->
  <BaseModal
    v-if="groupForm"
    :title="groupForm.id ? t('resource.groupRenameTitle') : t('resource.groupAddTitle')"
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

  <!-- 模组备注：文件名只读展示（用户认的是名字，落盘的键是文件名） -->
  <BaseModal
    v-if="noteForm"
    :title="t('resource.modNoteTitle')"
    :closable="false"
    @close="noteForm = null"
  >
    <label class="field-label">{{ noteForm.item.name || noteForm.item.file }}</label>
    <p class="mod-note-file" :title="noteForm.item.file">{{ noteForm.item.file }}</p>
    <textarea
      v-model="noteForm.text"
      class="field-input mod-note-input"
      :placeholder="t('resource.modNotePlaceholder')"
      spellcheck="false"
      @keydown.ctrl.enter="saveNote(noteForm.text)"
    />
    <p class="mod-note-hint">{{ t("resource.modNoteHint") }}</p>
    <div class="modal-actions">
      <BaseButton
        v-if="noteForm.item.note"
        :disabled="noteBusy"
        @click="saveNote('')"
      >
        {{ t("resource.modNoteClear") }}
      </BaseButton>
      <BaseButton :disabled="noteBusy" @click="noteForm = null">
        {{ t("resource.cancel") }}
      </BaseButton>
      <BaseButton variant="primary" :disabled="noteBusy" @click="saveNote(noteForm.text)">
        {{ t("resource.save") }}
      </BaseButton>
    </div>
  </BaseModal>
</template>

<!-- 备注弹窗的样式走 scoped：BaseModal 是 `Teleport to="body"` 的，弹窗不在
     `.resource-layout` 里，窗口级 resource.css 那套前缀规则够不着它
     （同窗口的 ServerFormModal 也是这么处理的） -->
<style scoped>
/* 文件名（落盘的键）压暗一行 */
.mod-note-file {
  margin: 0 0 8px;
  font-size: 11.5px;
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.mod-note-input {
  min-height: 96px;
  resize: vertical;
  line-height: 1.5;
}

.mod-note-hint {
  margin: 8px 0 0;
  font-size: 11.5px;
  color: var(--text-dim);
  line-height: 1.5;
}
</style>
