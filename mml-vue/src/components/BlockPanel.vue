<script setup lang="ts">
// 方块列表：左侧分类栏（带数量）+ 右侧网格（搜索 / 分类过滤）
// 点击方块弹详情（大图预览 + 设为实例图标 / 删除皮肤方块），不再点击即改图标
// 渲染状态驱动视图：渲染中在顶部占一行进度（下方列表照常可用），
// 已渲染 → 工具条 + 分类栏 + 网格，未渲染 → 提示卡
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { locale, t, tErr } from "../lib/i18n";
import { showToast } from "../lib/toast";
import type { BlockItemDto, BlockStatusDto, InstanceInfoDto } from "../lib/bindings";
import AsyncImage from "./ui/AsyncImage.vue";
import BaseButton from "./ui/BaseButton.vue";
import BaseModal from "./ui/BaseModal.vue";
import InstanceIcon from "./InstanceIcon.vue";
import { api, blockRenderCancel, blockRenderStart, blockSetIcon, blockSkinAdd, blockSkinRemove, getBlockList, getBlockStatus, onBlockRender, onDownloadTask } from "../lib/api";
import { openWindow } from "../windows/windowManager";

defineProps<{
  currentInstance: InstanceInfoDto | null;
}>();

const status = ref<BlockStatusDto | null>(null);
const unlisten: Array<() => void> = [];

/** 本轮渲染是否已弹过下载窗口（渲染结束复位，下一轮再触发） */
let downloadWinOpened = false;
/** 上一次见到的渲染错误（相同错误只弹一次） */
let lastError: string | null = null;

onMounted(async () => {
  try {
    status.value = await getBlockStatus();
  } catch {
    status.value = null;
  }
  // 渲染已在跑且还没开始步进（下载阶段）：若有活跃下载任务，补开下载窗口。
  // 任务的 add 事件可能发生在本窗口打开前，事件弹窗路径会漏
  if (status.value?.running && (status.value.now ?? 0) === 0) {
    try {
      const ds = await api.getDownloadStatus();
      if (ds.tasks.length > 0 && !downloadWinOpened) {
        downloadWinOpened = true;
        openWindow("download");
      }
    } catch {
      // 状态查不到就算了，不影响主流程
    }
  }
  unlisten.push(await onBlockRender((e) => {
    status.value = e;
    // 渲染刚结束（且已有结果）：刷新列表（首次渲染完成时列表还没拉过）
    if (!e.running && e.rendered) void loadBlocks();
    if (!e.running) {
      downloadWinOpened = false;
      cancelling.value = false;
    }
    // 渲染失败主动弹提示：重新渲染失败时界面停在网格视图，错误没有落点，
    // 只有 toast 能让用户知道（首次渲染失败另有提示卡显示详情）
    if (e.error && e.error !== lastError) {
      showToast(e.error, 4000);
    }
    lastError = e.error ?? null;
  }));
  // 首次渲染要下载游戏核心 jar：渲染中收到新下载任务就打开下载窗口，
  // 否则下载在后台静默进行，用户只看到 0 / 0 的渲染进度无从得知。
  // 事件可能早于本地状态（本轮从别处触发），running 以现查为准
  unlisten.push(
    await onDownloadTask(async (e) => {
      if (e.type !== "add") return;
      let st = status.value;
      if (!st?.running) {
        try {
          st = await getBlockStatus();
          status.value = st;
        } catch {
          return;
        }
      }
      if (st?.running && !downloadWinOpened) {
        downloadWinOpened = true;
        openWindow("download");
      }
    }),
  );
});

onUnmounted(() => unlisten.forEach((fn) => fn()));

// ---------- 三态视图 ----------

const rendered = computed(() => !!status.value?.rendered);
const running = computed(() => !!status.value?.running);
/** 渲染前置阶段（拉清单 / 下载核心 jar）：进度还没开始步进，显示下载文案而非 0 / 0 */
const preparing = computed(() => running.value && (status.value?.now ?? 0) === 0);
/** 渲染进度百分比（前置阶段未知，显示 0） */
const percent = computed(() => {
  const total = status.value?.total ?? 0;
  return total > 0 ? Math.min(100, ((status.value?.now ?? 0) / total) * 100) : 0;
});

