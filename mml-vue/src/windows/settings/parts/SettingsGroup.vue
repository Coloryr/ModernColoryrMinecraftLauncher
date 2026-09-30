<script setup lang="ts">
// 设置分组：标题（带强调色竖条）+ 可选「恢复默认」按钮 + 搜索锚点
//
// 每个设置分组都包一层它，带来三件事：
// 1. `#set-<id>` 锚点 —— 设置项搜索命中后跳到这里；
// 2. 分组级「恢复默认」—— 按钮只在 `resettable` 时出现，点击把 id 交回窗口派发；
// 3. `flash` 脉冲 —— 恢复默认 / 搜索命中后短暂高亮，让用户看清改了哪一块。
import { t } from "../../../lib/i18n";

defineProps<{
  /** 分组 id（与搜索索引、恢复默认的派发键一致） */
  id: string;
  /** 标题文案键；不传则不渲染标题（用于整页只有一个隐式分组的标签） */
  titleKey?: string;
  /** 该分组支持恢复默认 */
  resettable?: boolean;
  /** 高亮脉冲 */
  flash?: boolean;
}>();

const emit = defineEmits<{ (e: "reset", id: string): void }>();
</script>

<template>
  <section :id="`set-${id}`" class="setting-group" :class="{ flash }">
    <h3 v-if="titleKey" class="group-title">
      {{ t(titleKey) }}
      <button
        v-if="resettable"
        type="button"
        class="group-reset"
        v-tip="t('winSettings.resetGroupHint')"
        @click="emit('reset', id)"
      >
        {{ t("winSettings.resetGroup") }}
      </button>
    </h3>
    <slot />
    <!-- 没有标题的分组（如皮肤页）：把按钮放在内容下方，右对齐 -->
    <div v-if="!titleKey && resettable" class="group-reset-row">
      <button
        type="button"
        class="group-reset static"
        v-tip="t('winSettings.resetGroupHint')"
        @click="emit('reset', id)"
      >
        {{ t("winSettings.resetGroup") }}
      </button>
    </div>
  </section>
</template>
