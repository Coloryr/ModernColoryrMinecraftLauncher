// 全局轻提示（所有窗口统一使用）
// 用法：import { showToast } from "./toast"; showToast("xxx");
import { ref } from "vue";

export interface ToastItem {
  id: number;
  msg: string;
}

export const toasts = ref<ToastItem[]>([]);

let nextId = 1;

/** 弹出轻提示，duration 毫秒后自动消失（默认 2200） */
export function showToast(msg: string, duration = 2200) {
  const id = nextId++;
  toasts.value.push({ id, msg });
  setTimeout(() => dismissToast(id), duration);
}

/** 手动关闭某条提示 */
export function dismissToast(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id);
}
