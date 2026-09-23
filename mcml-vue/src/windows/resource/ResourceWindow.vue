<script setup lang="ts">
// 资源管理窗口：存档 / 数据包 / 模组 / 资源包 / 截图 / 服务器 / 光影包 / 结构文件
// 数据来自 resource_* 命令；删除进回收站，操作后重拉当前分类
import { computed, onMounted, ref, watch } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import AsyncImage from "../../components/ui/AsyncImage.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import { t, tErr } from "../../lib/i18n";
import { showToast } from "../../lib/toast";
import {
  addServer,
  api,
  backupSave,
  clearScreenshots,
  deleteDatapack,
  deleteMod,
  deleteResourcepack,
  deleteSave,
  deleteScreenshot,
  deleteServer,
  deleteShaderpack,
  deleteSchematic,
  disableMod,
  enableMod,
  getImageBaseUrl,
  listDatapacks,
  listMods,
  listResourcepacks,
  listSaves,
  listScreenshots,
  listServers,
  listShaderpacks,
  listSchematics,
  openResourceFolder,
  setShader,
  toggleDatapack,
  updateServer,
} from "../../lib/api";
import { loadGuiConfig } from "../../lib/guiConfig";
import type {
  DataPackItemDto,
  ModItemDto,
  PackItemDto,
  SaveItemDto,
  ScreenshotItemDto,
  ServerItemDto,
  ShaderItemDto,
  SchematicItemDto,
} from "../../lib/bindings";

type CategoryId =
  | "saves"
  | "mods"
  | "resourcepacks"
  | "screenshots"
  | "servers"
  | "shaders"
  | "schematics";

const category = ref<CategoryId>("saves");
const saveTab = ref<"saves" | "datapacks">("saves");

const instance = ref<InstanceInfoLite | null>(null);

/** 目标实例（gui_config 里主窗口选中的实例） */
let instanceUuid = "";

const loading = ref(false);
const busy = ref(false);

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

/** 截图预览协议前缀 */
const imgBase = ref("");

interface InstanceInfoLite {
  name: string;
  version: string;
}

const categories: Array<{ id: CategoryId; label: string }> = [
  { id: "saves", label: t("resource.saves") },
  { id: "mods", label: t("resource.mods") },
  { id: "resourcepacks", label: t("resource.resourcepacks") },
  { id: "screenshots", label: t("resource.screenshots") },
  { id: "servers", label: t("resource.servers") },
  { id: "shaders", label: t("resource.shaders") },
  { id: "schematics", label: t("resource.schematics") },
];

const categoryName = computed(
  () => categories.find((c) => c.id === category.value)?.label ?? category.value,
);

// ---------- 数据加载 ----------

/** 按当前分类拉取列表（每次切换 / 操作后都重拉，保证与磁盘一致） */
async function load() {
  if (!instanceUuid) return;
  loading.value = true;
  try {
    switch (category.value) {
      case "mods":
        mods.value = await listMods(instanceUuid);
        break;
      case "resourcepacks":
        packs.value = await listResourcepacks(instanceUuid);
        break;
      case "saves":
        saves.value = await listSaves(instanceUuid);
        break;
      case "screenshots":
        shots.value = await listScreenshots(instanceUuid);
        break;
      case "servers":
        servers.value = await listServers(instanceUuid);
        break;
      case "shaders":
        shaders.value = await listShaderpacks(instanceUuid);
        break;
      case "schematics":
        schematics.value = await listSchematics(instanceUuid);
        break;
    }
  } catch (e) {
    showToast(tErr(e));
  } finally {
    loading.value = false;
  }
}

/** 数据包列表（依赖子页选中的存档） */
async function loadDatapacks() {
  if (!instanceUuid || !dpSave.value) return;
  loading.value = true;
  try {
    datapacks.value = await listDatapacks(instanceUuid, dpSave.value);
  } catch (e) {
    datapacks.value = [];
    showToast(tErr(e));
  } finally {
    loading.value = false;
  }
}

function selectCategory(id: CategoryId) {
  category.value = id;
}

watch(category, load);
watch(saveTab, async (tab) => {
  if (tab !== "datapacks") return;
  // 首次进入数据包子页：存档列表未加载则先拉，选中第一个存档（watch dpSave 触发加载）
  if (!saves.value.length && instanceUuid) await load();
  if (!dpSave.value && saves.value.length) dpSave.value = saves.value[0].dir;
});
watch(dpSave, loadDatapacks);

