// window.mml 的类型提示
//
// 这份声明由示例工程自己维护，**不要**依赖启动器仓库里的类型（`mml-vue/src/lib/bindings.ts`
// 是启动器自己页面的 IPC 包装，自定义页面用不到，也不该被它牵着走）。
//
// `window.mml` 由启动器注入：返回自定义主页面的 HTML 会被塞进
// `<script src="/__mml_bridge.js">`，那个脚本在页面里建立 `window.mml`。
// 桥接脚本本体见启动器仓库 `mml-gui/src-tauri/resources/custom_home_bridge.js`。
//
// 注意：自定义页面调命令是「原始命令名 + 原始 camelCase 参数」，
// 没有 bindings.ts 那层 `commands.xxx.yyy(...)` 包装。

/** 实例信息（`main_get_instances` 返回数组的元素，字段与启动器 InstanceInfoDto 一致） */
export interface MmlInstance {
  uuid: string;
  name: string;
  /** 所属分组（未分组为 null） */
  group: string | null;
  /** 游戏版本，如 1.20.1 */
  version: string;
  /** 版本类型（release / snapshot …，可能为 null） */
  versionType: string | null;
  /** Mod 加载器名（原版为空串） */
  loader: string;
  loaderVersion: string | null;
  /** 实例目录绝对路径 */
  dir: string;
  /** 是否正在运行 */
  running: boolean;
  modpackType: string | null;
  pid: string | null;
  fid: string | null;
  serverUrl: string | null;
  lang: string | null;
  logEncoding: string | null;
  source: string | null;
}

/** 启动阶段事件（`launch-state` 的 payload） */
export interface MmlLaunchState {
  uuid: string;
  /** 阶段标识，如 login / check / download / jvm / end */
  state: string;
  /** 进度百分比；该阶段没有进度语义时为 null */
  progress: number | null;
}

/** 游戏退出事件（`game-exit` 的 payload） */
export interface MmlExitEvent {
  uuid: string;
  /** 退出码，0 表示正常退出 */
  code: number;
}

/** 启动失败事件（`launch-error` 的 payload） */
export interface MmlErrorEvent {
  uuid: string | null;
  message: string;
}

/** 主题（启动器 `data-theme` 的取值） */
export type MmlTheme = "Dark" | "Light";

/** 启动器注入的桥接对象 */
export interface MmlApi {
  /**
   * 调用启动器命令
   *
   * 命令名 = `#[tauri::command]` 的函数名，如 `main_get_instances`；
   * 参数键名与启动器源码里的参数名一致（camelCase）。
   * 命令不存在或执行失败时 Promise 会被 reject（错误是 `Error`，message 是后端错误串）。
   */
  invoke<T = unknown>(cmd: string, args?: Record<string, unknown> | null): Promise<T>;
  /**
   * 订阅启动器事件，返回取消订阅函数
   *
   * 只支持启动器白名单里的事件（见工程 README），白名单外会立即收到一条错误应答。
   * payload 是原始数据、没有类型信息；用 `src/mml.ts` 里的 onLaunchState / onGameExit
   * 这类小包装可以拿到类型。
   */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  on(event: string, cb: (payload: any) => void): () => void;
  /**
   * 订阅主题变化，返回取消订阅函数
   *
   * 注册后**不会**立刻回调一次；当前值读 `mml.theme`，或者干脆用
   * `<html data-mml-theme="Dark|Light">` 写纯 CSS（本工程就是后者）。
   */
  onTheme(cb: (theme: MmlTheme) => void): () => void;
  /**
   * 当前主题（启动器下发的，实时更新）
   *
   * iframe 是独立文档，拿不到启动器页面上的 CSS 变量，只能靠这个值 / `data-mml-theme` 属性。
   * 启动器还没下发时（浏览器预览、或启动器主题为 System 时系统刚切换），
   * 桥接脚本会先按系统偏好（`prefers-color-scheme`）兜底。
   */
  theme: MmlTheme;
  /**
   * 与启动器父窗口的握手
   *
   * 正常在启动器里打开时会很快 resolve；页面不是启动器加载的（比如浏览器里直接打开）
   * 则要等桥接脚本重试耗尽（约 10 秒）才 resolve，且之后的 invoke 永远不会有应答。
   * 所以**不要**用 `ready` 判断自己是否在启动器里，用 `probeHost()`（见 src/bridge.ts）。
   */
  ready: Promise<void>;
}

declare global {
  interface Window {
    /** 不在启动器里打开时不存在 */
    mml?: MmlApi;
  }
}
