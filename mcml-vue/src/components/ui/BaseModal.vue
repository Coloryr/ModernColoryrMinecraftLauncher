<script setup lang="ts">
// 统一弹窗：标题 + 内容插槽 + 关闭
withDefaults(defineProps<{ title?: string; width?: number; closable?: boolean }>(), {
  title: "",
  width: 420,
  closable: true,
});

const emit = defineEmits<{ (e: "close"): void }>();
</script>

<template>
  <Teleport to="body">
    <div class="modal-mask" @click.self="emit('close')">
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

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 18px;
}
</style>
