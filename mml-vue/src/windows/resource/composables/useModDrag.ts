// 模组行的拖拽：把模组拖进某个分组（或拖到"未分组"里摘出来）
//
// 与实例 / 收藏那两套同一路数（指针事件模拟，不用 HTML5 draggable：WebView2 里会出现
// "禁止"光标且投放不可靠）：按下记候选 → 移动超阈值才开始拖 → 指针下方找**分组容器**
// （`[data-mod-group]`）→ 松手交给调用方提交。
//
// 与 useCollectDrag 的差别：这里只管"拖的是哪个 key、落在哪个组"，提交（调后端）由调用方
// 的 onDrop 做 —— 三个视图（列表 / 表格 / 树）共用这一份拖拽，落点判定完全一样。
import { onMounted, onUnmounted, ref } from "vue";

/** 开始拖拽的位移阈值（与实例 / 收藏拖拽一致） */
const DRAG_THRESHOLD = 5;

interface ModDragDeps {
  /** 提交：`group` 为 null 表示"未分组"（移出所有组） */
  onDrop: (group: string | null, keys: string[]) => void;
}

export function useModDrag(deps: ModDragDeps) {
  /** 按下但还没超过阈值（可能只是点击） */
  let candidate: string | null = null;
  /** 真正在拖的模组 key */
  const draggingKey = ref<string | null>(null);
  /** 指针当前落在哪个分组上（"" = 未分组；null = 不在任何分组容器上） */
  const dropGroup = ref<string | null>(null);

  let pointerId: number | null = null;
  let startX = 0;
  let startY = 0;
  /** 拖拽结束后抑制紧随的 click（否则松手会顺手触发行的点击） */
  let suppressClick = false;

  function onRowPointerDown(e: PointerEvent, key: string) {
    if (e.button !== 0) return;
    suppressClick = false;
    pointerId = e.pointerId;
    startX = e.clientX;
    startY = e.clientY;
    candidate = key;
    draggingKey.value = null;
    dropGroup.value = null;
  }

  function onPointerMove(e: PointerEvent) {
    if (pointerId === null || e.pointerId !== pointerId || !candidate) return;
    if (!draggingKey.value) {
      if (Math.hypot(e.clientX - startX, e.clientY - startY) <= DRAG_THRESHOLD) return;
      draggingKey.value = candidate;
    }
    updateDropGroup(e);
  }

  /** 指针下方是哪个分组容器（容器上挂 `data-mod-group="<组名|空串>"`） */
  function updateDropGroup(e: PointerEvent) {
    const el = document.elementFromPoint(e.clientX, e.clientY);
    // 靠近内容区上下边缘自动滚动，方便投给远处的分组
    const scroller = (el?.closest?.(".item-list, .mod-table, .mod-tree") ?? null) as HTMLElement | null;
    if (scroller) {
      const rect = scroller.getBoundingClientRect();
      if (e.clientY < rect.top + 60) scroller.scrollTop -= 10;
      else if (e.clientY > rect.bottom - 60) scroller.scrollTop += 10;
    }
    const box = (el?.closest?.("[data-mod-group]") ?? null) as HTMLElement | null;
    dropGroup.value = box ? (box.dataset.modGroup ?? "") : null;
  }

  function onPointerUp(e: PointerEvent) {
    if (pointerId === null || e.pointerId !== pointerId) return;
    const key = draggingKey.value;
    const target = dropGroup.value;
    const moved = key !== null && target !== null;
    reset();
    if (moved && key) {
      suppressClick = true;
      // 空串 = 未分组：传 null 让后端把它从所有组里摘出来
      deps.onDrop(target === "" ? null : target, [key]);
    }
  }

  function reset() {
    pointerId = null;
    candidate = null;
    draggingKey.value = null;
    dropGroup.value = null;
  }

  /** 消费拖拽后的抑制点击标记；返回 true 表示本次 click 应被忽略 */
  function consumeSuppressClick(): boolean {
    if (suppressClick) {
      suppressClick = false;
      return true;
    }
    return false;
  }

  onMounted(() => {
    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp);
    // 指针被系统收走（触摸被打断等）当作取消
    window.addEventListener("pointercancel", reset);
  });
  onUnmounted(() => {
    window.removeEventListener("pointermove", onPointerMove);
    window.removeEventListener("pointerup", onPointerUp);
    window.removeEventListener("pointercancel", reset);
  });

  return { draggingKey, dropGroup, onRowPointerDown, consumeSuppressClick };
}
