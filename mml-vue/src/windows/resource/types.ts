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

/**
 * 模组的展示方式（两种视图，见 parts/mod/）
 *
 * 原先还有第三种「树形」：按 jar-in-jar 展开层级。它已**并入列表** ——
 * 层级现在就在列表列里展开（有下级的行前面是展开箭头，下一层缩进显示），
 * 所以"看得见层级"与"看得见图标/副标题"不再需要二选一。
 */
export type ModView = "list" | "table";

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
  /** 编辑备注：弹窗在 ModPane，三种视图只管把要编辑的那一条抛上来 */
  (e: "note", item: ModItemDto): void;
  /**
   * 折叠 / 展开所在分组（表格视图的第一列有折叠箭头）
   *
   * 列表视图的折叠在分组头上、树形视图已并入列表，所以只有表格会发这个事件。
   */
  (e: "toggle-group"): void;
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

/**
 * 模组的显示排序：**按 modid 正序**
 *
 * 列表与表格两个视图共用这一份 —— 各自写一遍的话迟早会漂移成两种顺序，
 * 用户在两个视图之间切换就会看到"同一个分组、变了顺序"。
 *
 * 规则：
 * 1. modid 字典序（`localeCompare` 固定 `en`，避免跟随系统区域导致顺序不稳）；
 * 2. **没有 modid 的沉到末尾**（用 `\uffff` 当哨兵：它比任何常规字符都大。
 *    不能拿空串比 —— 空串会排到最前面，让"识别失败的包"挤在正常模组之前）；
 * 3. modid 相同时按显示名（缺失时用文件名）兜底，保证顺序是全序、不抖动。
 */
export function compareMod(a: ModItemDto, b: ModItemDto): number {
  const ka = a.modId || "\uffff";
  const kb = b.modId || "\uffff";
  const byId = ka.localeCompare(kb, "en");
  if (byId !== 0) return byId;
  return (a.name || a.file).localeCompare(b.name || b.file, "en");
}

/**
 * 排好序的模组副本（**不改原数组**）
 *
 * 调用方拿到的 `items` 是分组计算出来的切片，直接 `sort` 会就地把上游数据改掉。
 */
export function sortedMods(items: ModItemDto[]): ModItemDto[] {
  return [...items].sort(compareMod);
}