/** 开始渲染（首渲染；重试同路径） */
async function startRender(force: boolean) {
  try {
    const started = await blockRenderStart(force);
    if (!started) showToast(t("blocks.rendering"));
  } catch (e) {
    showToast(tErr(e));
  }
}

/** 已发出取消、还没收到渲染结束事件（渲染循环要几毫秒才收手，期间禁用按钮防连点） */
const cancelling = ref(false);

/** 取消本轮渲染：内核侧协作式取消，随后会推一次 running=false 的状态 */
async function cancelRender() {
  if (cancelling.value) return;
  cancelling.value = true;
  try {
    // 返回 false = 没有在跑的渲染（刚好结束），不会有后续状态事件，就地复位
    if (await blockRenderCancel()) {
      showToast(t("blocks.renderCancelled"));
    } else {
      cancelling.value = false;
    }
  } catch (e) {
    cancelling.value = false;
    showToast(tErr(e));
  }
}

// ---------- 列表 ----------

const blocks = ref<BlockItemDto[]>([]);

async function loadBlocks() {
  try {
    blocks.value = await getBlockList(locale.value);
  } catch {
    blocks.value = [];
  }
}

watch(locale, () => void loadBlocks());

// 已渲染但列表还没拉（如渲染结束事件刚到）时拉一次
watch(rendered, (val) => {
  if (val && blocks.value.length === 0) void loadBlocks();
});

onMounted(() => {
  if (status.value?.rendered) void loadBlocks();
});

/** 搜索关键字（匹配 id / 显示名 / 分类名） */
const keyword = ref("");
/** 当前分类（"" = 全部） */
const cat = ref("");

/** 出现过的分类（保持后端排序），原版分组后端已按游戏语言翻译，自定义分组走前端键 */
const cats = computed(() => [...new Set(blocks.value.map((b) => b.cat).filter(Boolean))]);

/** 各分类条目数（一次聚合，供左侧分类栏显示） */
const catCounts = computed(() => {
  const map = new Map<string, number>();
  for (const b of blocks.value) {
    if (b.cat) map.set(b.cat, (map.get(b.cat) ?? 0) + 1);
  }
  return map;
});

// 当前分类消失（重渲染换版本 / 删掉最后一个皮肤方块）时回到「全部」
watch(cats, (list) => {
  if (cat.value && !list.includes(cat.value)) cat.value = "";
});

function catLabel(c: string): string {
  const key = `blocks.cat.${c}`;
  const text = t(key);
  // t() miss 时返回 key 本身，此时回退原文（原版分组即游戏语言翻译结果）
  return text === key ? c : text;
}

const filtered = computed(() => {
  const kw = keyword.value.trim().toLowerCase();
  return blocks.value.filter((b) => {
    if (cat.value && b.cat !== cat.value) return false;
    if (!kw) return true;
    return (
      b.id.toLowerCase().includes(kw) ||
      b.name.toLowerCase().includes(kw) ||
      catLabel(b.cat).toLowerCase().includes(kw)
    );
  });
});

// ---------- 方块详情 ----------

/** 详情弹窗当前展示的方块（null = 不显示） */
const detail = ref<BlockItemDto | null>(null);

function openDetail(b: BlockItemDto) {
  detail.value = b;
}

// ---------- 设为实例图标（先选实例） ----------

/** 实例选择弹窗是否打开 */
const iconPick = ref(false);
/** 实例列表（首次打开弹窗时拉一次） */
const instances = ref<InstanceInfoDto[]>([]);
/** 正在设图标（防连点） */
const iconBusy = ref(false);

/** 详情弹窗里点「设为实例图标」：弹出实例选择 */
async function openIconPick() {
  if (!detail.value) return;
  iconPick.value = true;
  if (instances.value.length) return;
  try {
    instances.value = await api.getInstances();
  } catch {
    instances.value = [];
  }
}

