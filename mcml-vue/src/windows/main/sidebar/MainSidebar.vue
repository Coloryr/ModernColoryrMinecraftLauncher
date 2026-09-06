<script setup lang="ts">
// 主窗口侧栏：模式切换 / 搜索 / 分组列表（拖拽排序）/ 平铺
// 状态与交互全部通过 props / emits 与 MainWindow 联通
import SegmentedTabs from "../../../components/ui/SegmentedTabs.vue";
import InstanceIcon from "../../../components/InstanceIcon.vue";
import { t } from "../../../lib/i18n";
import type { InstanceInfo } from "../../../lib/types";
import type { DragCandidate, GroupView, ViewMode } from "../types";

const props = defineProps<{
  mode: ViewMode;
  modeOptions: Array<{ value: string; label: string; icon: string }>;
  searchText: string;
  groups: GroupView[];
  filteredGroups: GroupView[];
  filteredInstances: InstanceInfo[];
  searching: boolean;
  selected: InstanceInfo | null;
  multiSelect: boolean;
  selectedIds: Set<string>;
  collapsedGroups: Record<string, boolean>;
  dragActive: DragCandidate | null;
  draggingUuid: string | null;
  isCollapsed: (name: string) => boolean;
  onDragPointerDown: (e: PointerEvent, cand: DragCandidate) => void;
  onInstClick: (inst: InstanceInfo) => void;
  onInstContext: (e: MouseEvent, inst: InstanceInfo) => void;
  onGroupContext: (e: MouseEvent, groupName: string) => void;
  onGroupTitleClick: (name: string) => void;
  isInstInsert: (groupName: string, idx: number) => boolean;
  isInstInsertEnd: (groupName: string) => boolean;
  isGroupInsert: (index: number) => boolean;
}>();

const emit = defineEmits<{
  (e: "update:mode", v: ViewMode): void;
  (e: "update:searchText", v: string): void;
  (e: "add-instance"): void;
  (e: "add-group"): void;
  (e: "collapse"): void;
}>();

