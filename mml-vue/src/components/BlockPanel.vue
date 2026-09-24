<script setup lang="ts">
// 主页方块列表：网格视图（搜索 / 分类 / 设为实例图标）
// 三态由渲染状态驱动：未渲染 → 提示卡；渲染中 → 进度条；已渲染 → 工具栏 + 网格
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { locale, t, tErr } from "../lib/i18n";
import { showToast } from "../lib/toast";
import type { BlockItemDto, BlockStatusDto, InstanceInfoDto } from "../lib/bindings";
import AsyncImage from "./ui/AsyncImage.vue";
import BaseButton from "./ui/BaseButton.vue";
import BaseModal from "./ui/BaseModal.vue";
import { blockRenderStart, blockSetIcon, blockSkinAdd, blockSkinRemove, getBlockList, getBlockStatus, onBlockRender } from "../lib/api";

const props = defineProps<{
  currentInstance: InstanceInfoDto | null;
}>();

const status = ref<BlockStatusDto | null>(null);
let unlisten: (() => void) | null = null;

onMounted(async () => {
  try {
    status.value = await getBlockStatus();
  } catch {
    status.value = null;
  }
  unlisten = await onBlockRender((e) => {
    status.value = e;
    // 渲染刚结束（且已有结果）：刷新列表（首次渲染完成时列表还没拉过）
    if (!e.running && e.rendered) void loadBlocks();
  });
});

onUnmounted(() => unlisten?.());

// ---------- 三态视图 ----------

const rendered = computed(() => !!status.value?.rendered);
const running = computed(() => !!status.value?.running);

/** 开始渲染（首渲染；重试同路径） */
async function startRender(force: boolean) {
  try {
    const started = await blockRenderStart(force);
    if (!started) showToast(t("blocks.rendering"));
  } catch (e) {
    showToast(String(e));
  }
}

// ---------- 网格 ----------

const blocks = ref<BlockItemDto[]>([]);

async function loadBlocks() {
  try {
    blocks.value = await getBlockList(locale.value);
  } catch {
    blocks.value = [];
  }
}

watch(locale, () => void loadBlocks());

// 已渲染但列表还没拉（如重启后 opt-in 自动补渲染完成）时拉一次
watch(rendered, (val) => {
  if (val && blocks.value.length === 0) void loadBlocks();
});

onMounted(() => {
  if (status.value?.rendered) void loadBlocks();
});

/** 搜索关键字（匹配 id / 显示名） */
const keyword = ref("");
/** 当前分类（"" = 全部） */
const cat = ref("");

/** 出现过的分类（保持后端排序），原版分组后端已按游戏语言翻译，自定义分组走前端键 */
const cats = computed(() => [...new Set(blocks.value.map((b) => b.cat).filter(Boolean))]);

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
      b.id.toLowerCase().includes(kw) || b.name.toLowerCase().includes(kw)
    );
  });
});

