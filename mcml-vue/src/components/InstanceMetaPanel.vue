<script setup lang="ts">
// 实例元信息面板：版本类型+版本 / 加载器+加载器版本 / 游戏内语言+日志编码 / 整合包类型
// 改动经 update 事件走 IPC 写入核心实例配置；刷新按钮与添加实例窗口同逻辑
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { t } from "../lib/i18n";
import { api, onAddLoaderProgress } from "../lib/api";
import { showToast } from "../lib/toast";
import BaseButton from "./ui/BaseButton.vue";
import type { InstanceInfo, VersionInfo } from "../lib/types";

const props = defineProps<{
  instance: InstanceInfo;
  versions: VersionInfo[];
}>();

const emit = defineEmits<{
  (e: "update", patch: Partial<InstanceInfo>): void;
  (e: "refreshed", versions: VersionInfo[]): void;
}>();

function change(patch: Partial<InstanceInfo>) {
  emit("update", patch);
}

// 版本类型显示名（独立 ID 走 i18n：add.type.*）
function typeName(vt: string): string {
  return t(`add.type.${vt}`);
}

// 实例的版本类型粒度是 release / snapshot / other（旧版合并），
// 版本清单里旧版拆成 old_beta / old_alpha，统一归并成 other
function normType(vt: string): string {
  return vt === "old_beta" || vt === "old_alpha" ? "other" : vt;
}

// ================= 版本类型 + 版本 =================

const versionTypes = computed(() => {
  const set = new Set<string>();
  for (const v of props.versions) set.add(normType(v.versionType));
  return [...set];
});

const typeFilter = ref(normType(props.instance.versionType ?? "release"));

// 跟随实例（切换实例 / 更新后同步）
watch(
  () => props.instance.versionType,
  (v) => {
    typeFilter.value = normType(v ?? "release");
  },
);

const filteredVersions = computed(() =>
  props.versions.filter((v) => normType(v.versionType) === typeFilter.value),
);

function onVersionTypeChange(value: string) {
  typeFilter.value = value;
  // 当前版本不在该类型中时，切到该类型第一个版本
  const target = props.versions.find((v) => normType(v.versionType) === value);
  if (target && target.id !== props.instance.version) {
    change({ version: target.id, versionType: value });
  } else {
    change({ versionType: value });
  }
}

// 刷新版本列表（清空后端缓存重新拉取，版本类型 / 版本随列表一起更新）
const verLoading = ref(false);

async function onRefreshVersions() {
  verLoading.value = true;
  try {
    emit("refreshed", await api.refreshVersions());
    showToast(t("tip.refreshed"));
  } catch (e) {
    showToast(String(e));
  } finally {
    verLoading.value = false;
  }
}

// ================= 加载器类型 + 加载器版本 =================

// 加载器显示名（独立 ID 走 i18n：add.loader.*）
function loaderLabel(id: string): string {
  return t(`add.loader.${id}`);
}

// 默认只有原版 / 当前加载器 / 自定义；点刷新拉取该版本支持的加载器后合并进来
const fetchedLoaders = ref<string[]>([]);
const loadersLoading = ref(false);

const loaderOptions = computed(() => {
  const opts = ["normal"];
  const cur = props.instance.loader;
  if (cur && !opts.includes(cur)) opts.push(cur);
  for (const id of fetchedLoaders.value) {
    if (!opts.includes(id)) opts.push(id);
  }
  if (!opts.includes("custom")) opts.push("custom");
  return opts;
});

// 支持列表查询进度（事件为全局广播，仅本面板查询期间显示）
const progressStep = ref(0);
const progressTotal = ref(0);

let unlistenProgress: (() => void) | null = null;
onMounted(async () => {
  unlistenProgress = await onAddLoaderProgress((e) => {
    if (!loadersLoading.value) return;
    progressStep.value = e.step;
    progressTotal.value = e.total;
  });
});
onUnmounted(() => unlistenProgress?.());

async function onRefreshLoaders() {
  if (!props.instance.version) return;
  loadersLoading.value = true;
  progressStep.value = 0;
  try {
    fetchedLoaders.value = await api.addGetSupportLoaders(props.instance.version);
    showToast(t("tip.refreshed"));
  } catch (e) {
    showToast(String(e));
  } finally {
    loadersLoading.value = false;
  }
}

// 加载器版本默认锁死：点刷新按钮（或切换加载器）拉取真实列表后解锁
const loaderVersions = ref<string[]>([]);
const lvLocked = ref(true);
const lvLoading = ref(false);