/** 把当前详情方块设为所选实例的图标 */
async function setIconFor(inst: InstanceInfoDto) {
  const b = detail.value;
  if (!b || iconBusy.value) return;
  iconBusy.value = true;
  try {
    await blockSetIcon(inst.uuid, b.id);
    showToast(t("blocks.setIconOk", { name: inst.name }));
    iconPick.value = false;
    detail.value = null;
  } catch (e) {
    showToast(tErr(e));
  } finally {
    iconBusy.value = false;
  }
}

// ---------- 皮肤方块 ----------

/** 皮肤方块分组（与 Rust 侧 SKIN_CAT 一致），ID 形如 custom:<名字> */
const SKIN_CAT = "playerSkin";

const skinOpen = ref(false);
const skinInput = ref("");
const skinBusy = ref(false);

function openSkinDialog() {
  skinInput.value = "";
  skinOpen.value = true;
}

/** 按用户名或UUID添加（同名覆盖），成功后刷新列表 */
async function addSkin() {
  const input = skinInput.value.trim();
  if (!input || skinBusy.value) return;
  skinBusy.value = true;
  try {
    await blockSkinAdd(input);
    showToast(t("blocks.skinAddOk"));
    skinOpen.value = false;
    await loadBlocks();
  } catch (e) {
    showToast(tErr(e));
  } finally {
    skinBusy.value = false;
  }
}

/** 删除皮肤方块（名字不含 custom: 前缀） */
async function removeSkin(b: BlockItemDto) {
  try {
    await blockSkinRemove(b.id.slice("custom:".length));
    showToast(t("blocks.skinRemoveOk"));
    if (detail.value?.id === b.id) detail.value = null;
    await loadBlocks();
  } catch (e) {
    showToast(tErr(e));
  }
}
</script>