function onSearchInput(e: Event) {
  emit("update:searchText", (e.target as HTMLInputElement).value);
}
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-head">
      <SegmentedTabs
        :model-value="mode"
        :options="modeOptions"
        @update:model-value="emit('update:mode', $event as ViewMode)"
      />
      <div class="sidebar-head-actions">
        <button class="icon-btn" :title="t('sidebar.collapse')" @click="emit('collapse')">‹</button>
      </div>
    </div>

    <!-- 实例搜索 -->
    <div class="search-box">
      <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
        <circle cx="11" cy="11" r="7" />
        <path d="m20 20-3.5-3.5" />
      </svg>
      <input
        :value="searchText"
        class="search-input"
        :placeholder="t('search.placeholder')"
        spellcheck="false"
        @input="onSearchInput"
      />
      <button v-if="searchText" class="search-clear" @click="emit('update:searchText', '')">✕</button>
    </div>

    <!-- 分组：可收缩，组内顶部有添加 -->
    <div v-if="mode === 'group'" class="group-list">
      <template v-for="(g, gi) in filteredGroups" :key="g.name">
        <!-- 分组拖拽插入占位 -->
        <div v-if="isGroupInsert(gi)" class="drop-ghost-group">
          <span class="drop-ghost-group-name">{{ dragActive?.groupName }}</span>
        </div>
        <div class="group-block" :data-group="g.name">
        <div
          class="group-title-row"
          @contextmenu.prevent="onGroupContext($event, g.name)"
          @pointerdown="onDragPointerDown($event, { kind: 'group', groupName: g.name })"
        >
          <button class="group-title" :title="t('group.collapse')" @click="onGroupTitleClick(g.name)">
            <svg
              class="group-chevron"
              :class="{ collapsed: isCollapsed(g.name) }"
              viewBox="0 0 24 24"
              width="13"
              height="13"
              fill="none"
              stroke="currentColor"
              stroke-width="2"
            >
              <path d="m6 9 6 6 6-6" />
            </svg>
            <span>{{ g.name }}</span>
            <span class="group-count">{{ g.items.length }}</span>
          </button>
        </div>

        <!-- 拖拽实例时自动展开，便于投放 -->
        <div
          v-show="!isCollapsed(g.name) || searching || dragActive?.kind === 'instance'"
          class="group-items"
        >
          <!-- 添加实例（与实例行同尺寸；多选时隐藏，拖拽实例时禁用并变色） -->
          <button
            v-show="!multiSelect"
            class="add-inst-row"
            :disabled="dragActive?.kind === 'instance'"
            @click="emit('add-instance')"
          >
            <span class="add-inst-icon">＋</span>
            <span class="add-inst-text">{{ t("list.add") }}</span>
          </button>
          <template v-for="(inst, idx) in g.items" :key="inst.uuid">
            <!-- 正在拖拽的实例行不渲染（位置由插入占位显示），避免出现空位 / 双占位 -->
            <template v-if="inst.uuid !== draggingUuid">
              <!-- 实例拖拽插入占位（淡化的实例，位于鼠标对应位置） -->
              <div v-if="isInstInsert(g.name, idx)" class="drop-ghost-row">
                <InstanceIcon :name="dragActive?.instance?.name ?? ''" :uuid="dragActive?.instance?.uuid ?? '0'" :size="38" />
                <span class="inst-name">{{ dragActive?.instance?.name ?? "" }}</span>
              </div>
              <div
                class="inst-row"
                :data-uuid="inst.uuid"
                :class="{
                  active: selected?.uuid === inst.uuid,
                  'multi-checked': multiSelect && selectedIds.has(inst.uuid),
                }"
                @pointerdown="onDragPointerDown($event, { kind: 'instance', instance: inst })"
                @click="onInstClick(inst)"
                @contextmenu.prevent="onInstContext($event, inst)"
              >
                <span v-if="multiSelect" class="row-check" :class="{ on: selectedIds.has(inst.uuid) }">✓</span>
                <InstanceIcon :name="inst.name" :uuid="inst.uuid" :size="38" />
                <span v-if="inst.loader !== 'normal'" class="loader-text">{{ t(`add.loader.${inst.loader}`) }}</span>
                <span class="inst-name">{{ inst.name }}</span>
                <span v-if="inst.running" class="run-dot" title="running"></span>
              </div>
            </template>
          </template>
          <!-- 分组末尾插入占位 -->
          <div v-if="isInstInsertEnd(g.name)" class="drop-ghost-row">
            <InstanceIcon :name="dragActive?.instance?.name ?? ''" :uuid="dragActive?.instance?.uuid ?? '0'" :size="38" />
            <span class="inst-name">{{ dragActive?.instance?.name ?? "" }}</span>
          </div>
        </div>
        </div>
      </template>
      <!-- 分组列表末尾插入占位 -->
      <div v-if="isGroupInsert(filteredGroups.length)" class="drop-ghost-group">
        <span class="drop-ghost-group-name">{{ dragActive?.groupName }}</span>
      </div>
      <div v-if="groups.length === 0" class="empty-tip">{{ t("list.empty") }}</div>
      <div v-else-if="filteredGroups.length === 0" class="empty-tip">{{ t("search.empty") }}</div>

      <!-- 添加分组（虚线行，与组内添加实例一致） -->
      <button
        class="add-inst-row add-group-row"
        :disabled="dragActive?.kind === 'instance'"
        @click="emit('add-group')"
      >
        <span class="add-inst-icon">＋</span>
        <span>{{ t("group.addGroup") }}</span>
      </button>
    </div>

    <!-- 平铺：首格为添加 -->
    <div v-else class="tile-list">
      <div v-show="!multiSelect" class="tile add-tile" @click="emit('add-instance')">
        <span class="add-plus">＋</span>
        <span class="tile-name">{{ t("group.add") }}</span>
      </div>
      <div
        v-for="inst in filteredInstances"
        :key="inst.uuid"
        class="tile"
        :class="{
          active: selected?.uuid === inst.uuid,
          'multi-checked': multiSelect && selectedIds.has(inst.uuid),
        }"
        @click="onInstClick(inst)"
        @contextmenu.prevent="onInstContext($event, inst)"
      >
        <span v-if="multiSelect" class="row-check" :class="{ on: selectedIds.has(inst.uuid) }">✓</span>
        <InstanceIcon :name="inst.name" :uuid="inst.uuid" :size="44" />
        <span
          v-if="inst.loader !== 'normal'"
          class="loader-corner loader-text"
        >{{ t(`add.loader.${inst.loader}`) }}</span>
        <span class="tile-name">{{ inst.name }}</span>
        <span v-if="inst.running" class="run-dot" title="running"></span>
      </div>
      <div v-if="filteredInstances.length === 0" class="empty-tip tile-empty">{{ t("search.empty") }}</div>
    </div>
  </aside>
