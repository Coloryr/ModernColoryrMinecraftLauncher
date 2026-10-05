<script setup lang="ts">
// 模组视图之一：列表（每行 图标 + 名称/徽标 + 副标题 + 操作）—— 与其它分类的行同一观感
//
// **内置模组（jar-in-jar）就展开在同一列里**：有下级的行前面多一个展开箭头，
// 展开后按 22px 缩进显示下一层（那是这个 jar 里装的模组）。原先这是独立的「树形」视图，
// 两者已合并 —— 列表本来就要显示"这个包内置了什么"，分成两个视图等于让用户在
// "看得见层级"和"看得见图标/副标题"之间二选一。
//
// 自引用递归：本组件按文件名引用自己渲染下一层（Vue SFC 支持），
// 所以 jar 里套 jar 也能一层层展开，不用写死层数。
//
// 内置模组装在父 jar 里，**没有独立文件**：只有展示，不给操作按钮，也不参与拖拽归组
// （归组的是父 jar）。
//
// 排序：**按 modid 正序**（见 types.ts 的 `sortedMods`）—— 与表格视图共用同一份比较器，
// 两个视图的顺序必须一致，否则切换视图会看着"顺序变了"。
import { computed, ref } from "vue";
import { t } from "../../../../lib/i18n";
import GlyphIcon from "../../../../components/ui/GlyphIcon.vue";
import ResourceRow from "../ResourceRow.vue";
import {
  modRowKey,
  modSub,
  sortedMods,
  type ModViewEmits,
  type ModViewProps,
} from "../../types";

const props = withDefaults(defineProps<ModViewProps & { depth?: number }>(), { depth: 0 });
const emit = defineEmits<ModViewEmits>();

/** 按 modid 正序排好的本层条目（递归渲染的每一层各自排） */
const rows = computed(() => sortedMods(props.items));

/**
 * **展开**的父条目 key（默认**收起**）
 *
 * 内置模组（jar-in-jar）装在父 jar 里、多数时候只是"顺带一提"：
 * 默认铺开会把列表撑得又长又碎（一个包动辄带好几个内置库），
 * 想看的那个模组反而被挤到屏幕外。所以默认只显示父 jar，点箭头才展开。
 *
 * 存"展开的那些"而不是"收起的那些"：默认收起，空集合即初始状态。
 */
const expanded = ref<Set<string>>(new Set());

function toggleExpand(key: string) {
  const next = new Set(expanded.value);
  if (next.has(key)) {
    next.delete(key);
  } else {
    next.add(key);
  }
  expanded.value = next;
}

function isOpen(key: string): boolean {
  return expanded.value.has(key);
}

/** 展开状态用的键：顶层认 sha1，内置条目没有 sha1 时退回名字 */
function keyOf(item: ModViewProps["items"][number]): string {
  return modRowKey(item);
}
</script>

<template>
  <template v-for="item in rows" :key="keyOf(item)">
    <ResourceRow
      :icon="item.icon"
      :name="item.name || item.file"
      :depth="depth"
      :class="{ 'mod-dragging': !!item.sha1 && draggingKey === item.sha1 }"
      @pointerdown="depth === 0 ? emit('drag-start', { event: $event, item }) : undefined"
    >
      <!-- 展开箭头：只有带内置模组的行才有（内置行自己没有下级）；默认收起 -->
      <template #lead>
        <button
          v-if="item.jarInJar.length"
          class="mod-tree-caret"
          :class="{ collapsed: !isOpen(keyOf(item)) }"
          v-tip="
            isOpen(keyOf(item))
              ? t('resource.modBuiltinCollapse')
              : t('resource.modBuiltinExpand')
          "
          @click.stop="toggleExpand(keyOf(item))"
          @pointerdown.stop
        >
          <GlyphIcon name="chevron-down" :size="13" :weight="2.4" />
        </button>
        <!-- 占位：没有下级的行也留出箭头的宽度，图标与名字才能对齐成一列 -->
        <span v-else class="mod-tree-caret-placeholder" />
      </template>

      <template #badges>
        <span v-if="depth > 0" class="badge badge-dim">{{ t("resource.modBuiltin") }}</span>
        <span v-if="item.disable" class="badge badge-dim">{{ t("resource.modDisabled") }}</span>
        <span v-if="item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
        <span v-if="item.core" class="badge">{{ t("resource.modCore") }}</span>
        <span v-if="depth === 0 && item.jarInJar.length" class="badge">
          {{ t("resource.modBuiltinCount", { n: item.jarInJar.length }) }}
        </span>
      </template>

      <template #sub>
        <span>{{ modSub(item) }}</span>
        <span v-if="item.note" class="mod-note" :title="item.note">
          · {{ t("resource.modNotePrefix") }}{{ item.note }}
        </span>
      </template>

      <!-- 只有顶层条目（真实文件）才有操作；按钮上不许起拖拽 -->
      <template v-if="depth === 0" #actions>
        <span class="acts-stop" @pointerdown.stop>
          <button class="mini-btn" :disabled="busy" @click.stop="emit('toggle', item)">
            {{ item.disable ? t("resource.enable") : t("resource.disable") }}
          </button>
          <button
            class="mini-btn"
            :class="{ on: !!item.note }"
            :disabled="busy"
            @click.stop="emit('note', item)"
          >
            {{ t("resource.modNote") }}
          </button>
          <button class="mini-btn" :disabled="busy" @click.stop="emit('open-folder', item)">
            {{ t("resource.openFolder") }}
          </button>
          <button class="mini-btn danger" :disabled="busy" @click.stop="emit('remove', item)">
            {{ t("resource.delete") }}
          </button>
        </span>
      </template>
    </ResourceRow>

    <!-- 内置模组：递归展开下一层（默认展开，跟着父条目的折叠状态） -->
    <ModList
      v-if="item.jarInJar.length && isOpen(keyOf(item))"
      :items="item.jarInJar"
      :busy="busy"
      :dragging-key="draggingKey"
      :depth="depth + 1"
      @toggle="emit('toggle', $event)"
      @remove="emit('remove', $event)"
      @note="emit('note', $event)"
      @open-folder="emit('open-folder', $event)"
      @drag-start="emit('drag-start', $event)"
    />
  </template>
</template>
