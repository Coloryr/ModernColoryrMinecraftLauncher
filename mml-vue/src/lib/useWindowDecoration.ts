// 窗口装饰生命周期：激活插件装饰 + 注册贴靠热区
//
// 每个窗口（主窗口与全部功能窗口）都要做这两件事：
//   1. 把贴靠热区（自己的最大化按钮矩形）交给插件 —— 插件用它摆原生命中区；
//   2. 调 window_activate_decoration 激活自绘装饰并显示窗口。
//
// 为什么必须"先注册再激活"：插件在激活流程里就要量那个矩形，量不到会以
// kind="disabled" 完成这次激活（原生 frame 不摘、系统标题栏与自绘按钮并存），
// 之后再补报也改不回已完成的那次。而窗口是按 `visible(false)` 建出来的，
// 激活同时负责显示 —— 所以顺序错了窗口表现会很怪。
//
// 启动时机：窗口建出来时是隐藏的，前端挂载后才显示。功能窗口没有启动画面，
// 标题栏随组件一起渲染，所以这里直接 nextTick 后量即可（主窗口走 MainWindow
// 自己的路径，它要等启动画面切走）。
import { nextTick, onMounted, onUnmounted, ref, type Ref } from "vue";
import { commands } from "./bindings";
import { isTauri } from "../windows/windowManager";
import { registerSnapTarget, waitForSnapTarget } from "./decoration";

/**
 * 给当前窗口接上插件装饰
 *
 * - `rootEl`: 该窗口的根元素（贴靠热区在它范围内查找最大化按钮）
 */
export function useWindowDecoration(rootEl: Ref<HTMLElement | null>) {
  /** 装饰是否已由插件接管（macOS 下插件用原生红黄绿，前端不该再画圆点） */
  const decorated = ref(false);
  let unregister: (() => void) | null = null;

  onMounted(async () => {
    if (!isTauri()) return;

    const root = rootEl.value;
    unregister = registerSnapTarget(root);

    // 等一帧让标题栏（含窗口按钮）渲染出来，否则量不到矩形
    await nextTick();
    await waitForSnapTarget(root);

    try {
      const mode = await commands.windows.activateDecoration();
      if (mode === "custom") {
        decorated.value = true;
      } else {
        console.warn("[decoration] 退回原生 frame:", mode);
      }
    } catch (e) {
      console.warn("[decoration] 激活失败:", e);
    }
  });

  onUnmounted(() => {
    unregister?.();
    unregister = null;
  });

  return { decorated };
}
