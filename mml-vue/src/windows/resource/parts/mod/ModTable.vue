<script setup lang="ts">
// 模组视图之二：表格（14 列，**分组名就是第一列**）
//
// 表格不是"每个分组各一张小表"，而是**一张总表**：分组名占第一列（带折叠箭头），
// 模组行挂在各自分组名下 —— 这样"哪个模组在哪个组"一眼看得到，也不必在每块上重复表头。
//
// 列序（与用户给的参考图一致）：
//   分组名 | 启用 | 备注 | modid | 名字 | 版本 | 加载器类型 | 加载侧
//          | 下载源 | 项目编号 | 文件编号 | 路径 | 作者 | 网页链接
//
// **不做点表头排序**：固定按 modid 正序（用户指定）。同 modid 的再按名字兜底，
// 免得顺序在两次扫描之间抖动。内置模组（jar-in-jar）跟着父行、排在其父行之后展开。
import { computed, ref } from "vue";
import { t } from "../../../../lib/i18n";
import GlyphIcon from "../../../../components/ui/GlyphIcon.vue";
import type { ModItemDto } from "../../../../lib/bindings";
import { modRowKey, sortedMods, type ModViewEmits, type ModViewProps } from "../../types";

const props = withDefaults(
  defineProps<
    ModViewProps & {
      /** 这一批属于哪个分组（表格第一列显示它；分组模式下一张表传一批） */
      groupLabel?: string;
      /** 该分组当前的折叠状态（false = 收起，不渲染模组行） */
      groupOpen?: boolean;
      /** 该分组的落点信息（拖拽时整块高亮用），与分组块的 data 属性同一份口径 */
      custom?: boolean;
      /** 分组键（自建分组 = 组名；状态分组 = 带前缀的内置键） */
      groupKey?: string;
      /** 正在被拖动排序的分组键（整块淡出） */
      draggingGroup?: string | null;
      depth?: number;
      /**
       * 不画 15 列表头（表格视图里分组是**多个块**，但同属一张总表 ——
       * 只有第一块画表头，其余块关掉，否则一屏里重复十几列）
       */
      hideHeader?: boolean;
    }
  >(),
  {
    groupLabel: "",
    groupOpen: true,
    custom: false,
    groupKey: "",
    draggingGroup: null,
    depth: 0,
    hideHeader: false,
  },
);

const emit = defineEmits<ModViewEmits>();

/**
 * 表格里展开的行（父行 + 缩进的内部模组行拍平成一串）
 *
 * 内置模组装在父 jar 里、没有独立文件：它们**没有**启用 / 备注 / 下载源这些东西，
 * 所以只在"名字"那一列缩进显示，其余列留空 —— 与它们在内核里的实际情况一致。
 */
interface TableRow {
  item: ModItemDto;
  depth: number;
}

/**
 * **展开**的父条目 key（默认**收起**，与列表视图同一口径）
 *
 * 内置模组（jar-in-jar）默认不铺开：一个包常带好几个内置库，全展开会把表撑得很长，
 * 想看的模组被挤到屏幕外。点名字前的箭头才展开。
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

/**
 * 把要显示的行拍平（父行 + **已展开**的内置模组行）
 *
 * 排序按 modid 正序（见 types.ts 的 `sortedMods`；与列表视图共用，顺序必须一致）。
 * 只有被展开的父条目才递归下去 —— 默认收起。
 */
function flatten(items: ModItemDto[], depth: number, out: TableRow[]) {
  for (const item of sortedMods(items)) {
    out.push({ item, depth });
    const key = modRowKey(item);
    if (item.jarInJar.length && isOpen(key)) flatten(item.jarInJar, depth + 1, out);
  }
}

/** 本表要显示的行 */
const rows = computed(() => {
  const out: TableRow[] = [];
  flatten(props.items, props.depth, out);
  return out;
});

/**
 * 加载器列的显示名
 *
 * **可能不止一个**：一个 jar 里可以有多份元数据（同时带 `fabric.mod.json` 与
 * `META-INF/mods.toml` 的多加载器包），后端已汇总去重，这里逐个翻译后连起来。
 */
function loaderNames(v: string[]): string {
  return v.map((name) => t(`resource.loader.${name}`)).join(" / ");
}

/** 加载侧 / 下载源的显示名（后端出稳定枚举名，文案在前端翻） */
function sideName(v: string): string {
  return v ? t(`resource.side.${v}`) : "";
}
function sourceName(v: string): string {
  return v ? t(`resource.source.${v}`) : "";
}
</script>

