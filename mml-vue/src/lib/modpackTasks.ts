// 整合包安装任务状态（下载整合包窗口 / 主窗口共用）
//
// 任务在后端全局表维护，与窗口生命周期解耦；本模块只负责把
// add-modpack-status 事件 / 初始查询落到本地 ref，并处理终态副作用：
// - 安装成功 → 写 localStorage 通知主窗口切换选中实例（见 MainWindow 的 storage 监听）
// - 完成 / 失败 → toast；只由当前"负责显示"的窗口弹（整合包窗口开着时主窗口不弹）
import { ref } from "vue";
import { api, onAddModpackStatus } from "./api";
import { t, tErr } from "./i18n";
import { showToast } from "./toast";
import type { ModPackStatusDto } from "./bindings";

/**
 * @param own 是否为下载整合包窗口本体（true = 始终负责 toast；false = 仅在整合包窗口关闭时负责）
 * @param onAddedInstance 安装成功且本窗口负责提示时回调（主窗口用它刷新并选中
 *   新实例——整合包窗口关闭时没人写 storage 事件，只能在本窗口直接通知）
 */
export function useModpackStatus(
  own: boolean,
  onAddedInstance?: (uuid: string) => void,
) {
  const status = ref<ModPackStatusDto | null>(null);
  /** 已处理过终态的任务 uuid（避免重复 toast） */
  const seenDone = new Set<string>();

  function apply(e: ModPackStatusDto, silent: boolean) {
    for (const task of e.tasks) {
      if (!(task.done || task.failed || task.cancelled) || seenDone.has(task.uuid)) {
        continue;
      }
      seenDone.add(task.uuid);
      if (task.instanceUuid) {
        localStorage.setItem("mml.addedInstance", task.instanceUuid);
      }
      // 初次同步不提示（窗口打开前已结束的任务），整合包窗口开着时主窗口不提示
      if (silent || (!own && e.windowOpen)) {
        continue;
      }
      if (task.done) {
        if (task.instanceUuid) {
          onAddedInstance?.(task.instanceUuid);
        }
        showToast(t("modpack.installDone", { name: task.name }));
      } else if (task.failed) {
        showToast(t("add.createFail", { msg: task.error ? tErr(task.error) : "" }));
      }
    }
    status.value = e;
  }

  /** 挂载时调用：先同步一次快照（已有的终态任务不提示），再订阅事件 */
  async function init(): Promise<() => void> {
    try {
      apply(await api.getModpackStatus(), true);
    } catch {
      status.value = null;
    }
    return onAddModpackStatus((e) => apply(e, false));
  }

  return { status, init };
}
