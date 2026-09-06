// 拖拽整合包文件到窗口的逻辑：显示遮罩层，松开后按扩展名创建实例
import { onMounted, onUnmounted, ref, type Ref } from "vue";
import { api } from "../lib/api";
import { t } from "../lib/i18n";
import { showToast } from "../lib/toast";
import type { VersionInfo } from "../lib/types";

interface FileDropDeps {
  versions: Ref<VersionInfo[]>;
  loadInstances: () => Promise<void> | void;
}

export function useFileDrop(deps: FileDropDeps) {
  const fileDragOver = ref(false);
  let fileDragDepth = 0;

  function hasFiles(e: DragEvent): boolean {
    return !!e.dataTransfer?.types?.includes("Files");
  }

  function onWindowDragEnter(e: DragEvent) {
    if (!hasFiles(e)) return;
    fileDragDepth++;
    fileDragOver.value = true;
  }

  function onWindowDragOver(e: DragEvent) {
    if (hasFiles(e)) e.preventDefault(); // 允许放置
  }

  function onWindowDragLeave(e: DragEvent) {
    if (!hasFiles(e)) return;
    fileDragDepth = Math.max(0, fileDragDepth - 1);
    if (fileDragDepth === 0) fileDragOver.value = false;
  }

  /** 松开：读取整合包文件并添加为实例 */
  async function onWindowDrop(e: DragEvent) {
    if (!hasFiles(e)) return;
    e.preventDefault();
    fileDragDepth = 0;
    fileDragOver.value = false;
    const files = e.dataTransfer?.files ? [...e.dataTransfer.files] : [];
    const packs = files.filter((f) => /\.(zip|mrpack|modpack)$/i.test(f.name));
    if (packs.length === 0) {
      if (files.length > 0) showToast(t("drop.notModpack"));
      return;
    }
    for (const f of packs) {
      const name = f.name.replace(/\.(zip|mrpack|modpack)$/i, "");
      try {
        const inst = await api.createInstance(name, deps.versions.value[0]?.id ?? "1.21.1");
        showToast(t("drop.added", { name: inst.name }));
      } catch (err) {
        showToast(t("add.createFail", { msg: String(err) }));
      }
    }
    await deps.loadInstances();
  }

  onMounted(() => {
    window.addEventListener("dragenter", onWindowDragEnter);
    window.addEventListener("dragover", onWindowDragOver);
    window.addEventListener("dragleave", onWindowDragLeave);
    window.addEventListener("drop", onWindowDrop);
  });
  onUnmounted(() => {
    window.removeEventListener("dragenter", onWindowDragEnter);
    window.removeEventListener("dragover", onWindowDragOver);
    window.removeEventListener("dragleave", onWindowDragLeave);
    window.removeEventListener("drop", onWindowDrop);
  });

  return { fileDragOver };
}
