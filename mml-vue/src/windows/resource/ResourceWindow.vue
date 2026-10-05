<script setup lang="ts">
// 资源管理窗口：左侧分类导航 + 右侧内容区（存档分类下另有「数据包」子页）
// 数据来自 resource_* 命令；删除进回收站，操作后重拉当前分类。
//
// 这个文件只做编排：
// - 数据与加载        → composables/useResourceData（分类 / 子页选择也在那儿，加载跟着它走）
// - 共用操作          → composables/useResourceOps（忙碌标记 / 确认弹窗 / 打开目录 / 重拉）
// - 各分类的列表      → parts/*Pane.vue（每类一个，自带的弹窗跟上）
// - 窗口级样式        → resource.css（非 scoped，拆开的子组件共用；见该文件头部说明）
import { onMounted, watch } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import { t } from "../../lib/i18n";
import { useWindowRefresh } from "../../composables/useWindowRefresh";
import { openWindow } from "../windowManager";
import { useResourceData } from "./composables/useResourceData";
import { useResourceOps } from "./composables/useResourceOps";
import { useResourceView } from "./composables/useResourceView";
import CategoryRail from "./parts/CategoryRail.vue";
import ConfirmModal from "./parts/ConfirmModal.vue";
import DatapackPane from "./parts/DatapackPane.vue";
import InstanceChip from "./parts/InstanceChip.vue";
import ModPane from "./parts/ModPane.vue";
import PackPane from "./parts/PackPane.vue";
import SavePane from "./parts/SavePane.vue";
import SchematicPane from "./parts/SchematicPane.vue";
import ScreenshotPane from "./parts/ScreenshotPane.vue";
import ServerPane from "./parts/ServerPane.vue";
import ShaderPane from "./parts/ShaderPane.vue";
import type { CategoryId } from "./types";
import "./resource.css";

// 必须显式声明 close：不声明的话 Vue 会把父级的 @close 当 attrs 透传到根组件（WindowFrame），
// 与模板里的 @close="$emit('close')" 合并成两个处理器 —— 一次"返回"会调两遍 closeWindow()，
// 第二遍时 currentKind 已经回到 main，于是"返回"变成退出应用
defineEmits<{ (e: "close"): void }>();

/**
 * 视图偏好（分类顺序 / 上次打开的类别 / 模组展示方式）
 *
 * 存在**实例**的 `guisetting.json` 里，所以要先有实例才能读 —— 数据层建好之后
 * 把"读偏好"挂成它的实例就绪回调（见 useResourceData 的 setInstanceReadyHook），
 * 在它拉列表**之前**跑，第一次就按记住的顺序与类别显示。
 */
const data = useResourceData();
const view = useResourceView(data);
const ops = useResourceOps(data);
const { category, saveTab, instance, sync } = data;
const { confirmBox } = ops;
const { load: loadView } = view;

data.setInstanceReadyHook(async () => {
  await loadView();
  // 读回来的"上次类别"要落到当前选择上（数据层的 watch 会跟着拉对应列表）
  category.value = view.initialCategory.value;
});

// 记住这次看的是哪一类：下次打开直接停在这儿
watch(category, (id) => view.rememberCategory(id));

onMounted(sync);

// 单窗口模式：窗口被 KeepAlive 缓存，切回不会重新挂载 → 自己补一次（实例可能已被主窗口切走）
useWindowRefresh(sync);

/** 切分类：只改选择，加载由 useResourceData 里的 watch 负责 */
function selectCategory(id: CategoryId) {
  category.value = id;
}

/** 「下载资源」：开「添加资源」窗口（与主窗口侧边栏那个入口同一个去处） */
function openDownload() {
  openWindow("add_resource");
}
</script>

<template>
  <WindowFrame :title="t('resource.title')" @close="$emit('close')">
    <!-- 标题栏：当前实例（原先挤在左侧分类栏顶部）+ 下载资源入口 -->
    <template #head-right>
      <InstanceChip :instance="instance" />
      <BaseButton size="sm" variant="accent" @click="openDownload">
        {{ t("resource.download") }}
      </BaseButton>
    </template>

    <div class="resource-layout">
      <CategoryRail :data="data" :view="view" @select="selectCategory" />

      <section class="cat-content">
        <!-- 没有选中实例：只给这一条引导（不再顺带铺一个空列表，原先两段会同时出现） -->
        <div v-if="!instance" class="empty-tip">{{ t("resource.notSelected") }}</div>
        <template v-else>
          <ScreenshotPane v-if="category === 'screenshots'" :data="data" :ops="ops" />
          <ServerPane v-else-if="category === 'servers'" :data="data" :ops="ops" />
          <ModPane v-else-if="category === 'mods'" :data="data" :ops="ops" :view="view" />
          <PackPane v-else-if="category === 'resourcepacks'" :data="data" :ops="ops" />
          <ShaderPane v-else-if="category === 'shaders'" :data="data" :ops="ops" />
          <SchematicPane v-else-if="category === 'schematics'" :data="data" :ops="ops" />
          <SavePane v-else-if="saveTab === 'saves'" :data="data" :ops="ops" />
          <DatapackPane v-else :data="data" :ops="ops" />
        </template>
      </section>

      <!-- 删除 / 清空确认（全窗口共用一份，内容来自 ops） -->
      <ConfirmModal v-if="confirmBox" :ops="ops" @close="confirmBox = null" />
    </div>
  </WindowFrame>
</template>
