// 资源管理窗口的视图偏好：左侧分类顺序 / 上次打开的类别 / 模组展示方式
//
// **存在实例自己的 `gui_setting.json` 里**（`Gui` 字段，见后端 `gui_setting::GameViewSettingObj`），
// 不走前端本地存储：这三项都是"这个实例我习惯怎么看"，换个实例就该换一套 ——
// 本地存储是每台机器一份，切实例时顺序不变反而奇怪。
//
// 与模组分组同一份文件、同一套读改写（load → 改这一块 → 存回），互不覆盖。
// **排序口径与默认值全在这里**：后端只保管"一串分类 id"，不认识有哪几个分类
// （新增分类时只改这个文件，不用动 Rust）。
import { ref } from "vue";
import { getResourceView, setResourceView } from "../../../lib/api";
import { RESOURCE_CATEGORIES, type CategoryId, type ModView } from "../types";
import type { useResourceData } from "./useResourceData";

interface ViewPref {
  /** 左侧分类的顺序（用户拖出来的；只认已知分类，新增分类自动补在末尾） */
  order: string[];
  /** 上次打开的类别 */
  category: string;
  /** 模组的展示方式：列表 / 表格 / 树 */
  modView: string;
}

function isCategory(value: unknown): value is CategoryId {
  return RESOURCE_CATEGORIES.some((c) => c.id === value);
}

function isModView(value: unknown): value is ModView {
  return value === "list" || value === "table";
}

/**
 * 把存下来的顺序补齐成"完整且不重复"的一份
 *
 * 只认已知分类：版本升级后新增了分类，它会自动补在末尾；删掉的分类直接从顺序里消失。
 * 空数组（没存过）也走这条路，得到的就是全部分类的默认顺序。
 */
function normalizeOrder(saved: unknown): CategoryId[] {
  const all = RESOURCE_CATEGORIES.map((c) => c.id);
  const list = Array.isArray(saved) ? saved.filter(isCategory) : [];
  const seen = new Set<CategoryId>();
  const order: CategoryId[] = [];
  for (const id of [...list, ...all]) {
    if (seen.has(id)) continue;
    seen.add(id);
    order.push(id);
  }
  return order;
}

/**
 * 默认打开的分类
 *
 * 模组是这个窗口最常翻的一类，在 `RESOURCE_CATEGORIES` 里也排第一 ——
 * 打开就停在模组上，与左侧列表的顺序一致。
 * 单独抽成常量：它散在 4 处（初值 / 无实例 / 存的值不认识），改的时候容易漏。
 */
const DEFAULT_CATEGORY: CategoryId = "mods";

export function useResourceView(data: ReturnType<typeof useResourceData>) {
  const { instanceUuid } = data;

  /** 左侧分类的显示顺序（拖一下就会写回实例的 gui_setting.json） */
  const order = ref<CategoryId[]>(normalizeOrder([]));
  /** 上次打开的类别（没有记录时用 [`DEFAULT_CATEGORY`]） */
  const initialCategory = ref<CategoryId>(DEFAULT_CATEGORY);
  /** 模组的展示方式（列表 / 表格 / 树） */
  const modView = ref<ModView>("list");
  /** 当前类别（保存时要一起写回，和 order 是同一份偏好） */
  let lastCategory: CategoryId = DEFAULT_CATEGORY;

  /**
   * 加载代次号
   *
   * 实例可以切换，而每次加载都要等后端读文件；只有最新一发算数，
   * 否则先发后到的会把新实例的偏好覆盖成旧实例的（与 useResourceData 的 loadSeq 同一做法）。
   */
  let loadSeq = 0;

  /** 存一份到实例设置（后端整份覆盖这一块，未改的项照传） */
  function save() {
    const uuid = instanceUuid.value;
    if (!uuid) return;
    const value: ViewPref = {
      order: order.value,
      category: lastCategory,
      modView: modView.value,
    };
    // 失败只提示、不回滚：偏好记不住不该打断使用
    void setResourceView(uuid, value.order, value.category, value.modView).catch(() => { });
  }

  /** 读该实例的偏好（进入资源窗口 / 切换实例时调用） */
  async function load() {
    const uuid = instanceUuid.value;
    if (!uuid) {
      // 没有实例：回到默认顺序与默认类别
      order.value = normalizeOrder([]);
      modView.value = "list";
      initialCategory.value = DEFAULT_CATEGORY;
      lastCategory = DEFAULT_CATEGORY;
      return;
    }
    const seq = ++loadSeq;
    let pref: ViewPref;
    try {
      pref = await getResourceView(uuid);
    } catch {
      pref = { order: [], category: "", modView: "" };
    }
    // 慢的那一发回来时可能已经切到别的实例了：丢弃
    if (seq !== loadSeq) return;

    order.value = normalizeOrder(pref.order);
    modView.value = isModView(pref.modView) ? pref.modView : "list";
    const category: CategoryId = isCategory(pref.category) ? pref.category : DEFAULT_CATEGORY;
    initialCategory.value = category;
    lastCategory = category;
  }

  /** 记住这次打开的是哪一类（切换分类时由外壳调用） */
  function rememberCategory(id: CategoryId) {
    lastCategory = id;
    save();
  }

  /** 记下拖动后的新顺序 */
  function setOrder(next: CategoryId[]) {
    order.value = normalizeOrder(next);
    save();
  }

  /** 切换模组展示方式 */
  function setModView(next: ModView) {
    modView.value = next;
    save();
  }

  return {
    order,
    initialCategory,
    modView,
    load,
    rememberCategory,
    setOrder,
    setModView,
  };
}
