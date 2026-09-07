// 主窗口共享类型（MainWindow 与其子组件共用）
import type { InstanceInfo } from "../../lib/types";

export type ViewMode = "group" | "grid" | "list";

export type FeatureId = "settings" | "stats" | "skin" | "help" | "download";

export interface GroupView {
  name: string;
  items: InstanceInfo[];
}

/** 右键菜单状态 */
export interface CtxMenuState {
  x: number;
  y: number;
  kind: "group" | "multi" | "instance";
  group?: string;
  instance?: InstanceInfo;
}

/** 拖拽候选（实例 / 分组标题） */
export interface DragCandidate {
  kind: "instance" | "group";
  instance?: InstanceInfo;
  groupName?: string;
}

/** 实例右键菜单动作 */
export type InstMenuAction =
  | "launch"
  | "openFolder"
  | "viewLog"
  | "editConfig"
  | "rename"
  | "delete";
