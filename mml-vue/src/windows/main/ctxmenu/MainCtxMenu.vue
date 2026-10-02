<script setup lang="ts">
// 主窗口右键菜单：分组 / 实例 / 移动分组 / 多选 四种视图
// 所有动作通过事件抛给父组件处理
//
// 定位：`position: fixed` 钉在点击处，但**渲染后要量一次并收进窗口**——
// 否则靠近窗口下/右边缘点开时，菜单会超出可视区被裁掉（实例菜单 8 项约 285px 高）。
import { nextTick, ref, watchEffect } from "vue";
import { t } from "../../../lib/i18n";
import type { CtxMenuState, GroupView, InstMenuAction } from "../types";

const props = defineProps<{
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

/** 菜单根元素与最终落点（先按点击处放，量完再修正） */
const el = ref<HTMLElement | null>(null);
const pos = ref({ x: 0, y: 0 });
/** 与窗口边缘至少留这么宽（px） */
const MARGIN = 8;

watchEffect(async () => {
  const menu = props.menu;
  // 这几个依赖都会改变菜单高度（切换分组/实例/移动视图、目标分组增删），要跟着重新定位
  void props.moveGroupView;
  void props.groups.length;
  if (!menu) return;

  pos.value = { x: menu.x, y: menu.y };
  // 等菜单渲染出来才有尺寸可量（内容由 v-if 分支决定，量不到就保持点击处）
  await nextTick();
  const rect = el.value?.getBoundingClientRect();
  if (!rect) return;

  const overX = rect.right - (window.innerWidth - MARGIN);
  const overY = rect.bottom - (window.innerHeight - MARGIN);
  pos.value = {
    x: overX > 0 ? Math.max(MARGIN, menu.x - overX) : menu.x,
    y: overY > 0 ? Math.max(MARGIN, menu.y - overY) : menu.y,
  };
});
</script>

<template>
  <div
    v-if="menu"
    ref="el"
    class="ctx-menu"
    :style="{ left: pos.x + 'px', top: pos.y + 'px' }"
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

    <!-- 实例菜单：启动 / 打开文件夹 / 日志 / 修改图标 / 重命名 / 删除
         （「修改实例配置」已去掉：右侧面板本来就有设置区） -->
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
      <button class="ctx-item" @click="emit('inst-action', 'changeIcon')">
        {{ t("actions.pickImage") }}
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
  /* 屏幕很矮时（收进窗口后仍放不下）才内部滚动；正常窗口下列表项全部可见 */
  max-height: calc(100vh - 16px);
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
