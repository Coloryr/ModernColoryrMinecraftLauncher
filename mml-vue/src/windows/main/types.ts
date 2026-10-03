// 主窗口共享类型（MainWindow 与其子组件共用）
import type { InstanceInfoDto } from "../../lib/bindings";

/** 实例列表显示模式（来源见 lib/guiConfig.ts，默认 list） */
export type { ViewMode } from "../../lib/guiConfig";

export type FeatureId = "settings" | "stats" | "help" | "collect";

export interface GroupView {
  /** 分组 uuid（身份；改名不影响它） */
  id: string;
  /** 分组显示名（默认分组已翻译成"默认分组"） */
  name: string;
  /** 是否默认分组（它的名字是空白，不能靠名字判断） */
  isDefault: boolean;
  items: InstanceInfoDto[];
}

/** 右键菜单状态 */
export interface CtxMenuState {
  x: number;
  y: number;
  kind: "group" | "multi" | "instance";
  /** 分组 uuid（kind === "group" 时） */
  groupId?: string;
  instance?: InstanceInfoDto;
}

/** 拖拽候选（实例 / 分组标题） */
export interface DragCandidate {
  kind: "instance" | "group";
  instance?: InstanceInfoDto;
  /** 分组 uuid（身份：拖动与落点判断都用它） */
  groupId?: string;
  /** 分组显示名（只用于拖拽时那块占位上的文案） */
  groupName?: string;
}

/** 实例右键菜单动作 */
export type InstMenuAction =
  | "launch"
  | "openFolder"
  | "viewLog"
  | "editConfig"
  | "changeIcon"
  | "rename"
  | "delete";