// ---------- 通用操作 ----------

/** 确认弹窗（删除类操作共用）：title/text 为提示文案，run 为确认后执行的动作 */
const confirmBox = ref<{ title: string; text: string; run: () => Promise<void> } | null>(null);
const confirmBusy = ref(false);

function askConfirm(title: string, text: string, run: () => Promise<void>) {
  confirmBox.value = { title, text, run };
}

async function runConfirm() {
  const box = confirmBox.value;
  if (!box || confirmBusy.value) return;
  confirmBusy.value = true;
  try {
    await box.run();
    confirmBox.value = null;
  } catch (e) {
    showToast(tErr(e));
  } finally {
    confirmBusy.value = false;
  }
}

/** 执行操作（不弹确认的轻操作共用）：失败 toast，成功后重拉列表 */
async function act(run: () => Promise<void>) {
  if (busy.value) return;
  busy.value = true;
  try {
    await run();
    await load();
  } catch (e) {
    showToast(tErr(e));
  } finally {
    busy.value = false;
  }
}

function openFolder(kind: string, name: string | null, parent: string | null = null) {
  act(async () => {
    await openResourceFolder(instanceUuid, kind, name, parent);
  });
}

// ---------- 模组 ----------

function toggleMod(item: ModItemDto) {
  act(async () => {
    if (item.disable) {
      await enableMod(instanceUuid, item.uuid);
    } else {
      await disableMod(instanceUuid, item.uuid);
    }
  });
}

function deleteModAsk(item: ModItemDto) {
  askConfirm(t("resource.delete"), t("resource.deleteConfirm", { name: item.name || item.file }), async () => {
    await deleteMod(instanceUuid, item.uuid);
  });
}

// ---------- 材质包 ----------

function deletePackAsk(item: PackItemDto) {
  askConfirm(t("resource.delete"), t("resource.deleteConfirm", { name: item.file }), async () => {
    await deleteResourcepack(instanceUuid, item.file);
  });
}

// ---------- 服务器 ----------

/** 服务器表单弹窗（edit = 编辑已有条目，origName/origIp 为定位键） */
const serverForm = ref<{
  edit: boolean;
  name: string;
  ip: string;
  acceptTextures: boolean;
  origName: string;
  origIp: string;
} | null>(null);
const serverBusy = ref(false);

function openServerForm(item: ServerItemDto | null) {
  serverForm.value = item
    ? {
        edit: true,
        name: item.name,
        ip: item.ip,
        acceptTextures: item.acceptTextures,
        origName: item.name,
        origIp: item.ip,
      }
    : { edit: false, name: "", ip: "", acceptTextures: false, origName: "", origIp: "" };
}

async function saveServerForm() {
  const form = serverForm.value;
  if (!form || serverBusy.value) return;
  const name = form.name.trim();
  const ip = form.ip.trim();
  if (!name || !ip) {
    showToast(t("err.nameIp"));
    return;
  }
  serverBusy.value = true;
  try {
    if (form.edit) {
      await updateServer(instanceUuid, form.origName, form.origIp, name, ip, form.acceptTextures);
    } else {
      await addServer(instanceUuid, name, ip);
    }
    serverForm.value = null;
    await load();
  } catch (e) {
    showToast(tErr(e));
  } finally {
    serverBusy.value = false;
  }
}

function deleteServerAsk(item: ServerItemDto) {
  askConfirm(t("resource.delete"), t("resource.deleteConfirm", { name: item.name || item.ip }), async () => {
    await deleteServer(instanceUuid, item.name, item.ip);
  });
}

// ---------- 光影包 ----------

function toggleShader(item: ShaderItemDto) {
  act(async () => {
    await setShader(instanceUuid, item.selected ? null : item.file);
  });
}

function deleteShaderAsk(item: ShaderItemDto) {
  askConfirm(t("resource.delete"), t("resource.deleteConfirm", { name: item.name || item.file }), async () => {
    await deleteShaderpack(instanceUuid, item.file);
  });
}

// ---------- 结构文件 ----------