<template>
  <div class="block-panel">
    <!-- 渲染中：顶部一行进度（不挤掉下方内容，取消后立即消失） -->
    <div v-if="running" class="block-progress">
      <span class="block-progress-text">
        {{ status?.text || (preparing ? t("blocks.downloadingCore") : t("blocks.rendering")) }}
      </span>
      <div class="block-progress-bar">
        <div class="block-progress-fill" :style="{ width: percent + '%' }"></div>
      </div>
      <span class="block-progress-percent">{{ preparing ? "—" : percent.toFixed(1) + "%" }}</span>
      <span v-if="!preparing" class="block-progress-num">
        {{ status?.now ?? 0 }} / {{ status?.total ?? 0 }}
      </span>
      <BaseButton size="sm" :disabled="cancelling" @click="cancelRender">
        {{ cancelling ? t("blocks.cancelling") : t("blocks.cancelRender") }}
      </BaseButton>
    </div>

    <!-- 已渲染：工具条（渲染中仍可用，仅「重新渲染」禁用） -->
    <div v-if="rendered" class="block-top">
      <input
        v-model="keyword"
        class="field-input block-search"
        type="text"
        :placeholder="t('blocks.search')"
        @keydown.esc="keyword = ''"
      />
      <BaseButton @click="openSkinDialog">{{ t("blocks.addSkin") }}</BaseButton>
      <BaseButton :disabled="running" @click="startRender(true)">{{ t("blocks.reRender") }}</BaseButton>
      <span class="block-count">{{ t("blocks.count", { n: filtered.length }) }}</span>
    </div>

    <!-- 已渲染：分类栏 + 网格 -->
    <div v-if="rendered" class="block-main">
      <!-- 分类栏：全部 + 各分类条目数 -->
      <aside class="cat-rail">
        <button
          class="cat-item"
          :class="{ on: cat === '' }"
          @click="cat = ''"
        >
          <span class="cat-name">{{ t("blocks.catAll") }}</span>
          <span class="cat-num">{{ blocks.length }}</span>
        </button>
        <button
          v-for="c in cats"
          :key="c"
          class="cat-item"
          :class="{ on: cat === c }"
          @click="cat = c"
        >
          <span class="cat-name">{{ catLabel(c) }}</span>
          <span class="cat-num">{{ catCounts.get(c) ?? 0 }}</span>
        </button>
        <div v-if="status?.version" class="cat-foot">
          {{ t("blocks.version", { v: status.version }) }}
        </div>
      </aside>

      <!-- 网格 -->
      <div class="block-grid-wrap">
        <div v-if="filtered.length" class="block-grid">
          <button
            v-for="b in filtered"
            :key="b.id"
            class="block-cell"
            v-tip="b.id"
            @click="openDetail(b)"
          >
            <!-- 皮肤方块：悬停角标删除 -->
            <span
              v-if="b.cat === SKIN_CAT"
              class="block-del"
              v-tip="t('blocks.skinRemove')"
              @click.stop="removeSkin(b)"
            >✕</span>
            <AsyncImage class="block-img" :src="b.image" :alt="b.name" />
            <span class="block-name">{{ b.name }}</span>
          </button>
        </div>
        <div v-else class="empty-tip">{{ t("blocks.empty") }}</div>
      </div>
    </div>

    <!-- 未渲染：提示卡（渲染中显示进度说明，不再给按钮） -->
    <div v-if="!rendered" class="block-prompt">
      <div class="block-prompt-icon">
        <svg viewBox="0 0 24 24" width="40" height="40" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 3 4.5 7.5v9L12 21l7.5-4.5v-9L12 3z" />
          <path d="M4.5 7.5 12 12l7.5-4.5" />
          <path d="M12 12v9" />
        </svg>
      </div>
      <h2 class="block-prompt-title">
        {{ status?.error ? t("blocks.renderFailed") : running ? t("blocks.rendering") : t("blocks.renderPromptTitle") }}
      </h2>
      <p class="block-prompt-desc">
        {{ status?.error ?? (running ? t("blocks.renderFirstHint") : t("blocks.renderPromptDesc")) }}
      </p>
      <p v-if="!status?.error && !running" class="block-prompt-hint">{{ t("blocks.renderSizeHint") }}</p>
      <BaseButton v-if="!running" variant="primary" @click="startRender(false)">
        {{ status?.error ? t("blocks.retry") : t("blocks.renderNow") }}
      </BaseButton>
    </div>

    <!-- 添加皮肤方块（用户名或UUID） -->
    <BaseModal v-if="skinOpen" :title="t('blocks.addSkin')" :closable="false" @close="!skinBusy && (skinOpen = false)">
      <label class="field-label">{{ t("blocks.skinInput") }}</label>
      <input
        v-model="skinInput"
        class="field-input"
        :placeholder="t('blocks.skinInputHint')"
        spellcheck="false"
        @keyup.enter="addSkin"
      />

      <div class="modal-actions">
        <BaseButton :disabled="skinBusy" @click="skinOpen = false">
          {{ t("blocks.cancel") }}
        </BaseButton>
        <BaseButton variant="primary" :disabled="skinBusy || !skinInput.trim()" @click="addSkin">
          {{ skinBusy ? t("blocks.skinAdding") : t("blocks.addSkin") }}
        </BaseButton>
      </div>
    </BaseModal>

    <!-- 方块详情：大图预览 + ID / 分类 + 设为实例图标
         （选实例弹窗打开时先让位，避免两层遮罩叠着） -->
    <BaseModal v-if="detail && !iconPick" :title="detail.name" :width="470" :closable="false" @close="detail = null">
      <div class="detail-body">
        <div class="detail-stage">
          <AsyncImage class="detail-img" :src="detail.image" :alt="detail.name" />
        </div>
        <div class="detail-info">
          <div class="detail-row">
            <span class="detail-label">{{ t("blocks.detailId") }}</span>
            <span class="detail-value detail-mono">{{ detail.id }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">{{ t("blocks.detailCat") }}</span>
            <span class="detail-value">{{ catLabel(detail.cat) }}</span>
          </div>
        </div>
      </div>

      <div class="modal-actions">
        <BaseButton
          v-if="detail.cat === SKIN_CAT"
          variant="danger"
          @click="removeSkin(detail)"
        >
          {{ t("blocks.skinRemove") }}
        </BaseButton>
        <BaseButton @click="detail = null">{{ t("blocks.close") }}</BaseButton>
        <BaseButton variant="primary" @click="openIconPick">
          {{ t("blocks.setIcon") }}
        </BaseButton>
      </div>
    </BaseModal>

    <!-- 设为实例图标：选哪个实例（当前实例高亮，点条目即设） -->
    <BaseModal v-if="iconPick" :title="t('blocks.pickInstance')" :width="430" @close="iconPick = false">
      <p class="pick-desc">{{ t("blocks.pickInstanceDesc") }}</p>

      <div v-if="instances.length" class="pick-list">
        <button
          v-for="inst in instances"
          :key="inst.uuid"
          class="pick-item"
          :class="{ on: inst.uuid === currentInstance?.uuid }"
          :disabled="iconBusy"
          @click="setIconFor(inst)"
        >
          <InstanceIcon :name="inst.name" :uuid="inst.uuid" :size="30" />
          <span class="pick-text">
            <span class="pick-name">{{ inst.name }}</span>
            <span class="pick-sub">{{ inst.version }}</span>
          </span>
          <span v-if="inst.uuid === currentInstance?.uuid" class="pick-cur">
            {{ t("blocks.currentInstance") }}
          </span>
        </button>
      </div>
      <div v-else class="empty-tip">{{ t("blocks.noInstances") }}</div>
    </BaseModal>
  </div>
</template>

<style scoped>
.block-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  flex: 1;
  min-height: 0;
}