</template>

<style scoped>
.sidebar {
  width: 320px;
  min-width: 320px;
  background: var(--bg-side);
  border-right: 1px solid var(--border);
  display: flex;
  flex-direction: column;
  min-height: 0;
}

.main.side-right .sidebar {
  border-right: none;
  border-left: 1px solid var(--border);
}

.sidebar-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 12px 12px 10px;
}

.sidebar-head-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}

/* 实例搜索框 */
.search-box {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 12px 8px;
  padding: 0 10px;
  height: 34px;
  border-radius: 9px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
  flex-shrink: 0;
}

.search-box:focus-within {
  border-color: var(--accent);
}

.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  background: transparent;
  color: var(--text);
  font-size: 12.5px;
  outline: none;
  font-family: inherit;
}

.search-input::placeholder {
  color: var(--text-dim);
}

.search-clear {
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  cursor: pointer;
  padding: 2px 4px;
}

.search-clear:hover {
  color: var(--text);
}

@media (max-width: 880px) {
  .sidebar {
    position: fixed;
    top: 64px;
    bottom: 0;
    left: 0;
    z-index: 160;
    box-shadow: var(--shadow-lg);
  }

  .main.side-right .sidebar {
    left: auto;
    right: 0;
  }
}

.icon-btn {
  width: 30px;
  height: 30px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text);
  font-size: 16px;
  line-height: 1;
  cursor: pointer;
  transition: all 0.15s;
  flex-shrink: 0;
}

.icon-btn:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
}

/* ----- 分组模式列表 ----- */

