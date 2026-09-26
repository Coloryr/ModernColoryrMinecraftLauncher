<script setup lang="ts">
// 统一弹窗：标题 + 内容插槽 + 关闭
const props = withDefaults(
  defineProps<{
    title?: string;
    width?: number;
    closable?: boolean;
    /** 遮罩从自绘标题栏下方开始，让标题栏（可拖动 / 窗口按钮）保持可操作 */
    belowTitlebar?: boolean;
    /** 点击遮罩空白处是否关闭（输入类弹窗建议关掉，避免误触丢内容） */
    overlayClose?: boolean;
  }>(),
  {
    title: "",
    width: 420,
    closable: true,
    belowTitlebar: false,
    overlayClose: true,
  },
);

const emit = defineEmits<{ (e: "close"): void }>();
</script>

<template>
  <Teleport to="body">
    <div
      class="modal-mask"
      :class="{ 'below-titlebar': props.belowTitlebar }"
      @click.self="props.overlayClose && emit('close')"
    >
      <div class="modal" :style="{ width: width + 'px' }">
        <div v-if="title" class="modal-head">
          <h3>{{ title }}</h3>
          <button v-if="closable" class="modal-x" @click="emit('close')">✕</button>
        </div>
        <div class="modal-body">
          <slot />
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-mask {
  position: fixed;
  inset: 0;
  background: var(--overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

/* 避开自绘标题栏：遮罩从标题栏下沿开始，标题栏仍可拖动 / 点窗口按钮 */
.modal-mask.below-titlebar {
  top: var(--titlebar-h);
}

.modal {
  max-width: 92vw;
  max-height: 85vh;
  overflow-y: auto;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 14px;
  padding: 22px 24px;
  box-shadow: var(--shadow-lg);
}

.modal-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.modal-head h3 {
  font-size: 17px;
}

.modal-x {
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-size: 14px;
  cursor: pointer;
  padding: 4px 6px;
  border-radius: 6px;
  line-height: 1;
}

.modal-x:hover {
  background: var(--bg-hover);
  color: var(--text);
}
</style>