// 锁死 / 列表为空时也显示当前版本，避免下拉显示空白
const lvOptions = computed(() => {
  const list = [...loaderVersions.value];
  const cur = props.instance.loaderVersion;
  if (cur && !list.includes(cur)) list.unshift(cur);
  return list;
});

/** 拉取指定加载器在当前版本下的版本列表（原版 / 自定义没有版本列表） */
async function fetchLoaderVersions(loaderId: string, notify = false) {
  if (!loaderId || loaderId === "normal" || loaderId === "custom") return;
  if (!props.instance.version) return;
  lvLoading.value = true;
  try {
    loaderVersions.value = await api.addLoaderVersions(loaderId, props.instance.version);
    lvLocked.value = false;
    if (notify) showToast(t("tip.refreshed"));
  } catch (e) {
    showToast(String(e));
  } finally {
    lvLoading.value = false;
  }
}

async function onRefreshLoaderVersions() {
  await fetchLoaderVersions(props.instance.loader, true);
}

// 自定义加载器：路径选择（与添加实例窗口一致，文件名存入加载器版本字段）
const loaderPathInput = ref<HTMLInputElement | null>(null);

function onLoaderPathPick(e: Event) {
  const input = e.target as HTMLInputElement;
  if (input.files?.[0]) change({ loaderVersion: input.files[0].name });
}

async function onLoaderChange(value: string) {
  // 切换加载器后重新锁死并自动拉取新加载器的版本列表
  lvLocked.value = true;
  loaderVersions.value = [];
  change({ loader: value, loaderVersion: null });
  await fetchLoaderVersions(value);
}

// ================= 游戏内语言 + 日志编码 =================

// 语言列表从实例的资源索引里查（minecraft/lang/*.json）；
// 资源未下载 / 查询失败时保留默认（简体中文 + English）
const DEFAULT_LANGS = [
  { value: "zh_cn", label: "简体中文" },
  { value: "en_us", label: "English" },
];
const langs = ref([...DEFAULT_LANGS]);

async function refreshLangs() {
  try {
    const list = await api.getInstanceLangs(props.instance.uuid);
    if (list.length) {
      langs.value = list.map((code) => ({ value: code, label: code }));
    }
  } catch {
    langs.value = [...DEFAULT_LANGS];
  }
}

// 切换实例：重置加载器查询状态并重新查询语言列表
watch(
  () => props.instance.uuid,
  () => {
    fetchedLoaders.value = [];
    loaderVersions.value = [];
    lvLocked.value = true;
    langs.value = [...DEFAULT_LANGS];
    refreshLangs();
  },
  { immediate: true },
);

// ================= 整合包类型（面板最底部） =================

const platforms = [
  { value: "", label: t("meta.platformNone") },
  { value: "curseforge", label: t("add.pack.curseforge") },
  { value: "modrinth", label: t("add.pack.modrinth") },
  { value: "serverpack", label: t("meta.platformOnline") },
];

function onPlatformChange(value: string) {
  // null 必须显式下发（JSON.stringify 会丢掉 undefined），后端才能区分“清空”
  if (!value) {
    change({ modpackType: null, pid: null, fid: null, serverUrl: null });
    return;
  }
  // 在线网络整合包用网址，其余平台用项目 / 文件 ID；切平台时清掉另一套字段
  if (value === "serverpack") {
    change({ modpackType: value, pid: null, fid: null });
  } else {
    change({ modpackType: value, serverUrl: null });
  }
}
</script>