<template>
  <div class="mod-table">
    <div class="mod-tr mod-th" :class="{ 'mod-th-hidden': hideHeader }">
      <span class="mod-th-cell mod-th-select"></span>
      <span class="mod-th-cell mod-th-group">{{ t("resource.modName") }}</span>
      <span class="mod-th-cell">{{ t("resource.colEnable") }}</span>
      <span class="mod-th-cell">{{ t("resource.modNote") }}</span>
      <span class="mod-th-cell">{{ t("resource.colModId") }}</span>
      <span class="mod-th-cell">{{ t("resource.modName") }}</span>
      <span class="mod-th-cell">{{ t("resource.modVersion") }}</span>
      <span class="mod-th-cell">{{ t("resource.colLoader") }}</span>
      <span class="mod-th-cell">{{ t("resource.colSide") }}</span>
      <span class="mod-th-cell">{{ t("resource.colSource") }}</span>
      <span class="mod-th-cell">{{ t("resource.colProjectId") }}</span>
      <span class="mod-th-cell">{{ t("resource.colFileId") }}</span>
      <span class="mod-th-cell">{{ t("resource.colPath") }}</span>
      <span class="mod-th-cell">{{ t("resource.modAuthor") }}</span>
      <span class="mod-th-cell">{{ t("resource.colUrl") }}</span>
      <span class="mod-th-cell mod-th-actions">{{ t("resource.modActions") }}</span>
    </div>

    <div
      v-for="row in rows"
      :key="modRowKey(row.item)"
      class="mod-tr"
      :class="{
        'mod-dragging': !!row.item.sha1 && draggingKey === row.item.sha1,
        'mod-selected': !!row.item.sha1 && selectedKeys.has(row.item.sha1),
        'mod-selecting': selectedKeys.size > 0,
        'mod-nested': row.depth > 1,
      }"
      :style="{ '--row-depth': row.depth - 1 }"
      @pointerdown="row.depth === 1 ? emit('drag-start', { event: $event, item: row.item }) : undefined"
      @contextmenu.prevent="row.depth === 1 ? emit('select', row.item) : undefined"
    >
      <!-- 多选勾选框（与列表视图同一套）：内置行没有 SHA1，留等宽占位保持各列对齐 -->
      <span class="mod-cell mod-select-cell">
        <input
          v-if="row.depth === 1 && row.item.sha1"
          type="checkbox"
          class="mod-select"
          :checked="selectedKeys.has(row.item.sha1)"
          @pointerdown.stop
          @change.stop="emit('select', row.item)"
        />
        <span v-else class="mod-select-gap" />
      </span>
      <!-- 第一列：分组名（只在父行上显示，带折叠箭头）。
           箭头与名字**都能点开 / 收起**：和分组头那边同一个口径，
           别让用户猜"到底点哪儿才算数" -->
      <span class="mod-cell mod-group-cell">
        <template v-if="row.depth === 1">
          <button
            v-if="groupLabel"
            class="mod-tree-caret"
            :class="{ collapsed: !groupOpen }"
            @click.stop="emit('toggle-group')"
          >
            <GlyphIcon name="chevron-down" :size="13" :weight="2.4" />
          </button>
          <span v-else class="mod-tree-caret-placeholder" />
          <!--
            分组名那一格：点 = 折叠 / 展开，按住拖 = 调整分组顺序。
            `.stop` 是必须的 —— 这一格在数据行（`.mod-tr`）**里面**，
            而那一行自己也监听 pointerdown（拖模组归组）。不拦住的话一次按下会
            同时起两个拖拽手势（改分组顺序 + 把模组拖进某个分组）。
          -->
          <span
            class="mod-group-label"
            :title="groupLabel"
            @pointerdown.stop="row.depth === 1 ? emit('drag-group', $event) : undefined"
            @click.stop="emit('toggle-group')"
          >{{ groupLabel }}</span>
        </template>
      </span>

      <!-- 启用：复选框（勾 = 启用，取消 = 禁用）；内置模组没有独立文件，不给 -->
      <span class="mod-cell mod-enable-cell" @pointerdown.stop>
        <input
          v-if="row.depth === 1"
          type="checkbox"
          class="mod-check"
          :checked="!row.item.disable"
          :disabled="busy || row.item.fail"
          @change="emit('toggle', row.item)"
        />
      </span>

      <span class="mod-cell" :title="row.item.note">{{ row.item.note }}</span>
      <span class="mod-cell" :title="row.item.modId">{{ row.item.modId }}</span>
      <span class="mod-cell mod-name-cell" :class="{ 'mod-indent': row.depth > 1 }">
        <!-- 展开箭头：只有带内置模组的行才有；默认收起（与列表视图同一口径）。
             放在名字**前面**，与分组那一列的箭头排在一起也是一列 -->
        <button
          v-if="row.item.jarInJar.length"
          class="mod-tree-caret"
          :class="{ collapsed: !isOpen(modRowKey(row.item)) }"
          v-tip="
            isOpen(modRowKey(row.item))
              ? t('resource.modBuiltinCollapse')
              : t('resource.modBuiltinExpand')
          "
          @click.stop="toggleExpand(modRowKey(row.item))"
          @pointerdown.stop
        >
          <GlyphIcon name="chevron-down" :size="13" :weight="2.4" />
        </button>
        <span
          v-else-if="row.depth > 1 || groupLabel"
          class="mod-tree-caret-placeholder"
        />
        <span class="mod-name" :title="row.item.name || row.item.file">
          {{ row.item.name || row.item.file }}
        </span>
        <!-- 内置的**库**（没有模组元数据的内置 jar）：不是模组，与"内置"分开标 -->
        <span v-if="row.item.library" class="badge badge-dim">
          {{ t("resource.modLibrary") }}
        </span>
        <span v-else-if="row.depth > 1" class="badge badge-dim">
          {{ t("resource.modBuiltin") }}
        </span>
        <span v-if="row.item.jarInJar.length" class="badge">
          {{ t("resource.modBuiltinCount", { n: row.item.jarInJar.length }) }}
        </span>
        <span v-if="row.item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
        <span v-if="row.item.core" class="badge">{{ t("resource.modCore") }}</span>
      </span>
      <span class="mod-cell" :title="row.item.version">{{ row.item.version }}</span>
      <span class="mod-cell" :title="loaderNames(row.item.loaders)">
        {{ loaderNames(row.item.loaders) }}
      </span>
      <span class="mod-cell">{{ sideName(row.item.side) }}</span>
      <span class="mod-cell">{{ sourceName(row.item.source) }}</span>
      <span class="mod-cell" :title="row.item.projectId">{{ row.item.projectId }}</span>
      <span class="mod-cell" :title="row.item.fileId">{{ row.item.fileId }}</span>
      <!-- 路径不挂 title：用户明确不要悬浮 -->
      <span class="mod-cell mod-path">{{ row.item.path }}</span>
      <span class="mod-cell" :title="row.item.author">{{ row.item.author }}</span>
      <span class="mod-cell" :title="row.item.url">{{ row.item.url }}</span>

      <span class="mod-cell mod-actions" @pointerdown.stop>
        <button
          v-if="row.depth === 1"
          class="mini-btn"
          :class="{ on: !!row.item.note }"
          :disabled="busy"
          @click.stop="emit('note', row.item)"
        >
          {{ t("resource.modNote") }}
        </button>
        <button
          v-if="row.depth === 1"
          class="mini-btn"
          :disabled="busy"
          @click.stop="emit('open-folder', row.item)"
        >
          {{ t("resource.openFolder") }}
        </button>
        <button
          v-if="row.depth === 1"
          class="mini-btn danger"
          :disabled="busy"
          @click.stop="emit('remove', row.item)"
        >
          {{ t("resource.delete") }}
        </button>
      </span>
    </div>

    <!--
      空分组：**也要有一行分组名**。
      分组名那一格在数据行里，没有数据行就没有它 —— 于是空分组连名字都没有，
      既不能折叠、也**不能拖动排序**（用户要求"所有分组都能拖动"）。
      多选列留等宽占位，提示文字跨掉剩下的列。
    -->
    <div v-if="!rows.length" class="mod-tr mod-tr-empty">
      <span class="mod-cell mod-select-cell" />
      <span class="mod-cell mod-group-cell">
        <button
          v-if="groupLabel"
          class="mod-tree-caret"
          :class="{ collapsed: !groupOpen }"
          @click.stop="emit('toggle-group')"
        >
          <GlyphIcon name="chevron-down" :size="13" :weight="2.4" />
        </button>
        <span
          v-if="groupLabel"
          class="mod-group-label"
          :title="groupLabel"
          @pointerdown.stop="emit('drag-group', $event)"
          @click.stop="emit('toggle-group')"
        >{{ groupLabel }}</span>
      </span>
      <!-- 与列表视图的空分组用同一句话，两个视图别各说各的 -->
      <span class="mod-cell mod-empty-tip">{{ t("resource.groupEmptyHint") }}</span>
    </div>
  </div>
</template>
