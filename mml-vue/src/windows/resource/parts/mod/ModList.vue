<script setup lang="ts">
// 模组视图之一：列表（每行 图标 + 名称/徽标 + 副标题 + 操作）—— 与其它分类的行同一观感
import { t } from "../../../../lib/i18n";
import ResourceRow from "../ResourceRow.vue";
import { modRowKey, modSub, type ModViewEmits, type ModViewProps } from "../../types";

defineProps<ModViewProps>();
const emit = defineEmits<ModViewEmits>();
</script>

<template>
  <ResourceRow
    v-for="item in items"
    :key="modRowKey(item)"
    :icon="item.icon"
    letter="M"
    :name="item.name || item.file"
    :class="{ 'mod-dragging': !!item.sha1 && draggingKey === item.sha1 }"
    @pointerdown="emit('drag-start', { event: $event, item })"
  >
    <template #badges>
      <span v-if="item.disable" class="badge badge-dim">{{ t("resource.modDisabled") }}</span>
      <span v-if="item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
      <span v-if="item.core" class="badge">{{ t("resource.modCore") }}</span>
      <span v-if="item.jarInJar.length" class="badge">{{ t("resource.modBuiltin") }}</span>
    </template>
    <template #sub>{{ modSub(item) }}</template>
    <template #actions>
      <!-- 按钮上不许起拖拽（否则按住按钮移动会把模组拖进别的分组） -->
      <span class="acts-stop" @pointerdown.stop>
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
    </template>
  </ResourceRow>
</template>