function deleteSchematicAsk(item: SchematicItemDto) {
  askConfirm(t("resource.delete"), t("resource.deleteConfirm", { name: item.name || item.file }), async () => {
    await deleteSchematic(instanceUuid, item.file);
  });
}

// ---------- 数据包 ----------

function toggleDataPack(item: DataPackItemDto) {
  act(async () => {
    await toggleDatapack(instanceUuid, dpSave.value, item.name);
  });
}

function deleteDataPackAsk(item: DataPackItemDto) {
  askConfirm(t("resource.delete"), t("resource.deleteConfirm", { name: item.file }), async () => {
    await deleteDatapack(instanceUuid, dpSave.value, item.name);
  });
}

/** 数据包启用徽标文案（enable 三态） */
function dpState(item: DataPackItemDto): string {
  if (item.enable === true) return t("resource.dpOn");
  if (item.enable === false) return t("resource.dpOff");
  return t("resource.dpNone");
}

/** 数据包显示名（去掉 NBT 登记名的 file/ 前缀） */
function dpName(item: DataPackItemDto): string {
  return item.name.startsWith("file/") ? item.name.slice(5) : item.file || item.name;
}

// ---------- 存档 ----------

function deleteSaveAsk(item: SaveItemDto) {
  askConfirm(t("resource.delete"), t("resource.deleteConfirm", { name: item.levelName || item.dir }), async () => {
    await deleteSave(instanceUuid, item.dir);
  });
}

function backupSaveRun(item: SaveItemDto) {
  act(async () => {
    const name = await backupSave(instanceUuid, item.dir);
    showToast(t("resource.backupOk", { name }));
  });
}

/** 上次游玩时间（Unix 毫秒 → 本地时间，未知显示 --） */
function formatTime(ms: number): string {
  if (!ms) return "--";
  return new Date(ms).toLocaleString();
}

// ---------- 截图 ----------

/** 当前预览的截图（BaseModal 大图） */
const preview = ref<ScreenshotItemDto | null>(null);

function shotUrl(item: ScreenshotItemDto): string {
  return `${imgBase.value}/screenshot/${instanceUuid}/${item.name}`;
}

function deleteShotAsk(item: ScreenshotItemDto) {
  askConfirm(t("resource.delete"), t("resource.deleteConfirm", { name: item.name }), async () => {
    await deleteScreenshot(instanceUuid, item.name);
    if (preview.value?.name === item.name) preview.value = null;
  });
}

function clearShotsAsk() {
  askConfirm(t("resource.clear"), t("resource.clearConfirm"), async () => {
    await clearScreenshots(instanceUuid);
    preview.value = null;
  });
}

onMounted(async () => {
  try {
    // 当前实例取自 gui_config.json（主窗口选中时写入），本窗口是独立 webview，需自己读一次
    const [list, cfg] = await Promise.all([api.getInstances(), loadGuiConfig()]);
    const uuid = cfg?.mainWindow.selectedInstance ?? "";
    const found = list.find((i) => i.uuid === uuid) ?? list[0] ?? null;
    instance.value = found ? { name: found.name, version: found.version } : null;
    instanceUuid = found?.uuid ?? "";
    imgBase.value = await getImageBaseUrl();
  } catch {
    instance.value = null;
  }
  await load();
});
</script>

