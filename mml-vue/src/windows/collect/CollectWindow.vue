<script setup lang="ts">
// 资源收藏窗口
//
// 布局与交互对应旧启动器的 CollectControl：顶部分组下拉（第 0 项「默认分组」= 不过滤）
// + 添加 / 删除分组 / 清空，下面一行类型过滤，主体是多选卡片网格，右键出菜单。
// 「安装选中」本轮未实现。
import { computed, onMounted, onUnmounted, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import { api, onCollectChange } from "../../lib/api";
import { loadGuiConfig, saveGuiConfig, type CollectConfig } from "../../lib/guiConfig";
import { t, tErr } from "../../lib/i18n";
import { showToast } from "../../lib/toast";
import type { CollectItemDto } from "../../lib/bindings";

defineEmits<{ (e: "close"): void }>();

/** 「默认分组」在下拉里是第 0 项，代表不过滤 */
const DEFAULT_GROUP = "";

const items = ref<CollectItemDto[]>([]);
const groups = ref<Record<string, string[]>>({});
const group = ref(DEFAULT_GROUP);
/** 勾选的收藏项 uuid */
const checked = ref<Set<string>>(new Set());

/** 类型过滤（持久化在 gui_config 的 collect 里） */
const filters = ref<CollectConfig>({
  modpack: true,
  showMod: true,
  resourcePack: true,
  shaderpack: true,
});

/** 勾选框 → 资源类型线串（`FileType::to_string()`；存档/数据包等不在收藏范围内） */
const FILTER_TYPES: Array<{ key: keyof CollectConfig; types: string[]; label: () => string }> = [
  { key: "modpack", types: ["modpack"], label: () => t("collect.filterModpack") },
  { key: "showMod", types: ["mod"], label: () => t("resource.mods") },
  { key: "resourcePack", types: ["resourcepack"], label: () => t("resource.resourcepacks") },
  { key: "shaderpack", types: ["shaderpack"], label: () => t("resource.shaders") },
];

const groupNames = computed(() => Object.keys(groups.value).sort());
/** 下拉项：第一项是「默认分组」 */
const groupOptions = computed(() => [DEFAULT_GROUP, ...groupNames.value]);

const visible = computed(() => {
  const ids = group.value === DEFAULT_GROUP ? null : new Set(groups.value[group.value] ?? []);
  // 只显示勾选的类型（与旧启动器一致：全不勾 = 空列表，没有「全选」兜底）
  const on = new Set(
    FILTER_TYPES.filter((f) => filters.value[f.key]).flatMap((f) => f.types),
  );
  return items.value.filter(
    (item) => on.has(item.fileType) && (ids === null || ids.has(item.uuid)),
  );
});

const checkedUuids = computed(() => [...checked.value]);

// ---------------- 加载 ----------------

async function reload() {
  const data = await api.collectGetData();
  items.value = data.items;
  groups.value = data.groups;
  // 分组可能已被删除，回落到「默认分组」
  if (group.value !== DEFAULT_GROUP && !(group.value in data.groups)) {
    group.value = DEFAULT_GROUP;
  }
  // 丢掉已不存在的勾选
  const alive = new Set(data.items.map((i) => i.uuid));
  checked.value = new Set([...checked.value].filter((u) => alive.has(u)));
}

onMounted(async () => {
  const cfg = await loadGuiConfig();
  if (cfg) {
    filters.value = cfg.collect;
  }
  const unlisten = await onCollectChange(() => {
    reload().catch(() => {});
  });
  onUnmounted(unlisten);

  reload().catch((e) => showToast(tErr(e)));
});

// ---------------- 类型过滤 ----------------

async function toggleFilter(key: keyof CollectConfig, value: boolean) {
  filters.value = { ...filters.value, [key]: value };
  await saveGuiConfig({ collect: { [key]: value } });
}

// ---------------- 勾选 ----------------

function toggleCheck(uuid: string, value: boolean) {
  const next = new Set(checked.value);
  if (value) {
    next.add(uuid);
  } else {
    next.delete(uuid);
  }
  checked.value = next;
}

// ---------------- 分组 ----------------

const showAddGroup = ref(false);
const newGroupName = ref("");
const deleteGroupTarget = ref("");
/** 清空确认弹窗是否打开 */
const clearOpen = ref(false);
/** 清空目标：null = 全部，否则为分组名 */
const clearGroup = ref<string | null>(null);
const addToGroupTarget = ref("");

async function confirmAddGroup() {
  const name = newGroupName.value.trim();
  showAddGroup.value = false;
  if (!name) {
    return;
  }
  try {
    await api.collectAddGroup(name);
    newGroupName.value = "";
    group.value = name;
  } catch (e) {
    showToast(tErr(e));
  }
}

async function confirmDeleteGroup() {
  const name = deleteGroupTarget.value;
  deleteGroupTarget.value = "";
  try {
    await api.collectRemoveGroup(name);
    showToast(t("tip.deleted"));
  } catch (e) {
    showToast(tErr(e));
  }
}

async function confirmClear() {
  const target = clearGroup.value;
  clearOpen.value = false;
  try {
    await api.collectClear(target);
    checked.value = new Set();
  } catch (e) {
    showToast(tErr(e));
  }
}

async function confirmAddToGroup() {
  const name = addToGroupTarget.value;
  addToGroupTarget.value = "";
  const uuids = checkedUuids.value;
  if (!name || uuids.length === 0) {
    return;
  }
  try {
    await api.collectSetGroupItems(name, uuids);
  } catch (e) {
    showToast(tErr(e));
  }
}

// ---------------- 条目操作 ----------------

/** 删除勾选项：默认分组下从收藏删除，真实分组下只从该分组移除 */
async function removeChecked() {
  const uuids = checkedUuids.value;
  if (uuids.length === 0) {
    return;
  }
  try {
    await api.collectRemoveItems(uuids, group.value === DEFAULT_GROUP ? null : group.value);
    checked.value = new Set();
  } catch (e) {
    showToast(tErr(e));
  }
}

function openUrl(item: CollectItemDto) {
  if (item.url) {
    api.openUrl(item.url).catch(() => {});
  }
}

// ---------------- 右键菜单 ----------------

const menu = ref<{ x: number; y: number; item: CollectItemDto } | null>(null);
/** 右键点中、当前要操作的条目（卡片按钮也用它） */
const focus = ref<CollectItemDto | null>(null);

function openMenu(e: MouseEvent, item: CollectItemDto) {
  menu.value = { x: e.clientX, y: e.clientY, item };
  // 与旧启动器一致：右键会把该项勾上
  toggleCheck(item.uuid, true);
}

function closeMenu() {
  menu.value = null;
}
</script>

<template>
  <WindowFrame :title="t('winTitle.collect')" @close="$emit('close')">
    <div class="collect-body" @click="closeMenu">
      <!-- 分组 + 操作 -->
      <div class="collect-head">
        <label class="field-label">{{ t("collect.groupLabel") }}</label>
        <select v-model="group" class="field-input group-sel">
          <option v-for="g in groupOptions" :key="g" :value="g">
            {{ g === DEFAULT_GROUP ? t("collect.defaultGroup") : g }}
          </option>
        </select>
        <BaseButton @click="showAddGroup = true">{{ t("collect.addGroup") }}</BaseButton>
        <BaseButton
          :disabled="group === DEFAULT_GROUP"
          @click="deleteGroupTarget = group"
        >
          {{ t("collect.deleteGroup") }}
        </BaseButton>
        <BaseButton
          :disabled="visible.length === 0"
          @click="clearGroup = group === DEFAULT_GROUP ? null : group; clearOpen = true"
        >
          {{ t("collect.clear") }}
        </BaseButton>
        <span class="spacer" />
        <span class="count">{{ t("collect.count", { n: visible.length }) }}</span>
      </div>

      <!-- 类型过滤 -->
      <div class="collect-filters">
        <label v-for="f in FILTER_TYPES" :key="f.key" class="filter-item">
          <input
            type="checkbox"
            :checked="filters[f.key]"
            @change="toggleFilter(f.key, ($event.target as HTMLInputElement).checked)"
          />
          {{ f.label() }}
        </label>
      </div>

      <!-- 条目网格 -->
      <div v-if="visible.length" class="collect-grid">
        <div
          v-for="item in visible"
          :key="item.uuid"
          class="collect-card"
          :class="{ active: checked.has(item.uuid) }"
          @mouseenter="focus = item"
          @mouseleave="focus = null"
          @contextmenu.prevent="openMenu($event, item)"
        >
          <input
            type="checkbox"
            class="card-check"
            :checked="checked.has(item.uuid)"
            @change="toggleCheck(item.uuid, ($event.target as HTMLInputElement).checked)"
          />
          <div class="card-icon">
            <img v-if="item.icon" :src="item.icon" alt="" loading="lazy" />
            <span v-else class="card-icon-none">?</span>
          </div>
          <div class="card-text">
            <div class="card-name" :title="item.name">{{ item.name }}</div>
            <div class="card-meta">{{ item.source }} · {{ item.fileType }}</div>
          </div>
          <button
            v-if="focus?.uuid === item.uuid || checked.has(item.uuid)"
            class="card-open"
            :title="t('collect.openUrl')"
            @click.stop="openUrl(item)"
          >
            <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
              <path d="M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.5 1.5" />
              <path d="M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7L12 19" />
            </svg>
          </button>
        </div>
      </div>
      <p v-else class="collect-empty">{{ t("collect.empty") }}</p>

      <!-- 右键菜单 -->
      <div
        v-if="menu"
        class="ctx-menu"
        :style="{ left: menu.x + 'px', top: menu.y + 'px' }"
        @click.stop
      >
        <button class="ctx-item" @click="openUrl(menu.item); closeMenu()">
          {{ t("collect.openUrl") }}
        </button>
        <button class="ctx-item" @click="removeChecked(); closeMenu()">
          {{ group === DEFAULT_GROUP ? t("collect.deleteFav") : t("collect.removeFromGroup") }}
        </button>
        <button class="ctx-item" @click="addToGroupTarget = groupNames[0] ?? ''; closeMenu()">
          {{ t("collect.addToGroup") }}
        </button>
      </div>
    </div>

    <!-- 添加分组 -->
    <BaseModal v-if="showAddGroup" :title="t('collect.addGroupTitle')" @close="showAddGroup = false">
      <input
        v-model="newGroupName"
        class="field-input"
        :placeholder="t('collect.groupPlaceholder')"
        spellcheck="false"
        @keydown.enter="confirmAddGroup"
      />
      <div class="modal-actions">
        <BaseButton @click="showAddGroup = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" @click="confirmAddGroup">{{ t("add.yes") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 删除分组确认 -->
    <BaseModal
      v-if="deleteGroupTarget"
      :title="t('collect.deleteGroup')"
      @close="deleteGroupTarget = ''"
    >
      <p class="confirm-text">{{ t("collect.deleteGroupConfirm", { name: deleteGroupTarget }) }}</p>
      <div class="modal-actions">
        <BaseButton @click="deleteGroupTarget = ''">{{ t("add.no") }}</BaseButton>
        <BaseButton variant="primary" @click="confirmDeleteGroup">{{ t("add.yes") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 清空确认 -->
    <BaseModal v-if="clearOpen" :title="t('collect.clear')" @close="clearOpen = false">
      <p class="confirm-text">
        {{
          clearGroup === null
            ? t("collect.clearAllConfirm")
            : t("collect.clearGroupConfirm", { name: clearGroup })
        }}
      </p>
      <div class="modal-actions">
        <BaseButton @click="clearOpen = false">{{ t("add.no") }}</BaseButton>
        <BaseButton variant="primary" @click="confirmClear">{{ t("add.yes") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 添加到分组 -->
    <BaseModal
      v-if="addToGroupTarget !== ''"
      :title="t('collect.addToGroupTitle')"
      @close="addToGroupTarget = ''"
    >
      <select v-model="addToGroupTarget" class="field-input">
        <option v-for="g in groupNames" :key="g" :value="g">{{ g }}</option>
      </select>
      <div class="modal-actions">
        <BaseButton @click="addToGroupTarget = ''">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" @click="confirmAddToGroup">{{ t("add.yes") }}</BaseButton>
      </div>
    </BaseModal>
  </WindowFrame>
</template>

<style scoped>
.collect-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 100%;
}

.collect-head {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
}

.group-sel {
  width: 180px;
}

.collect-head .spacer {
  flex: 1;
}

.count {
  font-size: 12px;
  color: var(--text-dim);
}

.collect-filters {
  display: flex;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
}

.filter-item {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  color: var(--text);
  cursor: pointer;
}

.collect-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 10px;
}

.collect-card {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  width: 358px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-panel);
}

.collect-card.active {
  border-color: var(--accent);
}

.card-check {
  flex: 0 0 auto;
}

.card-icon {
  flex: 0 0 auto;
  width: 60px;
  height: 60px;
  border-radius: 6px;
  overflow: hidden;
  background: var(--bg-hover);
  display: flex;
  align-items: center;
  justify-content: center;
}

.card-icon img {
  width: 100%;
  height: 100%;
  object-fit: cover;
}

.card-icon-none {
  color: var(--text-dim);
  font-size: 20px;
}

.card-text {
  flex: 1;
  min-width: 0;
}

.card-name {
  font-size: 13.5px;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.card-meta {
  margin-top: 4px;
  font-size: 12px;
  color: var(--text-dim);
}

.card-open {
  flex: 0 0 auto;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 26px;
  height: 26px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: transparent;
  color: var(--text-dim);
  cursor: pointer;
}

.card-open:hover {
  color: var(--accent);
  border-color: var(--accent);
}

.collect-empty {
  margin: 40px 0;
  text-align: center;
  color: var(--text-dim);
  font-size: 13px;
}

.ctx-menu {
  position: fixed;
  z-index: 30;
  display: flex;
  flex-direction: column;
  min-width: 140px;
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-panel);
  box-shadow: 0 8px 24px rgb(0 0 0 / 30%);
}

.ctx-item {
  padding: 7px 10px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  text-align: left;
  cursor: pointer;
}

.ctx-item:hover {
  background: var(--bg-hover);
}

.confirm-text {
  margin: 4px 0 0;
  font-size: 13.5px;
  color: var(--text);
  line-height: 1.6;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 18px;
}
</style>
