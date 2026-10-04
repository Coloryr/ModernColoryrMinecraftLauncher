// 资源管理窗口的视图偏好：左侧分类顺序 / 上次打开的类别（将来还有模组的展示方式与分组）
//
// 纯界面偏好，存 localStorage —— 与方块窗口的 `mml.blockView` 同一做法：
// 换台机器 / 清了缓存不影响功能，也就没必要塞进 gui_config 那份跨窗口配置里。
import { ref } from "vue";
import { RESOURCE_CATEGORIES, type CategoryId, type ModView } from "../types";

const VIEW_KEY = "mml.resourceView";

interface ViewPref {
  /** 左侧分类的顺序（用户拖出来的；只认已知分类，新增分类自动补在末尾） */
  order: CategoryId[];
  /** 上次打开的类别 */
  category: CategoryId;
  /** 模组的展示方式：列表 / 表格 / 树 */
  modView: ModView;
}

function isCategory(value: unknown): value is CategoryId {
  return RESOURCE_CATEGORIES.some((c) => c.id === value);
}

function isModView(value: unknown): value is ModView {
  return value === "list" || value === "table" || value === "tree";
}

function readPref(): Partial<ViewPref> {
  try {
    const raw: unknown = JSON.parse(localStorage.getItem(VIEW_KEY) ?? "{}");
    return raw && typeof raw === "object" ? (raw as Partial<ViewPref>) : {};
  } catch {
    return {};
  }
}

/**
 * 把存下来的顺序补齐成"完整且不重复"的一份
 *
 * 只认已知分类：版本升级后新增了分类，它会自动补在末尾；删掉的分类直接从顺序里消失。
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

export function useResourceView() {
  const pref = readPref();

  /** 左侧分类的显示顺序（拖一下就会写回 localStorage） */
  const order = ref<CategoryId[]>(normalizeOrder(pref.order));
  /** 上次打开的类别（没有记录时默认存档） */
  const initialCategory: CategoryId = isCategory(pref.category) ? pref.category : "saves";
  /** 模组的展示方式（列表 / 表格 / 树） */
  const modView = ref<ModView>(isModView(pref.modView) ? pref.modView : "list");
  /** 当前类别（save 时要一起写回，和 order 是同一份偏好） */
  let lastCategory: CategoryId = initialCategory;

  function save() {
    const value: ViewPref = {
      order: order.value,
      category: lastCategory,
      modView: modView.value,
    };
    try {
      localStorage.setItem(VIEW_KEY, JSON.stringify(value));
    } catch {
      /* 隐私模式等写不了：偏好记不住不影响用 */
    }
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
    rememberCategory,
    setOrder,
    setModView,
  };
}