<template>
  <WindowFrame :title="t('resource.title')" @close="$emit('close')">
    <div class="resource-layout">
      <!-- 分类导航 -->
      <aside class="cat-nav">
        <div class="cat-instance" v-if="instance">
          <span class="cat-inst-name">{{ instance.name }}</span>
          <span class="cat-inst-sub">{{ instance.version }}</span>
        </div>
        <button
          v-for="c in categories"
          :key="c.id"
          class="cat-item"
          :class="{ active: category === c.id }"
          @click="selectCategory(c.id)"
        >
          {{ c.label }}
        </button>
      </aside>

      <!-- 内容区 -->
      <section class="cat-content">
        <div class="content-head">
          <div>
            <!-- 存档分类：存档 / 数据包 子页 -->
            <SegmentedTabs
              v-if="category === 'saves'"
              :model-value="saveTab"
              :options="[
                { value: 'saves', label: t('resource.saves') },
                { value: 'datapacks', label: t('resource.datapacks') },
              ]"
              @update:model-value="saveTab = $event as 'saves' | 'datapacks'"
            />
            <h3 v-else class="head-title">{{ categoryName }}</h3>
          </div>
          <!-- 截图分类常显清空；服务器分类可添加；全部分类可手动刷新（游戏运行中目录可能变化） -->
          <div class="head-actions">
            <button v-if="category === 'screenshots' && shots.length" class="mini-btn danger" :disabled="busy" @click="clearShotsAsk">
              {{ t("resource.clear") }}
            </button>
            <button v-if="category === 'servers'" class="mini-btn" :disabled="busy" @click="openServerForm(null)">
              {{ t("resource.serverAdd") }}
            </button>
            <button class="mini-btn" :disabled="loading" @click="load">{{ t("resource.refresh") }}</button>
          </div>
        </div>

        <!-- 加载中（模组解析 jar 元数据可能需要数秒） -->
        <div v-if="loading" class="empty-tip">{{ t("resource.loading") }}</div>

        <!-- 模组列表 -->
        <div v-else-if="category === 'mods'" class="item-list">
          <div v-for="item in mods" :key="item.uuid" class="item-row">
            <img v-if="item.icon" class="item-icon" :src="item.icon" alt="" />
            <span v-else class="item-icon item-icon-empty">M</span>
            <div class="item-main">
              <div class="item-name-line">
                <span class="item-name">{{ item.name || item.file }}</span>
                <span v-if="item.disable" class="badge badge-dim">{{ t("resource.modDisabled") }}</span>
                <span v-if="item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
                <span v-if="item.core" class="badge">{{ t("resource.modCore") }}</span>
              </div>
              <span class="item-sub">
                {{ [item.version, item.author, item.file].filter(Boolean).join(" · ") }}
              </span>
            </div>
            <div class="item-actions">
              <button class="mini-btn" @click="toggleMod(item)">
                {{ item.disable ? t("resource.enable") : t("resource.disable") }}
              </button>
              <button class="mini-btn" @click="openFolder('mods', item.file)">{{ t("resource.openFolder") }}</button>
              <button class="mini-btn danger" @click="deleteModAsk(item)">{{ t("resource.delete") }}</button>
            </div>
          </div>
          <div v-if="!mods.length" class="empty-tip">{{ t("resource.empty") }}</div>
        </div>

        <!-- 材质包列表 -->
        <div v-else-if="category === 'resourcepacks'" class="item-list">
          <div v-for="item in packs" :key="item.file" class="item-row">
            <img v-if="item.icon" class="item-icon" :src="item.icon" alt="" />
            <span v-else class="item-icon item-icon-empty">P</span>
            <div class="item-main">
              <div class="item-name-line">
                <span class="item-name">{{ item.file }}</span>
                <span v-if="item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
              </div>
              <span class="item-sub">
                {{ [item.description, t("resource.packFormat", { format: item.packFormat })].filter(Boolean).join(" · ") }}
              </span>
            </div>
            <div class="item-actions">
              <button class="mini-btn" @click="openFolder('resourcepacks', item.file)">{{ t("resource.openFolder") }}</button>
              <button class="mini-btn danger" @click="deletePackAsk(item)">{{ t("resource.delete") }}</button>
            </div>
          </div>
          <div v-if="!packs.length" class="empty-tip">{{ t("resource.empty") }}</div>
        </div>

        <!-- 存档列表 -->
        <div v-else-if="category === 'saves' && saveTab === 'saves'" class="item-list">
          <div v-for="item in saves" :key="item.dir" class="item-row">
            <img v-if="item.icon" class="item-icon" :src="item.icon" alt="" />
            <span v-else class="item-icon item-icon-empty">S</span>
            <div class="item-main">
              <div class="item-name-line">
                <span class="item-name">{{ item.levelName || item.dir }}</span>
                <span v-if="item.broken" class="badge badge-red">{{ t("resource.broken") }}</span>
                <span v-if="item.hardCore" class="badge badge-red">Hardcore</span>
              </div>
              <span class="item-sub">{{ item.dir }} · {{ t("resource.lastPlayed", { time: formatTime(item.lastPlayed) }) }}</span>
            </div>
            <div class="item-actions">
              <button class="mini-btn" :disabled="busy" @click="backupSaveRun(item)">{{ t("resource.backup") }}</button>
              <button class="mini-btn" @click="openFolder('saves', item.dir)">{{ t("resource.openFolder") }}</button>
              <button class="mini-btn danger" @click="deleteSaveAsk(item)">{{ t("resource.delete") }}</button>
            </div>
          </div>
          <div v-if="!saves.length" class="empty-tip">{{ t("resource.empty") }}</div>
        </div>

        <!-- 截图（网格 + 大图预览） -->
        <div v-else-if="category === 'screenshots'" class="shot-list">
          <div v-if="shots.length" class="shot-grid">
            <div v-for="item in shots" :key="item.name" class="shot-cell">
              <AsyncImage class="shot-img" :src="shotUrl(item)" :alt="item.name" :title="item.name" @click="preview = item" />
              <button class="shot-del" :title="t('resource.delete')" @click="deleteShotAsk(item)">×</button>
            </div>
          </div>
          <div v-else class="empty-tip">{{ t("resource.empty") }}</div>
        </div>

        <!-- 服务器列表（servers.dat） -->
        <div v-else-if="category === 'servers'" class="item-list">
          <div v-for="item in servers" :key="`${item.name}|${item.ip}`" class="item-row">
            <img v-if="item.icon" class="item-icon" :src="item.icon" alt="" />
            <span v-else class="item-icon item-icon-empty">S</span>
            <div class="item-main">
              <div class="item-name-line">
                <span class="item-name">{{ item.name }}</span>
                <span v-if="item.acceptTextures" class="badge badge-dim">{{ t("resource.acceptTextures") }}</span>
              </div>
              <span class="item-sub">{{ item.ip }}</span>
            </div>
            <div class="item-actions">
              <button class="mini-btn" @click="openServerForm(item)">{{ t("resource.serverEdit") }}</button>
              <button class="mini-btn" @click="openFolder('servers', null)">{{ t("resource.openFolder") }}</button>
              <button class="mini-btn danger" @click="deleteServerAsk(item)">{{ t("resource.delete") }}</button>
            </div>
          </div>
          <div v-if="!servers.length" class="empty-tip">{{ t("resource.empty") }}</div>
        </div>

        <!-- 光影包列表 -->
        <div v-else-if="category === 'shaders'" class="item-list">
          <div v-for="item in shaders" :key="item.file" class="item-row">
            <span class="item-icon item-icon-empty">G</span>
            <div class="item-main">
              <div class="item-name-line">
                <span class="item-name">{{ item.name || item.file }}</span>
                <span v-if="item.selected" class="badge">{{ t("resource.shaderOn") }}</span>
              </div>
              <span class="item-sub">
                {{ [item.comment, item.file].filter(Boolean).join(" · ") }}
              </span>
            </div>
            <div class="item-actions">
              <button class="mini-btn" @click="toggleShader(item)">
                {{ item.selected ? t("resource.disable") : t("resource.enable") }}
              </button>
              <button class="mini-btn" @click="openFolder('shaderpacks', item.file)">{{ t("resource.openFolder") }}</button>
              <button class="mini-btn danger" @click="deleteShaderAsk(item)">{{ t("resource.delete") }}</button>
            </div>
          </div>
          <div v-if="!shaders.length" class="empty-tip">{{ t("resource.empty") }}</div>
        </div>

        <!-- 结构文件列表 -->
        <div v-else-if="category === 'schematics'" class="item-list">
          <div v-for="item in schematics" :key="item.file" class="item-row">
            <span class="item-icon item-icon-empty">B</span>
            <div class="item-main">
              <div class="item-name-line">
                <span class="item-name">{{ item.name || item.file }}</span>
                <span class="badge badge-dim">{{ item.typeName }}</span>
                <span v-if="item.fail" class="badge badge-red">{{ t("resource.modFail") }}</span>
              </div>
              <span class="item-sub">
                {{ [item.author, t("resource.dims", { w: item.width, h: item.height, l: item.length }), t("resource.blockCount", { count: item.blockCount })].filter(Boolean).join(" · ") }}
              </span>
            </div>
            <div class="item-actions">
              <button class="mini-btn" @click="openFolder('schematics', item.file)">{{ t("resource.openFolder") }}</button>
              <button class="mini-btn danger" @click="deleteSchematicAsk(item)">{{ t("resource.delete") }}</button>
            </div>
          </div>
          <div v-if="!schematics.length" class="empty-tip">{{ t("resource.empty") }}</div>
        </div>

        <!-- 数据包子页：存档下拉 + 数据包列表 -->
        <div v-else-if="category === 'saves'" class="item-list">
          <select v-model="dpSave" class="dp-select">
            <option value="" disabled>{{ t("resource.selectSave") }}</option>
            <option v-for="s in saves" :key="s.dir" :value="s.dir">
              {{ s.levelName || s.dir }}
            </option>
          </select>

          <template v-if="dpSave">
            <div v-for="item in datapacks" :key="item.name" class="item-row">
              <span class="item-icon item-icon-empty">D</span>
              <div class="item-main">
                <div class="item-name-line">
                  <span class="item-name">{{ dpName(item) }}</span>
                  <span
                    class="badge"
                    :class="{ 'badge-red': item.enable === false }"
                  >{{ dpState(item) }}</span>
                </div>
                <span class="item-sub">
                  {{ [item.description, t("resource.packFormat", { format: item.packFormat })].filter(Boolean).join(" · ") }}
                </span>
              </div>
              <div class="item-actions">
                <button class="mini-btn" @click="toggleDataPack(item)">
                  {{ item.enable === false ? t("resource.enable") : t("resource.disable") }}
                </button>
                <button class="mini-btn" @click="openFolder('datapacks', null, dpSave)">{{ t("resource.openFolder") }}</button>
                <button class="mini-btn danger" @click="deleteDataPackAsk(item)">{{ t("resource.delete") }}</button>
              </div>
            </div>
            <div v-if="!datapacks.length && !loading" class="empty-tip">{{ t("resource.empty") }}</div>
          </template>
          <div v-else class="empty-tip">{{ t("resource.selectSave") }}</div>
        </div>

        <!-- 无实例 -->
        <div v-if="!instance" class="empty-tip">{{ t("resource.notSelected") }}</div>
      </section>
    </div>

    <!-- 删除 / 清空确认 -->
    <BaseModal
      v-if="confirmBox"
      :title="confirmBox.title"
      :closable="!confirmBusy"
      @close="confirmBox = null"
    >
      <p class="confirm-text">{{ confirmBox.text }}</p>
      <div class="modal-actions">
        <button class="modal-btn" :disabled="confirmBusy" @click="confirmBox = null">{{ t("resource.cancel") }}</button>
        <button class="modal-btn danger" :disabled="confirmBusy" @click="runConfirm">
          {{ confirmBusy ? t("actions.deleting") : t("actions.confirm") }}
        </button>
      </div>
    </BaseModal>

    <!-- 截图大图预览 -->
    <BaseModal v-if="preview" :title="preview.name" :width="720" @close="preview = null">
      <div class="preview-body">
        <img class="preview-img" :src="shotUrl(preview)" :alt="preview.name" />
      </div>
      <div class="modal-actions">
        <button class="modal-btn" @click="openFolder('screenshots', preview.name)">{{ t("resource.openFolder") }}</button>
        <button class="modal-btn danger" @click="deleteShotAsk(preview)">{{ t("resource.delete") }}</button>
      </div>
    </BaseModal>

    <!-- 服务器添加 / 编辑表单 -->
    <BaseModal
      v-if="serverForm"
      :title="serverForm.edit ? t('resource.serverEdit') : t('resource.serverAdd')"
      @close="serverForm = null"
    >
      <div class="form-grid">
        <label class="form-label">{{ t("resource.serverName") }}</label>
        <input v-model="serverForm.name" class="form-input" type="text" />
        <label class="form-label">{{ t("resource.serverIp") }}</label>
        <input v-model="serverForm.ip" class="form-input" type="text" />
        <template v-if="serverForm.edit">
          <label class="form-label">{{ t("resource.acceptTextures") }}</label>
          <input v-model="serverForm.acceptTextures" type="checkbox" />
        </template>
      </div>
      <div class="modal-actions">
        <button class="modal-btn" :disabled="serverBusy" @click="serverForm = null">{{ t("resource.cancel") }}</button>
        <button class="modal-btn danger" :disabled="serverBusy" @click="saveServerForm">
          {{ serverBusy ? t("actions.deleting") : t("resource.save") }}
        </button>
      </div>
    </BaseModal>
  </WindowFrame>
