// 模组的自定义分组：分组表的加载、增删改，"把模组归到某个组"，以及**分组块的显示顺序**
//
// 分组数据属于实例的 GUI 设置：`guisetting.json` 的 `Mod.Groups`（见后端 crate::gui_setting）。
// **分组用 uuid 作键**，名字只是对象里的一个字段（可以随便改，键不动）；成员按模组
// **SHA1** 记 —— 它是内容哈希，启用/禁用（改文件名）之后不变。
//
// 顺序（含三个**状态分组**）存在同一个文件的 `Mod.GroupOrder`：`Groups` 是 HashMap、
// 没有"建立顺序"，而用户要能拖动状态分组，所以顺序只能单独存一份。
// 顺序的规范化在后端 `resource.rs::group_order_of`（丢弃已删的分组、补上没记的），
// 前端只负责把拖出来的那一串原样交过去。
import { computed, ref } from "vue";
import { tErr } from "../../../lib/i18n";
import { showToast } from "../../../lib/toast";
import {
  addModGroup,
  getModGroups,
  getModGroupsCollapsed,
  removeModGroup,
  renameModGroup,
  setModGroup,
  setModGroupOrder,
  setModGroupsCollapsed,
} from "../../../lib/api";
import type { ModGroupDto } from "../../../lib/bindings";
import type { useResourceData } from "./useResourceData";

/**
 * 三个**状态分组**的固定 uuid
 *
 * 与后端 `resource.rs` 的 `STATE_GROUP_FAIL` / `_ON` / `_OFF` 一一对应（改要一起改）。
 * 它们不是用户数据（不进 `Mod.Groups`），但同样参与排序与折叠，所以需要稳定的键 ——
 * 现在与自建分组同一套形状（都是 uuid），前端不必再维护"两套键"的映射。
 *
 * 取值是"全 0 / 尾号 1 / 尾号 2"：调试时一眼认得出是状态分组，不用查表。
 */
export const STATE_GROUP_ID_FAIL = "00000000-0000-0000-0000-000000000000";
export const STATE_GROUP_ID_ON = "00000000-0000-0000-0000-000000000001";
export const STATE_GROUP_ID_OFF = "00000000-0000-0000-0000-000000000002";

