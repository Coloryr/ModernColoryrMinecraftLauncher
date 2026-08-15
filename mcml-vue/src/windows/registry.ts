// 窗口注册表：定义启动器的所有窗口
// 多窗口模式下每个窗口是独立的 Tauri WebviewWindow / 浏览器标签页，
// 单窗口模式下这些窗口在应用内切换展示。

export type WindowKind =
  | "main"
  | "settings"
  | "stats"
  | "skin"
  | "help"
  | "resource"
  | "account";

export interface WindowInfo {
  kind: WindowKind;
  title: string;
  /** 窗口尺寸（多窗口模式 / 真实 Tauri 窗口时使用） */
  width: number;
  height: number;
}

export const WINDOW_REGISTRY: WindowInfo[] = [
  { kind: "main", title: "MCML 启动器", width: 1100, height: 720 },
  { kind: "settings", title: "启动器设置", width: 760, height: 600 },
  { kind: "stats", title: "游戏统计", width: 760, height: 600 },
  { kind: "skin", title: "皮肤查看", width: 760, height: 600 },
  { kind: "help", title: "帮助手册", width: 760, height: 600 },
  { kind: "resource", title: "资源管理", width: 900, height: 640 },
  { kind: "account", title: "账户管理", width: 920, height: 640 },
];

export function isWindowKind(v: string | null): v is WindowKind {
  return !!v && WINDOW_REGISTRY.some((w) => w.kind === v);
}
