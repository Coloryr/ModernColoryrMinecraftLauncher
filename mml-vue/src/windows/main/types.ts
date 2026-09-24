// 主窗口共享类型（MainWindow 与其子组件共用）
import type { InstanceInfoDto } from "../../lib/bindings";

/** 实例列表显示模式（来源见 lib/guiConfig.ts，默认 list） */
export type { ViewMode } from "../../lib/guiConfig";

export type FeatureId = "settings" | "stats" | "skin" | "help" | "download" | "collect" | "log";

export interface GroupView {
  name: string;
  items: InstanceInfoDto[];
}

/** 右键菜单状态 */
export interface CtxMenuState {
  x: number;
  y: number;
  kind: "group" | "multi" | "instance";
  group?: string;
  instance?: InstanceInfoDto;
}

/** 拖拽候选（实例 / 分组标题） */
export interface DragCandidate {
  kind: "instance" | "group";
  instance?: InstanceInfoDto;
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
