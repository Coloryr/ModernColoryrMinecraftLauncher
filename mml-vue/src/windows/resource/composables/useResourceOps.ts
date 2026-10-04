// 资源管理窗口的共用操作：忙碌标记 / 确认弹窗 / 打开目录 / 操作后重拉
//
// 各分类的增删改在各自的 parts/*Pane.vue 里（它们知道该调哪个命令），
// 这里只放"所有分类都要走一遍"的那几件：串行化、失败提示、确认、刷新列表、
// 以及"打开目录不重拉列表"这条容易写错的边界（见 openFolder）。
import { ref } from "vue";
import { t, tErr } from "../../../lib/i18n";
import { showToast } from "../../../lib/toast";
import { openResourceFolder } from "../../../lib/api";
import type { useResourceData } from "./useResourceData";

/** 确认弹窗的内容：标题 + 正文 + 确认后要做的事（reload = 做完是否重拉列表） */
interface ConfirmBox {
  title: string;
  text: string;
  run: () => Promise<void>;
  reload: boolean;
}

export function useResourceOps(data: ReturnType<typeof useResourceData>) {
  const { instanceUuid, reloadCurrent } = data;

  /** 有操作在跑：列表上的按钮统一置灰，避免连点发两次 */
  const busy = ref(false);

  const confirmBox = ref<ConfirmBox | null>(null);
  const confirmBusy = ref(false);

  function askConfirm(
    title: string,
    text: string,
    run: () => Promise<void>,
    /** 确认后要不要重拉当前列表；分组这类**不在列表里**的数据传 false（重拉模组要重解析 jar） */
    opts?: { reload?: boolean },
  ) {
    confirmBox.value = { title, text, run, reload: opts?.reload ?? true };
  }

  /** 删除类确认：文案统一（"确定删除 X 吗"），确认后由 runConfirm 负责重拉 */
  function askDelete(name: string, run: () => Promise<void>) {
    askConfirm(t("resource.delete"), t("resource.deleteConfirm", { name }), run);
  }

  async function runConfirm() {
    const box = confirmBox.value;
    if (!box || confirmBusy.value) return;
    confirmBusy.value = true;
    try {
      await box.run();
      confirmBox.value = null;
      // 动作改的是磁盘上的列表：不重拉的话，删掉的那一行还留在界面上
      // （分组数据不在列表里，askConfirm 会明确传 reload: false）
      if (box.reload) await reloadCurrent();
    } catch (e) {
      showToast(tErr(e));
    } finally {
      confirmBusy.value = false;
    }
  }

  /** 轻操作（启用 / 禁用 / 备份 / 切换光影…）：串行化 + 失败提示 + 成功后重拉 */
  async function act(run: () => Promise<void>) {
    if (busy.value) return;
    busy.value = true;
    try {
      await run();
      await reloadCurrent();
    } catch (e) {
      showToast(tErr(e));
    } finally {
      busy.value = false;
    }
  }

  /**
   * 在文件管理器里打开目录
   *
   * **不重拉列表**：这是只读操作，列表不会变；而重拉一次模组要重新解析每个 jar 的元数据
   * （数秒），原先它和增删走同一条 act()，点一下"打开文件夹"整个列表就白转一圈。
   */
  async function openFolder(kind: string, name: string | null, parent: string | null = null) {
    if (busy.value) return;
    busy.value = true;
    try {
      await openResourceFolder(instanceUuid.value, kind, name, parent);
    } catch (e) {
      showToast(tErr(e));
    } finally {
      busy.value = false;
    }
  }

  return {
    busy,
    confirmBox,
    confirmBusy,
    askConfirm,
    askDelete,
    runConfirm,
    act,
    openFolder,
  };
}
