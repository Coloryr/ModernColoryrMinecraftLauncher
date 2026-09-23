<script setup lang="ts">
// 主窗口右键菜单：分组 / 实例 / 移动分组 / 多选 四种视图
// 所有动作通过事件抛给父组件处理
import { t } from "../../../lib/i18n";
import type { CtxMenuState, GroupView, InstMenuAction } from "../types";

defineProps<{
  menu: CtxMenuState | null;
  moveGroupView: boolean;
  groups: GroupView[];
}>();

const emit = defineEmits<{
  (e: "select-all", group?: string): void;
  (e: "launch-group", group?: string): void;
  (e: "open-move-view", group?: string): void;
  (e: "delete-group", group?: string): void;
  (e: "inst-action", id: InstMenuAction): void;
  (e: "back"): void;
  (e: "move-target", name: string | null): void;
  (e: "multi-move"): void;
  (e: "multi-delete"): void;
  (e: "multi-launch"): void;
}>();
</script>

<template>
  <div
    v-if="menu"
    class="ctx-menu"
    :style="{ left: menu.x + 'px', top: menu.y + 'px' }"
    @contextmenu.prevent
    @click.stop
  >
    <!-- 分组菜单：全选 / 启动全部 / 转移分组 / 删除分组 -->
    <template v-if="menu.kind === 'group'">
      <button class="ctx-item" @click="emit('select-all', menu.group)">
        {{ t("multi.selectAll") }}
      </button>
      <button class="ctx-item" @click="emit('launch-group', menu.group)">
        {{ t("multi.launch") }}
      </button>
      <button class="ctx-item" @click="emit('open-move-view', menu.group)">
        {{ t("group.moveTo") }}
      </button>
      <div class="ctx-sep"></div>
      <button
        class="ctx-item danger"
        :disabled="menu.group === t('group.default')"
        @click="emit('delete-group', menu.group)"
      >
        {{ t("group.delete") }}
      </button>
    </template>

    <!-- 实例菜单：启动 / 打开文件夹 / 日志 / 配置 / 重命名 / 删除 -->
    <template v-else-if="menu.kind === 'instance'">
      <button class="ctx-item" @click="emit('inst-action', 'launch')">
        {{ t("launch.play") }}
      </button>
      <button class="ctx-item" @click="emit('inst-action', 'openFolder')">
        {{ t("actions.openFolder") }}
      </button>
      <button class="ctx-item" @click="emit('inst-action', 'viewLog')">
        {{ t("actions.viewLog") }}
      </button>
      <button class="ctx-item" @click="emit('inst-action', 'editConfig')">
        {{ t("actions.editConfig") }}
      </button>
      <button class="ctx-item" @click="emit('inst-action', 'rename')">
        {{ t("actions.rename") }}
      </button>
      <div class="ctx-sep"></div>
      <button class="ctx-item danger" @click="emit('inst-action', 'delete')">
        {{ t("actions.delete") }}
      </button>
    </template>

    <!-- 移动 / 转移分组：选择目标分组 -->
    <template v-else-if="moveGroupView">
      <button class="ctx-item ctx-back" @click="emit('back')">
        {{ t("multi.back") }}
      </button>
      <div class="ctx-sep"></div>
      <button class="ctx-item" @click="emit('move-target', null)">
        <span class="ctx-label">{{ t("group.default") }}</span>
      </button>
      <button
        v-for="g in groups"
        :key="g.name"
        class="ctx-item"
        @click="emit('move-target', g.name)"
      >
        <span class="ctx-label">{{ g.name }}</span>
        <span class="ctx-count">{{ g.items.length }}</span>
      </button>
    </template>

    <!-- 多选实例操作菜单 -->
    <template v-else>
      <button class="ctx-item" @click="emit('multi-move')">
        {{ t("multi.moveGroup") }}
      </button>
      <button class="ctx-item danger" @click="emit('multi-delete')">
        {{ t("multi.delete") }}
      </button>
      <button class="ctx-item" @click="emit('multi-launch')">
        {{ t("multi.launch") }}
      </button>
    </template>
  </div>
</template>

<style scoped>
.ctx-menu {
  position: fixed;
  z-index: 120;
  min-width: 180px;
  max-height: 320px;
  overflow-y: auto;
  padding: 6px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: var(--shadow-lg);
  animation: ctx-in 0.12s ease;
}

@keyframes ctx-in {
  from {
    opacity: 0;
    transform: translateY(-4px) scale(0.98);
  }
  to {
    opacity: 1;
    transform: none;
  }
}

.ctx-item {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 9px 12px;
  border: none;
  border-radius: 8px;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: background 0.1s;
}

.ctx-item:hover {
  background: var(--bg-hover);
}

.ctx-item.danger {
  color: var(--red);
}

.ctx-item.danger:hover {
  background: rgba(255, 95, 86, 0.12);
}

.ctx-item:disabled {
  opacity: 0.4;
  cursor: not-allowed;
  background: transparent;
}

.ctx-back {
  color: var(--text-dim);
}

.ctx-label {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.ctx-count {
  font-size: 11.5px;
  color: var(--text-dim);
  margin-left: auto;
}

.ctx-sep {
  height: 1px;
  background: var(--border);
  margin: 4px 6px;
}
</style>
