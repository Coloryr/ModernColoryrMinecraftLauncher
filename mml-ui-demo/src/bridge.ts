// 桥接使用小工具：宿主探测、带超时的 invoke、启动阶段文案
//
// 这一层不是桥接的一部分，只是把示例页里容易踩的坑封装好：
// - `window.mml` 只在启动器里存在；
// - 没有宿主时 invoke 永远不会有应答（不 reject 也不 resolve），必须自己加超时；
// - `mml.ready` 在没有宿主时也会 resolve（约 10 秒后），不能当宿主检测用。
//
// 类型都来自 `./mml`（即 `src/mml.d.ts`，纯类型文件、运行时不存在这个模块），
// 所以这里的 import 必须写成 `import type`，绝不能变成值导入。

import type { MmlApi, MmlErrorEvent, MmlExitEvent, MmlLaunchState, MmlTheme } from "./mml";

/** 拿桥接对象（不在启动器里时为 undefined） */
export function getMml(): MmlApi | undefined {
  return window.mml;
}

/**
 * 当前主题（不在启动器里时按系统偏好给个值，方便浏览器预览）
 *
 * 注意：本工程的配色**不靠这个函数**——主题会写到 `<html data-mml-theme>`，
 * 纯 CSS 就能跟随（见 style.css，含系统偏好兜底）。这个函数只是给需要读值的地方
 * （比如 canvas 绘制）用。
 */
export function currentTheme(): MmlTheme {
  const t = getMml()?.theme;
  if (t === "Light" || t === "Dark") return t;
  return window.matchMedia?.("(prefers-color-scheme: light)").matches ? "Light" : "Dark";
}

/** 订阅主题变化（不在启动器里时返回空函数；浏览器预览可自行监听 prefers-color-scheme） */
export function onTheme(cb: (theme: MmlTheme) => void): () => void {
  return getMml()?.onTheme(cb) ?? (() => { });
}

/** 带超时的 invoke 超时抛出的错误，用来区分「没人应答」与「命令自己报错」 */
export class TimeoutError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TimeoutError";
  }
}

/** 给 Promise 加超时（超时抛 TimeoutError） */
export function withTimeout<T>(promise: Promise<T>, ms: number, what: string): Promise<T> {
  return new Promise<T>((resolve, reject) => {
    const timer = setTimeout(() => reject(new TimeoutError(`${what} 超时（${ms}ms 内没有应答）`)), ms);
    promise.then(
      (value) => {
        clearTimeout(timer);
        resolve(value);
      },
      (err: unknown) => {
        clearTimeout(timer);
        reject(err);
      },
    );
  });
}

/** 探测宿主：页面是否真的跑在启动器里 */
export async function probeHost(ms = 1500): Promise<boolean> {
  const mml = getMml();
  if (!mml) return false;
  try {
    // 拿一条最轻的命令当探针：有应答（成功或报错都算）就说明父窗口在。
    // 用 main_load_state 是因为它不碰磁盘也不启动任何东西。
    await withTimeout(mml.invoke("main_load_state"), ms, "握手");
    return true;
  } catch (err) {
    // 超时 = 父窗口没应答；其它错误 = 命令自己失败，但父窗口确实在
    return !(err instanceof TimeoutError);
  }
}

/** 调启动器命令（默认 10 秒超时，避免页面卡在「转圈」上） */
export function invoke<T>(cmd: string, args?: Record<string, unknown>, ms = 10000): Promise<T> {
  const mml = getMml();
  if (!mml) return Promise.reject(new Error("当前不在启动器中打开，window.mml 不可用"));
  return withTimeout(mml.invoke<T>(cmd, args ?? null), ms, `命令 ${cmd}`);
}

/**
 * 启动阶段文案
 *
 * state 取值与启动器一致；表里没有的一律原样显示，方便后端加新阶段时不至于空着。
 */
export const STAGE_LABELS: Record<string, string> = {
  launching: "正在启动",
  login: "登录账户",
  check: "检查游戏文件",
  readinfo: "读取版本信息",
  download: "下载缺失文件",
  jvm: "准备启动参数",
  pre: "执行启动前命令",
  post: "执行启动后命令",
  end: "启动完成",
  loadserverpack: "加载服务器包",
  checkserverpack: "检查服务器包",
  downloadserverpack: "下载服务器包",
};

// ---- 带类型的事件订阅（启动器白名单事件，见 README） ----
// 底层 mml.on 的 payload 是 any，这里给常用事件套一层类型；返回的都是取消订阅函数。

/** 订阅启动阶段（launch-state） */
export function onLaunchState(cb: (e: MmlLaunchState) => void): () => void {
  return getMml()?.on("launch-state", cb) ?? (() => { });
}

/** 订阅游戏退出（game-exit） */
export function onGameExit(cb: (e: MmlExitEvent) => void): () => void {
  return getMml()?.on("game-exit", cb) ?? (() => { });
}

/** 订阅启动失败（launch-error） */
export function onLaunchError(cb: (e: MmlErrorEvent) => void): () => void {
  return getMml()?.on("launch-error", cb) ?? (() => { });
}