</template>

<style scoped>
.resource-layout {
  display: flex;
  gap: 18px;
  height: 100%;
}

.cat-nav {
  width: 190px;
  min-width: 190px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.cat-instance {
  display: flex;
  flex-direction: column;
  gap: 2px;
  padding: 10px 12px;
  margin-bottom: 8px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
}

.cat-inst-name {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.cat-inst-sub {
  font-size: 11px;
  color: var(--text-dim);
}

.cat-item {
  text-align: left;
  padding: 10px 12px;
  border: none;
  border-radius: 9px;
  background: transparent;
  color: var(--text-dim);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.12s;
}

.cat-item:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.cat-item.active {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

.cat-content {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.content-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}

.head-title {
  font-size: 15px;
  font-weight: 700;
  padding: 4px 0;
}

.head-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.item-list {
  flex: 1;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  gap: 8px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 12px;
}

.item-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 9px;
  background: var(--bg-side);
  border: 1px solid var(--border);
}

.item-icon {
  width: 36px;
  height: 36px;
  border-radius: 7px;
  object-fit: cover;
  flex-shrink: 0;
  background: var(--bg-hover);
}

.item-icon-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 15px;
  font-weight: 700;
  color: var(--text-dim);
}

.item-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.item-name-line {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}

.item-name {
  font-size: 13px;
  color: var(--text);
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.item-sub {
  font-size: 11px;
  color: var(--text-dim);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.badge {
  flex-shrink: 0;
  padding: 1px 7px;
  border-radius: 6px;
  font-size: 10px;
  background: var(--accent-soft);
  color: var(--accent);
}

.badge-dim {
  background: var(--bg-hover);
  color: var(--text-dim);
}

.badge-red {
  background: rgba(220, 38, 38, 0.14);
  color: var(--red);
}

.item-actions {
  display: flex;
  gap: 6px;
  flex-shrink: 0;
}

.mini-btn {
  padding: 6px 12px;
  border-radius: 7px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.12s;
}

.mini-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
}

.mini-btn.danger:hover {
  color: var(--red);
  border-color: var(--red);
}

.mini-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

/* 截图网格 */
.shot-list {
  flex: 1;
  overflow-y: auto;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 12px;
}

.shot-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 10px;
}

.shot-cell {
  position: relative;
  border-radius: 9px;
  overflow: hidden;
  border: 1px solid var(--border);
  background: var(--bg-side);
}

.shot-img {
  display: block;
  width: 100%;
  aspect-ratio: 16 / 9;
  object-fit: cover;
  cursor: zoom-in;
}

.shot-del {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 22px;
  height: 22px;
  border: none;
  border-radius: 6px;
  background: rgba(0, 0, 0, 0.55);
  color: #fff;
  font-size: 14px;
  line-height: 1;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.12s;
}

.shot-cell:hover .shot-del {
  opacity: 1;
}

.shot-del:hover {
  background: var(--red);
}

/* 确认弹窗 */
.confirm-text {
  margin: 0;
  font-size: 13px;
  color: var(--text);
  line-height: 1.6;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 14px;
}

.modal-btn {
  padding: 8px 16px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.12s;
}

.modal-btn:hover {
  background: var(--bg-hover);
}

.modal-btn.danger {
  background: var(--red);
  border-color: var(--red);
  color: #fff;
}

.modal-btn.danger:hover {
  opacity: 0.9;
}

/* 数据包子页的存档下拉 */
.dp-select {
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-side);
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  cursor: pointer;
}

.dp-select:focus {
  outline: none;
  border-color: var(--accent);
}

/* 服务器表单 */
.form-grid {
  display: grid;
  grid-template-columns: auto 1fr;
  align-items: center;
  gap: 10px 12px;
}

.form-label {
  font-size: 12px;
  color: var(--text-dim);
}

.form-input {
  padding: 8px 10px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-side);
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  min-width: 0;
}

.form-input:focus {
  outline: none;
  border-color: var(--accent);
}

/* 大图预览 */
.preview-body {
  display: flex;
  justify-content: center;
  background: var(--bg-side);
  border-radius: 9px;
  padding: 8px;
}

.preview-img {
  max-width: 100%;
  max-height: 60vh;
  object-fit: contain;
  border-radius: 6px;
}
</style>