/* ---------- 提示卡（未渲染 / 失败） ---------- */
.block-prompt {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 30px 20px 26px;
  border: 1px dashed var(--accent-border);
  border-radius: 16px;
  background: var(--bg-card);
}

.block-prompt-icon {
  width: 84px;
  height: 84px;
  border-radius: 24px;
  background: var(--accent-soft);
  color: var(--accent);
  display: flex;
  align-items: center;
  justify-content: center;
  margin-bottom: 8px;
}

.block-prompt-title {
  font-size: 17px;
  font-weight: 700;
}

.block-prompt-desc {
  font-size: 13px;
  color: var(--text-dim);
  text-align: center;
  max-width: 420px;
}

.block-prompt-hint {
  font-size: 12px;
  color: var(--text-dim);
  opacity: 0.75;
  text-align: center;
  max-width: 420px;
  margin-bottom: 8px;
}

/* ---------- 进度（渲染中，顶部一行） ---------- */
.block-progress {
  flex-shrink: 0;
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
  font-size: 12.5px;
}

.block-progress-text {
  flex-shrink: 0;
  max-width: 45%;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text);
}

/* 进度条占满中段，两端文字按内容宽度 */
.block-progress-bar {
  flex: 1;
  min-width: 60px;
  height: 6px;
  border-radius: 999px;
  background: var(--bg-side);
  overflow: hidden;
}

.block-progress-fill {
  height: 100%;
  border-radius: 999px;
  background: var(--accent);
  transition: width 0.2s ease;
}

.block-progress-percent {
  flex-shrink: 0;
  font-variant-numeric: tabular-nums;
  font-weight: 600;
  color: var(--accent);
}

