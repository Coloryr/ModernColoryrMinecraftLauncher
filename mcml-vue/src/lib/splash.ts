// 启动画面控制器
//
// 主窗口在初始化完成（或失败）后调用 closeSplash() 关闭启动页；
// 接入真实后端时，也可由后端初始化流程直接调用本函数。
import { ref } from "vue";

/** 启动画面是否显示（主窗口读取） */
export const splashVisible = ref(true);

/** 关闭启动画面：初始化完成后调用 */
export function closeSplash() {
  splashVisible.value = false;
}