<template>
  <div class="meta-panel">
    <!-- 加载器查询进度（窗口正上方浮动提示）：支持列表为步进进度条，加载器版本为滚动条 -->
    <Teleport to="body">
      <Transition name="load-pop">
        <div v-if="loadersLoading || lvLoading" class="load-float">
          <span class="load-float-spinner"></span>
          <span class="load-float-label">{{ loadersLoading ? t("add.loaderQuerying") : t("add.loaderVerLoading") }}</span>
          <div class="load-float-bar">
            <div
              v-if="loadersLoading"
              class="load-float-fill"
              :style="{ width: progressTotal ? (progressStep / progressTotal) * 100 + '%' : '0%' }"
            ></div>
            <div v-else class="load-float-indet"></div>
          </div>
          <span v-if="loadersLoading" class="load-float-text">{{ progressStep }} / {{ progressTotal }}</span>
        </div>
      </Transition>
    </Teleport>

    <!-- 版本类型 + 版本 -->
    <span class="meta-label">{{ t("meta.versionType") }}</span>
    <select
      class="field-select"
      :value="typeFilter"
      @change="onVersionTypeChange(($event.target as HTMLSelectElement).value)"
    >
      <option v-for="vt in versionTypes" :key="vt" :value="vt">{{ typeName(vt) }}</option>
    </select>
    <span class="meta-label small">{{ t("meta.version") }}</span>
    <div class="select-row">
      <select
        class="field-select"
        :value="instance.version"
        @change="change({ version: ($event.target as HTMLSelectElement).value })"
      >
        <optgroup :label="typeName(typeFilter)">
          <option v-for="v in filteredVersions" :key="v.id" :value="v.id">{{ v.id }}</option>
        </optgroup>
      </select>
      <BaseButton
        size="sm"
        variant="ghost"
        :disabled="verLoading"
        :title="t('add.versionRefresh')"
        @click="onRefreshVersions"
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 1 1-2.64-6.36" />
          <polyline points="21 3 21 9 15 9" />
        </svg>
      </BaseButton>
    </div>

    <!-- 加载器类型 + 加载器版本 -->
    <span class="meta-label">{{ t("meta.loader") }}</span>
    <div class="select-row">
      <!-- 默认只有原版 / 当前 / 自定义，刷新后合并该版本支持的加载器；查询期间锁住 -->
      <select
        class="field-select"
        :value="instance.loader"
        :disabled="loadersLoading"
        @change="onLoaderChange(($event.target as HTMLSelectElement).value)"
      >
        <option v-for="l in loaderOptions" :key="l" :value="l">{{ loaderLabel(l) }}</option>
      </select>
      <BaseButton
        size="sm"
        variant="ghost"
        :disabled="loadersLoading || !instance.version"
        :title="t('add.loaderRefresh')"
        @click="onRefreshLoaders"
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 1 1-2.64-6.36" />
          <polyline points="21 3 21 9 15 9" />
        </svg>
      </BaseButton>
    </div>
    <span class="meta-label small">{{ instance.loader === "custom" ? t("add.loaderPath") : t("meta.loaderVersion") }}</span>
    <!-- 自定义加载器：路径选择（文件名存入加载器版本字段），其余为版本下拉 -->
    <div v-if="instance.loader === 'custom'" class="select-row">
      <input
        class="field-input"
        :value="instance.loaderVersion ?? ''"
        :placeholder="t('add.loaderPathPlaceholder')"
        spellcheck="false"
        @input="change({ loaderVersion: ($event.target as HTMLInputElement).value || null })"
      />
      <BaseButton size="sm" variant="accent" @click="loaderPathInput?.click()">…</BaseButton>
      <input ref="loaderPathInput" type="file" accept=".jar" class="hidden-input" @change="onLoaderPathPick" />
    </div>
    <div v-else class="select-row">
      <!-- 默认锁死；点刷新或切换加载器后自动拉取真实版本列表并解锁 -->
      <select
        class="field-select"
        :value="instance.loaderVersion ?? ''"
        :disabled="lvLocked || lvLoading"
        @change="change({ loaderVersion: ($event.target as HTMLSelectElement).value || null })"
      >
        <option v-for="lv in lvOptions" :key="lv" :value="lv">{{ lv }}</option>
      </select>
      <BaseButton
        size="sm"
        variant="ghost"
        :disabled="lvLoading || !instance.version || instance.loader === 'normal'"
        :title="t('add.loaderVerRefresh')"
        @click="onRefreshLoaderVersions"
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 12a9 9 0 1 1-2.64-6.36" />
          <polyline points="21 3 21 9 15 9" />
        </svg>
      </BaseButton>
    </div>

    <!-- 游戏内语言 + 日志编码 -->
    <span class="meta-label">{{ t("meta.lang") }}</span>
    <select
      class="field-select"
      :value="instance.lang ?? 'zh_cn'"
      @change="change({ lang: ($event.target as HTMLSelectElement).value })"
    >
      <option v-for="l in langs" :key="l.value" :value="l.value">{{ l.label }}</option>
    </select>
    <span class="meta-label small">{{ t("meta.logEncoding") }}</span>
    <select
      class="field-select"
      :value="instance.logEncoding ?? 'utf8'"
      @change="change({ logEncoding: ($event.target as HTMLSelectElement).value })"
    >
      <option value="utf8">{{ t("meta.utf8") }}</option>
      <option value="gbk">{{ t("meta.gbk") }}</option>
    </select>

    <!-- 整合包类型（最底部，不占整行；ID 平台填项目 / 文件 ID，在线平台填网址） -->
    <span class="meta-label">{{ t("meta.modpack") }}</span>
    <select
      class="field-select"
      :value="instance.modpackType === 'none' ? '' : instance.modpackType ?? ''"
      @change="onPlatformChange(($event.target as HTMLSelectElement).value)"
    >
      <option v-for="p in platforms" :key="p.value" :value="p.value">{{ p.label }}</option>
    </select>
    <!-- 在线网络整合包：网址输入框（隐藏 ID 输入框） -->
    <div v-if="instance.modpackType === 'serverpack'" class="modpack-url">
      <input
        class="field-input"
        :value="instance.serverUrl ?? ''"
        :placeholder="t('add.url')"
        spellcheck="false"
        @input="change({ serverUrl: ($event.target as HTMLInputElement).value || null })"
      />
    </div>
    <!-- ID 平台：项目 / 文件 ID 等宽并排 -->
    <div v-else-if="instance.modpackType && instance.modpackType !== 'none'" class="modpack-ids">
      <input
        class="field-input"
        :value="instance.pid ?? ''"
        :placeholder="t('meta.pid')"
        spellcheck="false"
        @input="change({ pid: ($event.target as HTMLInputElement).value || null })"
      />
      <input
        class="field-input"
        :value="instance.fid ?? ''"
        :placeholder="t('meta.fid')"
        spellcheck="false"
        @input="change({ fid: ($event.target as HTMLInputElement).value || null })"
      />
    </div>
  </div>
