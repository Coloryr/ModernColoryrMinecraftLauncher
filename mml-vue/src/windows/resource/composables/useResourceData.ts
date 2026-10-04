// 资源管理窗口的数据层：当前实例 + 七个分类的列表 + 数据包子页
//
// 分类与存档子页的**选择**也放这里（`category` / `saveTab`）：加载是跟着它们走的
// （watch 在里面），外壳与各列表组件共用同一份，不必再往上抛事件。
//
// 拆出来的原因：原先这 250 行和模板、样式挤在一个 1100 行的 SFC 里，
// 列表组件想拿数据只能整份 prop 传下去；现在按 `windows/block` 的写法
// （薄窗口 + parts/ + composables/）拆开，语义边界清楚。
import { ref, watch } from "vue";
import { tErr } from "../../../lib/i18n";
import { showToast } from "../../../lib/toast";
import {
  api,
  getImageBaseUrl,
  listDatapacks,
  listMods,
  listResourcepacks,
  listSaves,
  listScreenshots,
  listServers,
  listShaderpacks,
  listSchematics,
} from "../../../lib/api";
import { loadGuiConfig } from "../../../lib/guiConfig";
import type {
  DataPackItemDto,
  ModItemDto,
  PackItemDto,
  SaveItemDto,
  SchematicItemDto,
  ScreenshotItemDto,
  ServerItemDto,
  ShaderItemDto,
} from "../../../lib/bindings";
import type { CategoryId, SaveTab } from "../types";

/** 左侧导航上显示的当前实例（只需要名字与版本） */
export interface ResourceInstance {
  name: string;
  version: string;
}

export function useResourceData(initialCategory: CategoryId = "saves") {
  const category = ref<CategoryId>(initialCategory);
  const saveTab = ref<SaveTab>("saves");

  const instance = ref<ResourceInstance | null>(null);
  /** 目标实例 uuid：后端每个命令都要它；没有实例时为空串，加载直接跳过 */
  const instanceUuid = ref("");

  /** 有列表正在拉（模组那个分类要解析 jar 元数据，可能要几秒） */
  const loading = ref(false);

  const mods = ref<ModItemDto[]>([]);
  const packs = ref<PackItemDto[]>([]);
  const saves = ref<SaveItemDto[]>([]);
  const shots = ref<ScreenshotItemDto[]>([]);
  const servers = ref<ServerItemDto[]>([]);
  const shaders = ref<ShaderItemDto[]>([]);
  const schematics = ref<SchematicItemDto[]>([]);
  const datapacks = ref<DataPackItemDto[]>([]);

  /** 数据包子页当前选中的存档目录 */
  const dpSave = ref("");

  /** 截图预览协议前缀（`mml-image://...`） */
  const imgBase = ref("");

  /**
   * 加载代次号
   *
   * 分类可以连点，而每次加载都要等后端（模组那次尤其慢）。只有**最新一发**的结果与
   * 收尾算数：否则先发的那一发回来晚了会把 `loading` 提前清掉、把界面闪成空态
   * （与仓库里 `sourceSeq` 修排序竞态同一做法）。
   */
  let loadSeq = 0;

  /** 按当前分类拉列表（每次切换 / 操作后都重拉，保证与磁盘一致） */
  async function load() {
    if (!instanceUuid.value) return;
    const seq = ++loadSeq;
    loading.value = true;
    try {
      switch (category.value) {
        case "mods":
          mods.value = await listMods(instanceUuid.value);
          break;
        case "resourcepacks":
          packs.value = await listResourcepacks(instanceUuid.value);
          break;
        case "saves":
          saves.value = await listSaves(instanceUuid.value);
          break;
        case "screenshots":
          shots.value = await listScreenshots(instanceUuid.value);
          break;
        case "servers":
          servers.value = await listServers(instanceUuid.value);
          break;
        case "shaders":
          shaders.value = await listShaderpacks(instanceUuid.value);
          break;
        case "schematics":
          schematics.value = await listSchematics(instanceUuid.value);
          break;
      }
    } catch (e) {
      if (seq === loadSeq) showToast(tErr(e));
    } finally {
      // 慢的那一发回来时可能已经有更新的一发了：别把它的加载态关掉
      if (seq === loadSeq) loading.value = false;
    }
  }

  /** 数据包列表（依赖子页选中的存档） */
  async function loadDatapacks() {
    if (!instanceUuid.value || !dpSave.value) return;
    const seq = ++loadSeq;
    loading.value = true;
    try {
      datapacks.value = await listDatapacks(instanceUuid.value, dpSave.value);
    } catch (e) {
      if (seq === loadSeq) {
        datapacks.value = [];
        showToast(tErr(e));
      }
    } finally {
      if (seq === loadSeq) loading.value = false;
    }
  }

  /** 当前显示的是数据包子页（重拉时要挑对那一份加载函数） */
  function isDatapackTab(): boolean {
    return category.value === "saves" && saveTab.value === "datapacks";
  }

  /** 重拉**当前显示的东西**：操作成功之后调用（分类列表 / 数据包子页） */
  async function reloadCurrent() {
    if (isDatapackTab()) {
      await loadDatapacks();
    } else {
      await load();
    }
  }

  /** 进入数据包子页：先保证存档列表在、选中一个存档，再拉数据包 */
  async function enterDatapackTab() {
    if (!instanceUuid.value) return;
    if (!saves.value.length) await load();
    if (!dpSave.value && saves.value.length) {
      // 选中变化会触发下面的 watch，由它去拉数据包
      dpSave.value = saves.value[0].dir;
      return;
    }
    await loadDatapacks();
  }

  /** 当前实例 + 当前分类数据（首次挂载与切回本窗口都走这里） */
  async function sync() {
    try {
      // 当前实例取自 gui_config.json（主窗口选中时写入），本窗口是独立 webview，需自己读一次
      const [list, cfg] = await Promise.all([api.getInstances(), loadGuiConfig()]);
      const uuid = cfg?.mainWindow.selectedInstance ?? "";
      const found = list.find((i) => i.uuid === uuid) ?? list[0] ?? null;
      instance.value = found ? { name: found.name, version: found.version } : null;
      instanceUuid.value = found?.uuid ?? "";
      imgBase.value = await getImageBaseUrl();
    } catch {
      instance.value = null;
      instanceUuid.value = "";
    }
    // 重拉**当前显示的那一份**（停在数据包子页时不该去拉存档列表）
    await reloadCurrent();
  }

  /** 截图地址（走 `mml-image` 协议按实例 + 文件名取） */
  function shotUrl(item: ScreenshotItemDto): string {
    return `${imgBase.value}/screenshot/${instanceUuid.value}/${item.name}`;
  }

  watch(category, () => {
    // 停在数据包子页时切分类再切回来：要按子页那套重新进入，别把存档列表当数据包拉
    if (isDatapackTab()) {
      void enterDatapackTab();
      return;
    }
    void load();
  });
  watch(saveTab, (tab) => {
    if (tab === "datapacks") void enterDatapackTab();
  });
  watch(dpSave, () => void loadDatapacks());

  return {
    category,
    saveTab,
    instance,
    instanceUuid,
    loading,
    mods,
    packs,
    saves,
    shots,
    servers,
    shaders,
    schematics,
    datapacks,
    dpSave,
    reloadCurrent,
    sync,
    shotUrl,
  };
}
