// 单窗口模式：回到本窗口时重拉一次数据
//
// 单窗口模式下 App.vue 用 <KeepAlive> 缓存窗口组件（见 App.vue），切走只是 deactivate、
// 切回**不会重新挂载** —— 于是 onMounted 里拉的那份数据永远是第一次打开时的快照
// （统计 / 下载 / 资源 / 收藏 / 账户这类窗口尤其明显）。
//
// 首次挂载不触发：onActivated 在挂载时也会跑一次，那一次交给 onMounted，免得拉两遍。
//
// 用法（窗口根组件里一行）：
// ```ts
// useWindowRefresh(load);
// ```
import { onActivated } from "vue";

export function useWindowRefresh(refresh: () => void | Promise<void>) {
  let mounted = false;
  onActivated(() => {
    if (!mounted) {
      mounted = true;
      return;
    }
    void refresh();
  });
}