</template>

<style scoped>
/* 4 列网格：标签 | 下拉 | 标签 | 下拉，保证各下拉框左对齐 */
.meta-panel {
  display: grid;
  grid-template-columns: 84px 1fr 64px 1fr;
  gap: 10px;
  align-items: center;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 14px 16px;
}

.meta-label {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-dim);
  white-space: nowrap;
}

.meta-label.small {
  text-align: right;
}

/* 加载器支持列表查询进度（窗口正上方浮动提示，Teleport 到 body，样式与添加实例窗口一致） */
.load-float {
  position: fixed;
  top: 14px;
  left: 0;
  right: 0;
  margin: 0 auto;
  width: fit-content;
  z-index: 500;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 16px;
  background: var(--bg-card);
  border: 1px solid var(--accent-border);
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  font-size: 12.5px;
  color: var(--text);
  pointer-events: none;
}

.load-float-spinner {
  width: 14px;
  height: 14px;
  border-radius: 50%;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  animation: load-float-spin 0.8s linear infinite;
  flex-shrink: 0;
}

@keyframes load-float-spin {
  to {
    transform: rotate(360deg);
  }
}

.load-float-label {
  color: var(--text-dim);
  white-space: nowrap;
}

.load-float-bar {
  width: 90px;
  height: 6px;
  border-radius: 3px;
  background: var(--border);
  overflow: hidden;
}

.load-float-fill {
  height: 100%;
  border-radius: 3px;
  background: var(--accent);
  transition: width 0.2s;
}

/* 不确定进度：小色块来回滚动（无步数可报的任务，如加载器版本拉取） */
.load-float-indet {
  height: 100%;
  width: 40%;
  border-radius: 3px;
  background: var(--accent);
  animation: load-float-slide 1.1s ease-in-out infinite;
}

@keyframes load-float-slide {
  from {
    transform: translateX(-100%);
  }
  to {
    transform: translateX(300%);
  }
}

.load-float-text {
  color: var(--text-dim);
  min-width: 30px;
  text-align: right;
}

.load-pop-enter-active,
.load-pop-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}

.load-pop-enter-from,
.load-pop-leave-to {
  opacity: 0;
  transform: translateY(-8px);
}

/* 下拉 + 刷新按钮（与添加实例窗口一致的排布） */
.select-row {
  display: flex;
  gap: 6px;
  align-items: center;
  min-width: 0;
}

.select-row .field-select {
  flex: 1;
  min-width: 0;
}

.select-row .field-input {
  flex: 1;
  min-width: 0;
}

/* 隐藏的文件选择框（自定义加载器路径） */
.hidden-input {
  display: none;
}

/* 项目 / 文件 ID 等宽并排（跨右侧两列） */
.modpack-ids {
  grid-column: 3 / 5;
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
  min-width: 0;
}

/* 在线网络整合包网址输入框 */
.modpack-url {
  grid-column: 3 / 5;
  display: flex;
  min-width: 0;
}

.modpack-url .field-input {
  flex: 1;
  min-width: 0;
}
</style>
