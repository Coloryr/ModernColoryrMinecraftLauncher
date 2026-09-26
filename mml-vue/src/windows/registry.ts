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
  | "account"
  | "add"
  | "add_modpack"
  | "add_resource"
  | "collect"
  | "download"
  | "block";

export interface WindowInfo {
  kind: WindowKind;
  title: string;
}

// 窗口宽高不在前端维护：唯一来源是后端 WINDOWS_INFO（window_get_window_sizes
// 命令下发），真实窗口与 JS 回退路径都用后端的默认尺寸 + 历史几何
export const WINDOW_REGISTRY: WindowInfo[] = [
  { kind: "main", title: "ModernMinecraftLauncher" },
  { kind: "settings", title: "启动器设置" },
  { kind: "stats", title: "游戏统计" },
  { kind: "skin", title: "皮肤查看" },
  { kind: "help", title: "帮助手册" },
  { kind: "resource", title: "资源管理" },
  { kind: "account", title: "账户管理" },
  { kind: "add", title: "添加实例" },
  { kind: "add_modpack", title: "下载整合包" },
  { kind: "add_resource", title: "添加资源" },
  { kind: "collect", title: "资源收藏" },
  { kind: "download", title: "下载管理" },
  { kind: "block", title: "方块列表" },
];

export function isWindowKind(v: string | null): v is WindowKind {
  return !!v && WINDOW_REGISTRY.some((w) => w.kind === v);
}
