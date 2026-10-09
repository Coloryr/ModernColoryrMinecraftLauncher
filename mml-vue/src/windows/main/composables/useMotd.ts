// 主窗口的两处 MOTD 卡片：启动器下方的悬浮卡（客户端设置里配置的全局地址）与实例详情卡
// （查实例自己配置的服务器地址）。两者都走内核的 Server List Ping。
//
// 卡片外观（图标 / 彩色分段 / 人数-版本-延迟）在 components/MotdCard.vue —— 本窗口的悬浮卡、
// 实例详情卡、资源窗口服务器列表上方那张，三处共用同一份；这里只管**地址来源、竞态保护与定时刷新**。
import { onUnmounted, ref, watch, type Ref } from "vue";

import { api } from "../../../lib/api";
import type { ClientConfig } from "../../../lib/guiConfig";
import type { InstanceArgsDto, InstanceInfoDto, MotdDto } from "../../../lib/bindings";

/**
 * - `clientConfig`：客户端设置（MOTD 显示开关 / 地址 / 自动刷新间隔）
 * - `selected`：当前选中实例（实例详情卡的地址来源）
 * - `argsOf`：取实例启动参数（读其中的服务器地址与端口）
 */
export function useMotd(opts: {
  clientConfig: Ref<ClientConfig>;
  selected: Ref<InstanceInfoDto | null>;
  argsOf: (uuid: string) => InstanceArgsDto;
}) {
  const { clientConfig, selected, argsOf } = opts;

  // ---- 悬浮卡：查客户端设置里的全局地址 ----

  const motdInfo = ref<MotdDto | null>(null);
  const motdLoading = ref(false);
  // 竞态保护：旧请求晚到不覆盖新结果
  let motdSeq = 0;

  async function refreshMotd() {
    const addr = clientConfig.value.motdServer.trim();
    if (!addr || motdLoading.value) return;
    motdLoading.value = true;
    const seq = ++motdSeq;
    try {
      const dto = await api.getMotd(addr);
      if (seq === motdSeq) motdInfo.value = dto;
    } catch {
      if (seq === motdSeq) motdInfo.value = null;
    } finally {
      if (seq === motdSeq) motdLoading.value = false;
    }
  }

  // ---- 实例详情卡：查实例自己配置的服务器地址 ----

  const instMotd = ref<MotdDto | null>(null);
  const instMotdLoading = ref(false);
  let instMotdSeq = 0;

  async function refreshInstMotd() {
    const inst = selected.value;
    const ip = inst ? argsOf(inst.uuid).serverIp.trim() : "";
    if (!ip) {
      instMotd.value = null;
      return;
    }
    if (instMotdLoading.value) return;
    const port = inst ? argsOf(inst.uuid).serverPort || 25565 : 25565;
    instMotdLoading.value = true;
    const seq = ++instMotdSeq;
    try {
      const dto = await api.getMotd(`${ip}:${port}`);
      if (seq === instMotdSeq) instMotd.value = dto;
    } catch {
      if (seq === instMotdSeq) instMotd.value = null;
    } finally {
      if (seq === instMotdSeq) instMotdLoading.value = false;
    }
  }

  // 切换实例或改实例服务器地址后重新查询
  watch(
    () => {
      const inst = selected.value;
      if (!inst) return "";
      const a = argsOf(inst.uuid);
      return `${inst.uuid}|${a.serverIp}|${a.serverPort}`;
    },
    () => void refreshInstMotd(),
  );

  // ---- 悬浮卡的显隐与自动刷新定时器（由客户端设置驱动）----

  const motdCardVisible = ref(true);
  let motdTimer: number | null = null;

  function restartMotdTimer() {
    if (motdTimer !== null) clearInterval(motdTimer);
    motdTimer = null;
    if (!motdCardVisible.value) return;
    const sec = clientConfig.value.motdInterval;
    if (sec >= 5) {
      motdTimer = window.setInterval(refreshMotd, sec * 1000);
    }
  }

  // 定时器随组件卸载一起清掉（原来写在主窗口的 onUnmounted 里）
  onUnmounted(() => {
    if (motdTimer !== null) clearInterval(motdTimer);
  });

  return {
    motdInfo,
    motdLoading,
    instMotd,
    instMotdLoading,
    motdCardVisible,
    refreshMotd,
    refreshInstMotd,
    restartMotdTimer,
  };
}