export function useModGroups(data: ReturnType<typeof useResourceData>) {
  const { instanceUuid } = data;

  /**
   * 分组表（顺序 = 后端下发的顺序 = 用户拖出来的顺序；没拖过则是默认顺序）
   *
   * `guisetting.json` 的 `Groups` 是 HashMap，没有"建立顺序"可言，所以顺序由后端
   * `resource.rs::group_order_of` 统一决定再下发；前端**不要**再排一次
   * （两处排序规则一旦不一致，就会出现"看着没变但顺序变了"）。
   */
  const groups = ref<ModGroupDto[]>([]);

  /**
   * 分组块的顺序（**含状态分组**，键都是分组 uuid）
   *
   * 与 `groups` 是两份数据：这一份管排列，`groups` 管分组内容。
   * 后端只在 `resource_mod_groups` 里下发自建分组，所以状态分组的 uuid 要在这里补齐 ——
   * 顺序表是上下连贯的一串，缺了它们就没法把状态分组拖到前面去。
   */
  const order = ref<string[]>([]);

  /** 默认顺序：识别失败 → 已启用 → 已禁用（与后端 `default_group_order` 一致） */
  const DEFAULT_ORDER = [STATE_GROUP_ID_FAIL, STATE_GROUP_ID_ON, STATE_GROUP_ID_OFF];

  /**
   * 把后端下发的分组表还原成完整的块顺序
   *
   * 后端交给我们的 `groups` 已经是**按顺序排好的**，所以按它遍历就能把自建分组插回原位；
   * 状态分组的 uuid 不在里面，按默认顺序补在末尾 —— 用户拖过的话会被后面的落盘顺序覆盖。
   */
  function composeOrder(list: ModGroupDto[]): string[] {
    return [...list.map((g) => g.uuid), ...DEFAULT_ORDER];
  }

  /** 模组 SHA1 → 所在分组 uuid（未归组没有条目） */
  const groupOfKey = computed(() => {
    const map = new Map<string, string>();
    for (const group of groups.value) {
      for (const sha1 of group.mods) {
        map.set(sha1, group.uuid);
      }
    }
    return map;
  });

  /**
   * **收起**的分组块键集合（分组 uuid，与 `order` 同一套口径）
   *
   * 存"收起的那些"：默认全展开，空集合即初始状态。落盘在实例的
   * `guisetting.json`（`Mod.GroupCollapsed`），所以切换实例各记各的。
   */
  const collapsed = ref<Set<string>>(new Set());

  /** 拉分组（进入模组分类时调用） */
  async function load() {
    if (!instanceUuid.value) {
      groups.value = [];
      order.value = [];
      collapsed.value = new Set();
      return;
    }
    try {
      // 分组表与收起状态一起拉：同一份 guisetting.json，两次 IPC 不如一次拿到
      const [list, folded] = await Promise.all([
        getModGroups(instanceUuid.value),
        getModGroupsCollapsed(instanceUuid.value),
      ]);
      // 后端下发的是"按当前顺序排好的自建分组"，据此拼出完整顺序
      groups.value = list;
      order.value = composeOrder(list);
      collapsed.value = new Set(folded);
    } catch {
      groups.value = [];
      order.value = [];
      collapsed.value = new Set();
    }
  }

  /** 保存块顺序（拖完调用；失败只提示，不回滚 —— 下次 load 会拿到后端的真实顺序） */
  async function setOrder(next: string[]) {
    order.value = next;
    try {
      await setModGroupOrder(instanceUuid.value, next);
      // 顺序表落盘时后端会规范化一次，重新拉一遍拿到一致的版本
      await load();
    } catch (e) {
      showToast(tErr(e));
    }
  }

  /**
   * 切换某个块的收起 / 展开
   *
   * 先改内存再落盘：折叠要立刻有反馈，等 IPC 回来才动会让点击看着"没反应"。
   * 失败只提示，不回滚（下次 load 会拿到后端的真实状态）。
   */
  function toggleCollapsed(key: string) {
    const next = new Set(collapsed.value);
    if (next.has(key)) {
      next.delete(key);
    } else {
      next.add(key);
    }
    collapsed.value = next;
    if (!instanceUuid.value) return;
    void setModGroupsCollapsed(instanceUuid.value, [...next]).catch((e) => showToast(tErr(e)));
  }

  function isCollapsed(key: string): boolean {
    return collapsed.value.has(key);
  }

  /** 新建分组 */
  async function add(name: string): Promise<boolean> {
    const trimmed = name.trim();
    if (!trimmed) return false;
    try {
      await addModGroup(instanceUuid.value, trimmed);
      await load();
      return true;
    } catch (e) {
      showToast(tErr(e));
      return false;
    }
  }

  /** 重命名分组（**传 uuid**：只改名字，键与成员都不动） */
  async function rename(group: string, newName: string): Promise<boolean> {
    const trimmed = newName.trim();
    if (!trimmed) return false;
    try {
      await renameModGroup(instanceUuid.value, group, trimmed);
      await load();
      return true;
    } catch (e) {
      showToast(tErr(e));
      return false;
    }
  }

  /** 删除分组（组内模组回到"未分组"，磁盘文件不动） */
  async function remove(group: string) {
    try {
      await removeModGroup(instanceUuid.value, group);
      await load();
    } catch (e) {
      showToast(tErr(e));
    }
  }

  /** 把若干模组移到某个分组（**传分组 uuid**）；`group` 传 null = 移出所有分组（回到"未分组"） */
  async function setGroup(group: string | null, keys: string[]) {
    if (!keys.length) return;
    // 已经在目标组里的不用动（拖回原组是一次空操作）
    const target = group ?? "";
    const moved = keys.filter((key) => (groupOfKey.value.get(key) ?? "") !== target);
    if (!moved.length) return;
    try {
      await setModGroup(instanceUuid.value, group, moved);
      await load();
    } catch (e) {
      showToast(tErr(e));
    }
  }

  return {
    groups,
    order,
    collapsed,
    isCollapsed,
    toggleCollapsed,
    groupOfKey,
    load,
    add,
    rename,
    remove,
    setGroup,
    setOrder,
  };
}
