<script setup lang="ts">
// 设置窗口 · 设置项搜索
//
// 索引在 ../search.ts（静态表，覆盖 6 个标签）。这里只负责输入与结果列表：
// ↑/↓ 选择、Enter 跳转、点击跳转；Esc 有内容时先清空（不关窗口）。
import { computed, ref, watch } from "vue";
import { t } from "../../../lib/i18n";
import HighlightText from "../../../components/ui/HighlightText.vue";
import { searchSettings, type SettingsHit } from "../search";

const emit = defineEmits<{ (e: "select", hit: SettingsHit): void }>();

const query = ref("");
const open = ref(false);
const active = ref(0);

const hits = computed(() => searchSettings(query.value));

watch(hits, () => {
  active.value = 0;
});

function choose(i: number) {
  const hit = hits.value[i];
  if (!hit) return;
  // 选完清空：跳到目标分组后不该还压着一层下拉
  query.value = "";
  open.value = false;
  emit("select", hit);
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape") {
    if (!query.value) return; // 没内容：交给窗口的 Esc（关窗）
    query.value = "";
    e.stopPropagation(); // 有内容：只清搜索，别把窗口关了
    return;
  }
  if (e.key === "ArrowDown") {
    e.preventDefault();
    active.value = Math.min(hits.value.length - 1, active.value + 1);
  } else if (e.key === "ArrowUp") {
    e.preventDefault();
    active.value = Math.max(0, active.value - 1);
  } else if (e.key === "Enter") {
    e.preventDefault();
    choose(active.value);
  }
}
</script>

<template>
  <div class="set-search">
    <input
      v-model="query"
      class="field-input set-search-input"
      :placeholder="t('winSettings.searchPlaceholder')"
      spellcheck="false"
      autocomplete="off"
      @focus="open = true"
      @input="open = true"
      @blur="open = false"
      @keydown="onKey"
    />
    <div v-if="open && query.trim()" class="set-search-drop">
      <button
        v-for="(h, i) in hits"
        :key="`${h.tab}/${h.group}/${h.label}`"
        type="button"
        class="set-search-item"
        :class="{ on: i === active }"
        @mousedown.prevent
        @click="choose(i)"
      >
        <span class="set-search-label">
          <HighlightText :text="h.label" :query="query" />
        </span>
        <span class="set-search-path">
          {{ h.tabLabel }} › <HighlightText :text="h.groupLabel" :query="query" />
        </span>
      </button>
      <div v-if="!hits.length" class="empty-tip">{{ t("winSettings.searchNone") }}</div>
    </div>
  </div>
</template>
