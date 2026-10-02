<script setup lang="ts">
// 下载整合包窗口：CurseForge / Modrinth 在线搜索 + 选版本安装
// 安装是多任务的：命令立即返回，进度由 add-modpack-status 事件驱动，
// 顶部进度条显示任务数与详情（见 ModpackInstallBar）。
import { onMounted, onUnmounted, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import ModpackInstallBar from "../../components/ModpackInstallBar.vue";
import { api } from "../../lib/api";
import { t, tErr } from "../../lib/i18n";
import { showToast } from "../../lib/toast";
import { useModpackStatus } from "../../lib/modpackTasks";
import ModpackMode from "./ModpackMode.vue";

defineEmits<{ (e: "close"): void }>();

/** 详情是否展开：详情铺满窗口后，返回键放到标题栏（标题栏由本组件持有） */
const detailOpen = ref(false);
/** 详情状态在子组件里，标题栏的返回键只能通过它暴露的 closeDetail 收起 */
const modeRef = ref<InstanceType<typeof ModpackMode> | null>(null);

/** 分组（空 = 默认分组），带已有分组作为候选 */
const group = ref("");
const groups = ref<string[]>([]);

/** 安装任务状态（本窗口是"负责显示"的窗口，终态 toast 由这里弹） */
const { status: modpackStatus, init: initModpackStatus } = useModpackStatus(true);

/** 安装在线整合包：实例名取自整合包元数据，命令立即返回（任务后台安装） */
async function install(p: {
  source: string;
  projectId: string;
  fileId: string;
  fileName: string;
}) {
  const name = group.value.trim() === "" ? null : group.value.trim();
  try {
    await api.installModpack(p.source, p.projectId, p.fileId, name);
  } catch (e) {
    showToast(t("add.createFail", { msg: tErr(e) }));
  }
}

onMounted(async () => {
  const unlisten = await initModpackStatus();
  onUnmounted(unlisten);

  try {
    groups.value = (await api.getGroups()).filter((g) => g.trim());
  } catch {
    groups.value = [];
  }
});
</script>

<template>
  <WindowFrame
    :title="t('winTitle.addModpack')"
    :back="detailOpen ? t('modpack.back') : ''"
    hide-modpack-indicator
    @back="modeRef?.closeDetail()"
    @close="$emit('close')"
  >
    <div class="modpack-body">
      <!-- 安装任务进度条（多任务，点击展开详情） -->
      <ModpackInstallBar v-if="modpackStatus?.tasks.length" :status="modpackStatus" />

      <ModpackMode
        ref="modeRef"
        :group="group"
        :groups="groups"
        :status="modpackStatus"
        @update:group="group = $event"
        @install="install"
        @detail="detailOpen = $event"
      />
    </div>
  </WindowFrame>
</template>

<style scoped>
.modpack-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* 安装进度条也是这一列里的卡片：边缘与筛选卡、项目卡一样落在窗口内容区边距（26px）上，
   不再单独回缩 8px —— 那 8px 是项目列表给悬停阴影留的，已在 ModpackMode 里用负外边距抵掉 */
</style>
