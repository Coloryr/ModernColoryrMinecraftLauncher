<script setup lang="ts">
// 资源管理窗口：存档 / 模组 / 资源包 / 截图 / 服务器 / 光影包 / 结构文件
// 存档分类下有「存档 / 数据包」子页
import { computed, onMounted, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import { t } from "../../lib/i18n";
import { api } from "../../lib/api";
import type { InstanceInfo } from "../../lib/types";

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

const instance = ref<InstanceInfo | null>(null);

const MOCK_ITEMS: Record<string, string[]> = {
  saves: ["生存世界", "创造测试", "空岛地图", "红石实验"],
  datapacks: ["自定义合成", "新生物群系", "矿物调整"],
  mods: ["JEI 物品栏", "OptiFine HD", "小地图", "地形生成优化"],
  resourcepacks: ["Faithful 32x", "原版高清", "3D 默认"],
  screenshots: ["2026-04-12_21.30.12.png", "2026-04-10_20.44.05.png", "2026-04-08_12.05.33.png"],
  servers: ["生存服 127.0.0.1:25565", "创造服（未启用）"],
  shaders: ["BSL Shaders", "Complementary Reimagined", "SEUS PTGI"],
  schematics: ["古堡.schematic", "现代别墅.schematic", "红石电梯.schematic"],
};

const categories: Array<{ id: CategoryId; label: string }> = [
  { id: "saves", label: t("resource.saves") },
  { id: "mods", label: t("resource.mods") },
  { id: "resourcepacks", label: t("resource.resourcepacks") },
  { id: "screenshots", label: t("resource.screenshots") },
  { id: "servers", label: t("resource.servers") },
  { id: "shaders", label: t("resource.shaders") },
  { id: "schematics", label: t("resource.schematics") },
];

const currentItems = computed(() => {
  if (category.value === "saves") {
    return saveTab.value === "saves" ? MOCK_ITEMS.saves : MOCK_ITEMS.datapacks;
  }
  return MOCK_ITEMS[category.value] ?? [];
});

const toast = ref("");
let toastTimer: number | undefined;

function showToast(msg: string) {
  toast.value = msg;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.value = "";
  }, 2000);
}

function act(name: string) {
  showToast(t("actions.wip", { name }));
}

onMounted(async () => {
  try {
    const list = await api.getInstances();
    const uuid = localStorage.getItem("mcml.activeInstance");
    instance.value = list.find((i) => i.uuid === uuid) ?? list[0] ?? null;
  } catch {
    instance.value = null;
  }
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
          @click="category = c.id"
        >
          {{ c.label }}
        </button>
      </aside>

      <!-- 内容区 -->
      <section class="cat-content">
        <!-- 存档分类：存档 / 数据包 子页 -->
        <div v-if="category === 'saves'" class="content-head">
          <SegmentedTabs
            :model-value="saveTab"
            :options="[
              { value: 'saves', label: t('resource.saves') },
              { value: 'datapacks', label: t('resource.datapacks') },
            ]"
            @update:model-value="saveTab = $event as 'saves' | 'datapacks'"
          />
        </div>
        <h3 v-else class="content-head title-only">{{ categories.find((c) => c.id === category)?.label }}</h3>

        <div class="item-list">
          <div v-for="item in currentItems" :key="item" class="item-row">
            <span class="item-name">{{ item }}</span>
            <div class="item-actions">
              <button class="mini-btn" @click="act(item)">{{ t("actions.openFolder") }}</button>
              <button class="mini-btn danger" @click="act(item)">{{ t("actions.delete") }}</button>
            </div>
          </div>
          <div v-if="currentItems.length === 0" class="empty-tip">{{ t("resource.empty") }}</div>
        </div>
      </section>
    </div>

    <Transition name="toast">
      <div v-if="toast" class="toast">{{ toast }}</div>
    </Transition>
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
  justify-content: flex-end;
}

.title-only {
  font-size: 15px;
  font-weight: 700;
  justify-content: flex-start;
  padding: 4px 0;
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
  justify-content: space-between;
  gap: 10px;
  padding: 10px 12px;
  border-radius: 9px;
  background: var(--bg-side);
  border: 1px solid var(--border);
}

.item-name {
  font-size: 13px;
  color: var(--text);
  word-break: break-all;
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

.toast {
  position: fixed;
  bottom: 34px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--bg-card);
  border: 1px solid var(--accent-border);
  color: var(--text);
  font-size: 13px;
  padding: 11px 22px;
  border-radius: 10px;
  box-shadow: var(--shadow-lg);
  z-index: 500;
}

.toast-enter-active,
.toast-leave-active {
  transition: opacity 0.2s, transform 0.2s;
}

.toast-enter-from,
.toast-leave-to {
  opacity: 0;
  transform: translateX(-50%) translateY(8px);
}
</style>
