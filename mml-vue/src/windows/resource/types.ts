// 资源管理窗口的共享类型与分类清单

import type { ModItemDto } from "../../lib/bindings";

/** 资源分类（左侧导航的一项） */
export type CategoryId =
  | "saves"
  | "mods"
  | "resourcepacks"
  | "screenshots"
  | "servers"
  | "shaders"
  | "schematics";

/** 存档分类下的子页：存档本体 / 数据包 */
export type SaveTab = "saves" | "datapacks";

export interface ResourceCategory {
  id: CategoryId;
  /**
   * 显示名文案键
   *
   * 存**键**而不是 `t()` 出来的字符串：后者在 setup 阶段就把当时的语言定死了，
   * 切语言时侧栏文字不跟着变（本窗口原先就是这么冻住的）。渲染时再 `t()` 才跟着走。
   */
  labelKey: string;
}

export const RESOURCE_CATEGORIES: ResourceCategory[] = [
  { id: "saves", labelKey: "resource.saves" },
  { id: "mods", labelKey: "resource.mods" },
  { id: "resourcepacks", labelKey: "resource.resourcepacks" },
  { id: "screenshots", labelKey: "resource.screenshots" },
  { id: "servers", labelKey: "resource.servers" },
  { id: "shaders", labelKey: "resource.shaders" },
  { id: "schematics", labelKey: "resource.schematics" },
];

/** 某个分类的文案键（找不到时返回分类 ID 兜底） */
export function categoryLabelKey(id: CategoryId): string {
  return RESOURCE_CATEGORIES.find((c) => c.id === id)?.labelKey ?? id;
}

/** 服务器添加 / 编辑的表单草稿（edit=true 时另带原 name/ip 作为定位键） */
export interface ServerFormDraft {
  edit: boolean;
  name: string;
  ip: string;
  acceptTextures: boolean;
  origName: string;
  origIp: string;
}

/** 模组的展示方式（三种视图，见 parts/mod/） */
export type ModView = "list" | "table" | "tree";

/** 三种模组视图共用的 props */
export interface ModViewProps {
  /** 这一份要显示的模组（分组模式下是某个分组里的那一批） */
  items: ModItemDto[];
  /** 有操作在跑：按钮统一置灰 */
  busy: boolean;
  /** 正在被拖拽的模组 SHA1（那行变淡）：归组用 SHA1，与 guisetting.json 的 Groups 同一口径 */
  draggingKey: string | null;
}

/** 三种模组视图共用的行操作事件 */
export interface ModViewEmits {
  (e: "toggle", item: ModItemDto): void;
  (e: "remove", item: ModItemDto): void;
  (e: "open-folder", item: ModItemDto): void;
  /** 行按下：交给 ModPane 的拖拽逻辑（拖到分组上即归组） */
  (e: "drag-start", payload: { event: PointerEvent; item: ModItemDto }): void;
}

/** 一排模组行的通用文案片段：副标题 = 版本 · 作者 · 文件名 */
export function modSub(item: ModItemDto): string {
  return [item.version, item.author, item.file].filter(Boolean).join(" · ");
}

/**
 * 模组行的渲染标识（v-for 的 key）
 *
 * 内置模组（jar-in-jar）没有 sha1 也没有 uuid，退回 modid / 名字。
 * **分组用的是 `sha1`**（内容哈希，与 guisetting.json 的 Groups 一致），
 * 这个只是行身份，两件事别混。
 */
export function modRowKey(item: ModItemDto): string {
  return item.sha1 || item.uuid || item.modId || item.name;
}
