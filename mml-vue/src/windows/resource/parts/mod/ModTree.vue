<script setup lang="ts">
// 模组视图之三：树（按 jar-in-jar 结构展开）
//
// 父条目 = mods 目录里的那个 jar；`jarInJar` 是它内置的模组（META-INF/jars、jarjar）。
// 内置模组装在父 jar 里，**没有独立文件**：只展示，不给启用 / 删除按钮，
// 顺序也由父级决定（不参与拖拽归组 —— 归组的是父 jar）。
//
// 自引用递归：本组件按文件名引用自己渲染下一层（Vue SFC 支持），
// 所以 jar 里套 jar 也能一层层展开，不用写死层数。
import { ref } from "vue";
import { t } from "../../../../lib/i18n";
import GlyphIcon from "../../../../components/ui/GlyphIcon.vue";
import { modRowKey, modSub, type ModViewEmits, type ModViewProps } from "../../types";

const props = withDefaults(
  defineProps<ModViewProps & { depth?: number }>(),
  { depth: 0 },
);
const emit = defineEmits<ModViewEmits>();

/** 折叠起来的父条目 key（默认展开） */
const collapsed = ref<Set<string>>(new Set());

function toggleCollapse(key: string) {
  const next = new Set(collapsed.value);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  collapsed.value = next;
}

function isOpen(key: string): boolean {
  return !collapsed.value.has(key);
}
</script>

<template>
  <template v-for="item in items" :key="modRowKey(item)">
    <div
      class="mod-tree-row"
      :class="{ 'mod-dragging': !!item.sha1 && draggingKey === item.sha1, child: depth > 0 }"
      :style="{ paddingLeft: 10 + depth * 22 + 'px' }"
      @pointerdown="depth === 0 ? emit('drag-start', { event: $event, item }) : undefined"
    >
      <!-- 有内置模组才给折叠箭头；内置行自己没有下级 -->
      <button
        v-if="item.jarInJar.length"
        class="mod-tree-caret"
        :class="{ collapsed: !isOpen(item.sha1 || item.name) }"
        v-tip="t('resource.modBuiltin')"
        @click.stop="toggleCollapse(item.sha1 || item.name)"
      >
        <GlyphIcon name="chevron-down" :size="13" :weight="2.4" />
      </button>
      <span v-else class="mod-tree-caret-placeholder" />

      <span class="mod-tree-name" :title="item.name || item.file">
        {{ item.name || item.file }}
      </span>
      <span v-if="depth > 0" class="badge badge-dim">{{ t("resource.modBuiltin") }}</span>
      <span v-if="item.disable" class="badge badge-dim">{{ t("resource.modDisabled") }}</span>
      <span v-if="item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
      <span v-if="item.core" class="badge">{{ t("resource.modCore") }}</span>
      <span class="mod-tree-sub" :title="modSub(item)">{{ modSub(item) }}</span>

      <!-- 只有顶层条目（真实文件）才有操作；按钮上不许起拖拽 -->
      <span v-if="depth === 0" class="item-actions" @pointerdown.stop>
        <button class="mini-btn" :disabled="busy" @click.stop="emit('toggle', item)">
          {{ item.disable ? t("resource.enable") : t("resource.disable") }}
        </button>
        <button class="mini-btn" :disabled="busy" @click.stop="emit('open-folder', item)">
          {{ t("resource.openFolder") }}
        </button>
        <button class="mini-btn danger" :disabled="busy" @click.stop="emit('remove', item)">
          {{ t("resource.delete") }}
        </button>
      </span>
    </div>

    <!-- 内置模组：递归展开下一层（默认展开，跟着父条目的折叠状态） -->
    <ModTree
      v-if="item.jarInJar.length && isOpen(item.sha1 || item.name)"
      :items="item.jarInJar"
      :busy="busy"
      :dragging-key="draggingKey"
      :depth="depth + 1"
      @toggle="emit('toggle', $event)"
      @remove="emit('remove', $event)"
      @open-folder="emit('open-folder', $event)"
      @drag-start="emit('drag-start', $event)"
    />
  </template>
</template>
