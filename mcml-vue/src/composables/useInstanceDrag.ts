// 自定义拖拽逻辑：实例排序 / 跨组移动 / 分组排序
// 不用 HTML5 draggable（WebView2 中会出现“禁止”光标且投放不可靠），
// 改为指针事件模拟：按下记录候选 → 移动超阈值开始拖拽 →
// 组内插入占位（淡化的实例）→ 松开提交；支持同组排序与跨组移动。
import { computed, onMounted, onUnmounted, ref, type Ref } from "vue";
import { api } from "../lib/api";
import { t } from "../lib/i18n";
import type { InstanceInfo } from "../lib/types";

export interface GroupView {
  name: string;
  items: InstanceInfo[];
}

interface DragDeps {
  multiSelect: Ref<boolean>;
  groups: Ref<GroupView[]>;
  collapsedGroups: Ref<Record<string, boolean>>;
  loadInstances: () => Promise<void> | void;
  loadGroups: () => Promise<void> | void;
}

interface DragCandidate {
  kind: "instance" | "group";
  instance?: InstanceInfo;
  groupName?: string;
}

export function useInstanceDrag(deps: DragDeps) {
  const dragCandidate = ref<DragCandidate | null>(null);
  const dragActive = ref<DragCandidate | null>(null);
  /** 正在拖拽的实例 uuid（拖拽时该行不渲染，只显示插入占位） */
  const draggingUuid = computed(() =>
    dragActive.value?.kind === "instance"
      ? (dragActive.value.instance?.uuid ?? null)
      : null,
  );
  /** 实例拖拽插入位置（组名 + 组内下标） */
  const dragInsert = ref<{ group: string; index: number } | null>(null);
  /** 分组拖拽插入位置 */
  const dragGroupInsert = ref<{ index: number } | null>(null);

  let dragPointerId: number | null = null;
  let dragStartX = 0;
  let dragStartY = 0;
  /** 拖拽结束后抑制紧随的 click（避免误触选择 / 折叠） */
  let suppressClick = false;

  function onDragPointerDown(e: PointerEvent, cand: DragCandidate) {
    if (e.button !== 0 || deps.multiSelect.value) return;
    suppressClick = false;
    dragPointerId = e.pointerId;
    dragStartX = e.clientX;
    dragStartY = e.clientY;
    dragCandidate.value = cand;
  }

  function onDragPointerMove(e: PointerEvent) {
    if (dragPointerId === null || e.pointerId !== dragPointerId) return;
    if (!dragActive.value) {
      const d = Math.hypot(e.clientX - dragStartX, e.clientY - dragStartY);
      if (d > 5) {
        dragActive.value = dragCandidate.value;
        suppressClick = true;
        document.body.classList.add("drag-active");
      }
    }
    if (!dragActive.value) return;
    updateDragInsert(e);
  }

  function onDragPointerUp(e: PointerEvent) {
    if (dragPointerId === null || e.pointerId !== dragPointerId) return;
    if (dragActive.value) commitDrag(dragActive.value);
    resetDrag();
  }

  function resetDrag() {
    dragPointerId = null;
    dragCandidate.value = null;
    dragActive.value = null;
    dragInsert.value = null;
    dragGroupInsert.value = null;
    document.body.classList.remove("drag-active");
  }

  function updateDragInsert(e: PointerEvent) {
    const active = dragActive.value;
    if (!active) return;
    const el = document.elementFromPoint(e.clientX, e.clientY);
    // 靠近分组列表边缘自动滚动，方便投放到远处的分组
    const listEl = el?.closest?.(".group-list") as HTMLElement | null;
    if (listEl) {
      const rect = listEl.getBoundingClientRect();
      const threshold = 40;
      if (e.clientY < rect.top + threshold) listEl.scrollTop -= 10;
      else if (e.clientY > rect.bottom - threshold) listEl.scrollTop += 10;
    }
    if (active.kind === "instance") {
      const block = (el?.closest?.(".group-block") ?? null) as HTMLElement | null;
      if (!block) {
        dragInsert.value = null;
        return;
      }
      const groupName = block.dataset.group ?? "";
      // 插入位置按组内实例计算（排除正在拖拽的实例本身）
      const rows = [...block.querySelectorAll(".inst-row")].filter(
        (r) => (r as HTMLElement).dataset.uuid !== active.instance?.uuid,
      ) as HTMLElement[];
      let index = rows.length;
      for (let i = 0; i < rows.length; i++) {
        const r = rows[i].getBoundingClientRect();
        if (e.clientY < r.top + r.height / 2) {
          index = i;
          break;
        }
      }
      dragInsert.value = { group: groupName, index };
    } else {
      const list = el?.closest?.(".group-list") ?? null;
      const blocks = list ? [...list.querySelectorAll(".group-block")] : [];
      let index = blocks.length;
      for (let i = 0; i < blocks.length; i++) {
        const r = (blocks[i] as HTMLElement).getBoundingClientRect();
        if (e.clientY < r.top + r.height / 2) {
          index = i;
          break;
        }
      }
      dragGroupInsert.value = { index };
    }
  }

  async function commitDrag(active: DragCandidate) {
    if (active.kind === "instance" && active.instance) {
      const ins = dragInsert.value;
      if (!ins) return;
      const target = ins.group === t("group.default") ? null : ins.group;
      await api.moveInstance(active.instance.uuid, target, ins.index);
      await deps.loadInstances();
      // 展开目标分组
      const key = target || t("group.default");
      deps.collapsedGroups.value = { ...deps.collapsedGroups.value, [key]: false };
    } else if (active.kind === "group" && active.groupName) {
      const ins = dragGroupInsert.value;
      if (!ins) return;
      await api.moveGroup(active.groupName, ins.index);
      await deps.loadGroups();
    }
  }

  /** 实例插入占位：分组 groupName 中第 idx 个实例之前（末尾由 isInstInsertEnd 渲染，避免同组拖动出现双占位） */
  function isInstInsert(groupName: string, idx: number): boolean {
    const ins = dragInsert.value;
    if (!ins || ins.group !== groupName) return false;
    const active = dragActive.value;
    if (active?.kind !== "instance") return false;
    const dragging = active.instance;
    if (!dragging) return false;
    const items = deps.groups.value.find((g) => g.name === groupName)?.items ?? [];
    // 组内非拖拽实例总数
    const total = items.filter((i) => i.uuid !== dragging.uuid).length;
    let before = 0;
    for (let i = 0; i < idx; i++) {
      if (items[i].uuid !== dragging.uuid) before++;
    }
    return before === ins.index && ins.index < total;
  }

  /** 实例插入占位：分组末尾 */
  function isInstInsertEnd(groupName: string): boolean {
    const ins = dragInsert.value;
    if (!ins || ins.group !== groupName) return false;
    const active = dragActive.value;
    if (active?.kind !== "instance") return false;
    const dragging = active.instance;
    if (!dragging) return false;
    const items = deps.groups.value.find((g) => g.name === groupName)?.items ?? [];
    const before = items.filter((i) => i.uuid !== dragging.uuid).length;
    return before === ins.index;
  }

  /** 分组插入占位：分组列表第 index 个之前 */
  function isGroupInsert(index: number): boolean {
    return (
      dragActive.value?.kind === "group" &&
      !!dragGroupInsert.value &&
      dragGroupInsert.value.index === index
    );
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
    window.addEventListener("pointermove", onDragPointerMove);
    window.addEventListener("pointerup", onDragPointerUp);
  });
  onUnmounted(() => {
    window.removeEventListener("pointermove", onDragPointerMove);
    window.removeEventListener("pointerup", onDragPointerUp);
  });

  return {
    dragActive,
    draggingUuid,
    dragInsert,
    dragGroupInsert,
    onDragPointerDown,
    isInstInsert,
    isInstInsertEnd,
    isGroupInsert,
    consumeSuppressClick,
  };
}