.block-progress-num {
  flex-shrink: 0;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

/* ---------- 工具条 ---------- */
.block-top {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-shrink: 0;
}

/* 压到与工具条按钮同高（BaseButton md = 35px）：.field-input 默认 42px，
   两倍类名提高优先级，避免依赖样式注入顺序 */
.block-top .block-search {
  flex: 1;
  min-width: 0;
  height: 35px;
  min-height: 0;
  padding: 0 12px;
  font-size: 13px;
}

.block-count {
  margin-left: auto;
  flex-shrink: 0;
  font-size: 12px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

/* ---------- 主体：分类栏 + 网格 ---------- */
.block-main {
  flex: 1;
  min-height: 0;
  display: flex;
  gap: 16px;
}

.cat-rail {
  width: 168px;
  min-width: 168px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-y: auto;
  padding-right: 2px;
}

.cat-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 8px;
  padding: 8px 11px;
  border: none;
  border-radius: 9px;
  background: transparent;
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: all 0.12s;
}

.cat-item:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.cat-item.on {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

.cat-name {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.cat-num {
  flex-shrink: 0;
  font-size: 11.5px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.cat-item.on .cat-num {
  color: var(--accent);
}

.cat-foot {
  margin-top: auto;
  padding: 10px 11px 2px;
  font-size: 11.5px;
  color: var(--text-dim);
}

/* ---------- 网格 ---------- */
.block-grid-wrap {
  flex: 1;
  min-width: 0;
  min-height: 0;
  overflow-y: auto;
  padding-right: 2px;
}

.block-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(96px, 1fr));
  gap: 10px;
  padding: 2px;
}

.block-cell {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 10px 6px 8px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
  cursor: pointer;
  transition: border-color 0.15s, transform 0.15s;
  content-visibility: auto;
  position: relative;
}

/* 皮肤方块的删除角标（悬停显示） */
.block-del {
  position: absolute;
  top: 4px;
  right: 4px;
  width: 20px;
  height: 20px;
  display: none;
  align-items: center;
  justify-content: center;
  border-radius: 6px;
  background: var(--bg-hover);
  color: var(--text-dim);
  font-size: 11px;
  line-height: 1;
}

.block-cell:hover .block-del {
  display: flex;
}

.block-del:hover {
  background: var(--accent-soft);
  color: var(--accent);
}

.block-cell:hover {
  border-color: var(--accent);
  transform: translateY(-2px);
}

.block-img {
  width: 56px;
  height: 56px;
  border-radius: 8px;
  /* 贴图是小 PNG，不用平滑缩放的模糊感 */
  image-rendering: pixelated;
}

.block-name {
  width: 100%;
  font-size: 11.5px;
  color: var(--text);
  text-align: center;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* ---------- 选实例弹窗 ---------- */
.pick-desc {
  font-size: 12.5px;
  color: var(--text-dim);
  margin-bottom: 10px;
}

.pick-list {
  display: flex;
  flex-direction: column;
  gap: 4px;
  max-height: 320px;
  overflow-y: auto;
}

.pick-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 10px;
  border: 1px solid transparent;
  border-radius: 10px;
  background: transparent;
  color: var(--text);
  font-family: inherit;
  text-align: left;
  cursor: pointer;
  transition: all 0.12s;
}

.pick-item:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
}

.pick-item:disabled {
  cursor: default;
  opacity: 0.6;
}

.pick-item.on {
  background: var(--accent-soft);
  border-color: var(--accent-border);
}

.pick-text {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  line-height: 1.3;
}

.pick-name {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.pick-sub {
  font-size: 11.5px;
  color: var(--text-dim);
}

.pick-cur {
  flex-shrink: 0;
  font-size: 10px;
  padding: 0 6px;
  border-radius: 8px;
  background: var(--accent-soft);
  color: var(--accent);
  line-height: 1.7;
}

/* ---------- 详情弹窗 ---------- */
.detail-body {
  display: flex;
  gap: 16px;
  align-items: flex-start;
}

.detail-stage {
  flex-shrink: 0;
  width: 160px;
  height: 160px;
  display: flex;
  align-items: center;
  justify-content: center;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-side);
}

.detail-img {
  width: 132px;
  height: 132px;
  image-rendering: pixelated;
}

.detail-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding-top: 4px;
}

.detail-row {
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.detail-label {
  font-size: 11.5px;
  color: var(--text-dim);
}

.detail-value {
  font-size: 13px;
  color: var(--text);
  word-break: break-all;
}

.detail-mono {
  font-family: ui-monospace, Consolas, "Courier New", monospace;
  font-size: 12.5px;
}
</style>