.group-list {
  flex: 1;
  overflow-y: auto;
  padding: 2px 10px 14px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.group-title-row {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-bottom: 2px;
}

.group-title {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  font-weight: 700;
  color: var(--text-dim);
  padding: 8px 6px;
  border: none;
  background: transparent;
  cursor: pointer;
  letter-spacing: 0.5px;
  font-family: inherit;
  border-radius: 8px;
  transition: background 0.12s;
  text-align: left;
}

.group-title:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.group-chevron {
  transition: transform 0.15s;
  flex-shrink: 0;
}

.group-chevron.collapsed {
  transform: rotate(-90deg);
}

.group-count {
  font-size: 11px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 20px;
  padding: 0 8px;
  color: var(--text-dim);
  font-weight: 500;
  margin-left: auto;
}

.group-items {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-top: 0;
}

/* 添加实例行（与实例行同尺寸，位于实例上方） */
.add-inst-row {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 9px 12px;
  border: 1px dashed var(--border);
  border-radius: 10px;
  background: transparent;
  color: var(--text-dim);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.12s;
  min-height: 48px;
}

.add-inst-row:hover:not(:disabled) {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

/* 拖拽实例时禁用：变灰、禁止光标 */
.add-inst-row:disabled {
  cursor: not-allowed;
  border-color: var(--border);
  color: var(--text-dim);
  background: var(--bg);
  opacity: 0.55;
}

.add-inst-row:disabled .add-inst-icon {
  background: var(--bg-hover);
  color: var(--text-dim);
}

/* 添加分组行（列表底部） */
.add-group-row {
  margin-top: 6px;
}

.add-inst-icon {
  width: 38px;
  height: 38px;
  border-radius: 10px;
  background: var(--bg-hover);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 20px;
  flex-shrink: 0;
}

.add-inst-row:hover:not(:disabled) .add-inst-icon {
  background: var(--accent-soft);
}

.inst-row {
  position: relative;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  border-radius: 10px;
  cursor: grab;
  transition: background 0.12s;
  min-height: 50px;
}

.inst-row:active {
  cursor: grabbing;
}

/* 拖拽插入占位（淡化的实例 / 分组） */
.drop-ghost-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 9px 12px;
  min-height: 50px;
  border-radius: 10px;
  border: 1.5px dashed var(--accent-border);
  background: var(--accent-soft);
  color: var(--text-dim);
  font-size: 13px;
  opacity: 0.75;
  pointer-events: none;
}

.drop-ghost-group {
  margin: 2px 0;
  padding: 9px 12px;
  border: 1.5px dashed var(--accent-border);
  border-radius: 10px;
  background: var(--accent-soft);
  color: var(--text-dim);
  font-size: 13px;
  font-weight: 700;
  opacity: 0.75;
  pointer-events: none;
}

.inst-row:hover {
  background: var(--bg-hover);
}

.inst-row.active {
  background: var(--accent-soft);
  outline: 1px solid var(--accent-border);
}

.inst-name {
  font-size: 13.5px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  flex: 1;
}

/* ----- 平铺模式网格 ----- */

.tile-list {
  flex: 1;
  overflow-y: auto;
  padding: 6px 10px 14px;
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
  gap: 10px;
  align-content: start;
}

.tile {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 16px 8px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.tile:hover {
  background: var(--bg-hover);
  transform: translateY(-2px);
}

.tile.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.tile.add-tile {
  border: 1px solid var(--border);
  background: var(--bg-card);
  justify-content: center;
}

.tile.add-tile:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

.add-plus {
  font-size: 26px;
  line-height: 1;
  color: var(--text-dim);
}

.tile.add-tile:hover .add-plus {
  color: var(--accent);
}

.add-tile .tile-name {
  color: var(--accent);
  font-weight: 600;
}

.tile-empty {
  grid-column: 1 / -1;
}

.loader-corner {
  position: absolute;
  top: 8px;
  left: 8px;
}

.tile-name {
  font-size: 12px;
  text-align: center;
  line-height: 1.3;
  word-break: break-all;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.tile .run-dot {
  position: absolute;
  top: 10px;
  right: 10px;
}

.run-dot {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: var(--green);
  box-shadow: 0 0 6px var(--green);
  flex-shrink: 0;
}

/* 多选勾选 */
.row-check {
  position: absolute;
  left: 5px;
  top: 5px;
  z-index: 3;
  width: 19px;
  height: 19px;
  border-radius: 50%;
  border: 1.5px solid var(--text-dim);
  background: var(--bg-card);
  color: transparent;
  font-size: 12px;
  display: flex;
  align-items: center;
  justify-content: center;
  transition: all 0.12s;
  pointer-events: none;
}

.row-check.on {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.inst-row.multi-checked {
  background: var(--accent-soft);
  outline: 1px solid var(--accent-border);
}

.tile.multi-checked {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.loader-text {
  font-size: 10px;
  padding: 1px 7px;
  border-radius: 8px;
  background: var(--accent-soft);
  color: var(--accent);
  white-space: nowrap;
  flex-shrink: 0;
  line-height: 1.6;
}
</style>