/** 点条目：设为当前实例图标 */
async function pick(b: BlockItemDto) {
  if (!props.currentInstance) {
    showToast(t("blocks.noInstance"));
    return;
  }
  try {
    await blockSetIcon(props.currentInstance.uuid, b.id);
    showToast(t("blocks.setIconOk"));
  } catch (e) {
    showToast(tErr(e));
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

/** 删除皮肤方块 */
async function removeSkin(b: BlockItemDto) {
  try {
    await blockSkinRemove(b.id.slice("custom:".length));
    showToast(t("blocks.skinRemoveOk"));
    await loadBlocks();
  } catch (e) {
    showToast(tErr(e));
  }
}
</script>

<template>
  <div class="block-panel">
    <!-- 未渲染且未在跑：提示卡（有错误则显示失败 + 重试） -->
    <div v-if="!rendered && !running" class="block-prompt">
      <div class="block-prompt-icon">
        <svg viewBox="0 0 24 24" width="40" height="40" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round">
          <path d="M12 3 4.5 7.5v9L12 21l7.5-4.5v-9L12 3z" />
          <path d="M4.5 7.5 12 12l7.5-4.5" />
          <path d="M12 12v9" />
        </svg>
      </div>
      <h2 class="block-prompt-title">
        {{ status?.error ? t("blocks.renderFailed") : t("blocks.renderPromptTitle") }}
      </h2>
      <p class="block-prompt-desc">{{ status?.error ?? t("blocks.renderPromptDesc") }}</p>
      <button class="block-prompt-btn" @click="startRender(false)">
        {{ status?.error ? t("blocks.retry") : t("blocks.renderNow") }}
      </button>
    </div>

    <!-- 渲染中：进度条 -->
    <div v-else-if="running" class="block-progress">
      <div class="block-progress-head">
        <span>{{ status?.text || t("blocks.rendering") }}</span>
        <span class="block-progress-num">{{ status?.now ?? 0 }} / {{ status?.total ?? 0 }}</span>
      </div>
      <div class="block-progress-bar">
        <div
          class="block-progress-fill"
          :style="{ width: status?.total ? ((status.now / status.total) * 100).toFixed(1) + '%' : '0%' }"
        ></div>
      </div>
    </div>

    <!-- 已渲染：工具栏 + 网格 -->
    <template v-else>
      <div class="block-toolbar">
        <input
          v-model="keyword"
          class="block-search"
          type="text"
          :placeholder="t('blocks.search')"
        />
        <button class="block-rerender" :title="t('blocks.addSkin')" @click="openSkinDialog">
          {{ t("blocks.addSkin") }}
        </button>
        <button class="block-rerender" :title="t('blocks.reRender')" @click="startRender(true)">
          {{ t("blocks.reRender") }}
        </button>
      </div>

      <div class="block-cats">
        <button
          class="block-cat"
          :class="{ on: cat === '' }"
          @click="cat = ''"
        >
          {{ t("blocks.catAll") }}
        </button>
        <button
          v-for="c in cats"
          :key="c"
          class="block-cat"
          :class="{ on: cat === c }"
          @click="cat = c"
        >
          {{ catLabel(c) }}
        </button>
      </div>

      <div v-if="filtered.length" class="block-grid">
        <button
          v-for="b in filtered"
          :key="b.id"
          class="block-cell"
          :title="b.id"
          @click="pick(b)"
        >
          <!-- 皮肤方块：悬停角标删除 -->
          <span
            v-if="b.cat === SKIN_CAT"
            class="block-del"
            :title="t('blocks.skinRemove')"
            @click.stop="removeSkin(b)"
          >✕</span>
          <AsyncImage class="block-img" :src="b.image" :alt="b.name" />
          <span class="block-name">{{ b.name }}</span>
        </button>
      </div>
      <div v-else class="block-empty">{{ t("blocks.empty") }}</div>

      <div class="block-count">{{ t("blocks.count", { n: filtered.length }) }}</div>
    </template>

    <!-- 添加皮肤方块（用户名或UUID） -->
    <BaseModal v-if="skinOpen" :title="t('blocks.addSkin')" @close="!skinBusy && (skinOpen = false)">
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
  </div>
</template>

<style scoped>
.block-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
}

/* 提示卡 */
.block-prompt {
  display: flex;
  flex-direction: column;
  align-items: center;
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

.block-prompt-btn {
  margin-top: 8px;
  padding: 9px 22px;
  border: none;
  border-radius: 10px;
  background: var(--accent);
  color: #fff;
  font-size: 13.5px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  transition: filter 0.15s;
}

.block-prompt-btn:hover {
  filter: brightness(1.12);
}

/* 进度条 */
.block-progress {
  padding: 18px 20px;
  border: 1px solid var(--border);
  border-radius: 14px;
  background: var(--bg-card);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.block-progress-head {
  display: flex;
  justify-content: space-between;
  font-size: 13px;
  color: var(--text);
}

.block-progress-num {
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.block-progress-bar {
  height: 8px;
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

/* 工具栏 */
.block-toolbar {
  display: flex;
  gap: 10px;
}

.block-search {
  flex: 1;
  padding: 8px 13px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-card);
  color: var(--text);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color 0.15s;
}

.block-search:focus {
  border-color: var(--accent);
}

.block-rerender {
  padding: 8px 14px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-card);
  color: var(--text-dim);
  font-size: 12.5px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.block-rerender:hover {
  border-color: var(--accent);
  color: var(--accent);
}

/* 分类 chips */
.block-cats {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.block-cat {
  padding: 5px 12px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--bg-card);
  color: var(--text-dim);
  font-size: 12px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.15s;
}

.block-cat:hover {
  border-color: var(--accent);
  color: var(--accent);
}

.block-cat.on {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

/* 网格 */
.block-grid {
  flex: 1;
  min-height: 0;
  overflow-y: auto;
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

.block-empty {
  padding: 40px 0;
  text-align: center;
  color: var(--text-dim);
  font-size: 13px;
}

.block-count {
  font-size: 12px;
  color: var(--text-dim);
  text-align: right;
}
</style>
