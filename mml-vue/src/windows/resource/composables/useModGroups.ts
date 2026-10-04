// 模组的自定义分组：分组表的加载、增删改，以及"把模组归到某个组"
//
// 分组数据属于实例的 GUI 设置：`guisetting.json` 的 `Mod.Groups`（见后端 crate::gui_setting，
// 与 ColorMC 互通），成员按模组 **SHA1** 记 —— 它是内容哈希，启用/禁用（改文件名）之后不变；
// uuid 是文件路径的 v5，会跟着变，所以不能用它。
import { computed, ref } from "vue";
import { tErr } from "../../../lib/i18n";
import { showToast } from "../../../lib/toast";
import { addModGroup, getModGroups, removeModGroup, renameModGroup, setModGroup } from "../../../lib/api";
import type { ModGroupDto } from "../../../lib/bindings";
import type { useResourceData } from "./useResourceData";

export function useModGroups(data: ReturnType<typeof useResourceData>) {
  const { instanceUuid } = data;

  /** 分组表（顺序 = 后端存的顺序 = 用户建立顺序） */
  const groups = ref<ModGroupDto[]>([]);

  /** 模组 key → 所在分组名（未归组没有条目） */
  const groupOfKey = computed(() => {
    const map = new Map<string, string>();
    for (const group of groups.value) {
      for (const key of group.mods) {
        map.set(key, group.name);
      }
    }
    return map;
  });

  /** 拉分组（进入模组分类时调用） */
  async function load() {
    if (!instanceUuid.value) {
      groups.value = [];
      return;
    }
    try {
      groups.value = await getModGroups(instanceUuid.value);
    } catch {
      groups.value = [];
    }
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

  /** 重命名分组 */
  async function rename(name: string, newName: string): Promise<boolean> {
    const trimmed = newName.trim();
    if (!trimmed) return false;
    try {
      await renameModGroup(instanceUuid.value, name, trimmed);
      await load();
      return true;
    } catch (e) {
      showToast(tErr(e));
      return false;
    }
  }

  /** 删除分组（组内模组回到"未分组"，磁盘文件不动） */
  async function remove(name: string) {
    try {
      await removeModGroup(instanceUuid.value, name);
      await load();
    } catch (e) {
      showToast(tErr(e));
    }
  }

  /** 把若干模组移到某个分组；`group` 传 null = 移出所有分组（回到"未分组"） */
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

  return { groups, groupOfKey, load, add, rename, remove, setGroup };
}
