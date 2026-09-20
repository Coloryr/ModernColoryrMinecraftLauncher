<script setup lang="ts">
// 下载整合包窗口：CurseForge / Modrinth 在线搜索 + 选版本安装
// 安装进度由后端 add-pack-progress 事件驱动，展示在本窗口。
import { onMounted, onUnmounted, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import { api, onAddPackProgress } from "../../lib/api";
import { t, tErr } from "../../lib/i18n";
import { showToast } from "../../lib/toast";
import type { PackProgressDto } from "../../lib/bindings";
import ModpackMode from "./ModpackMode.vue";

defineEmits<{ (e: "close"): void }>();

/** 分组（空 = 默认分组），带已有分组作为候选 */
const group = ref("");
const groups = ref<string[]>([]);
const installing = ref(false);

/** 安装进度（add-pack-progress 事件驱动，null = 未在安装） */
const packProgress = ref<PackProgressDto | null>(null);

/** 安装在线整合包：实例名取自整合包元数据 */
async function install(p: {
  source: string;
  projectId: string;
  fileId: string;
  fileName: string;
}) {
  if (installing.value) {
    return;
  }
  const name = group.value.trim() === "" ? null : group.value.trim();
  installing.value = true;
  try {
    const uuid = await api.installModpack(p.source, p.projectId, p.fileId, name);
    // 主窗口靠 instance-change 事件刷新列表，这里只把选中实例切过去
    localStorage.setItem("mcml.addedInstance", uuid);
    showToast(t("modpack.installDone", { name: p.fileName }));
  } catch (e) {
    packProgress.value = null;
    showToast(t("add.createFail", { msg: tErr(e) }));
  } finally {
    installing.value = false;
  }
}

onMounted(async () => {
  const unlisten = await onAddPackProgress((e) => {
    packProgress.value = e;
    if (e.state === "done") {
      setTimeout(() => {
        packProgress.value = null;
      }, 600);
    }
  });
  onUnmounted(unlisten);

  try {
    groups.value = (await api.getGroups()).filter((g) => g.trim());
  } catch {
    groups.value = [];
  }
});
</script>

<template>
  <WindowFrame :title="t('winTitle.addModpack')" @close="$emit('close')">
    <div class="modpack-body">
      <ModpackMode
        :group="group"
        :groups="groups"
        @update:group="group = $event"
        @install="install"
      />
    </div>

    <!-- 整合包安装进度 -->
    <BaseModal v-if="packProgress" :title="t('add.installing')" :closable="false">
      <div class="install-progress">
        <div class="install-state">{{ t(`add.packState.${packProgress.state}`) }}</div>
        <div class="progress-track">
          <div
            class="progress-fill"
            :style="{ width: packProgress.total ? (packProgress.now / packProgress.total) * 100 + '%' : '0%' }"
          />
        </div>
        <div class="install-num" v-if="packProgress.total">
          {{ packProgress.now }} / {{ packProgress.total }}
        </div>
        <template v-if="packProgress.subText || packProgress.subTotal">
          <div class="install-sub">{{ packProgress.subText || "" }}</div>
          <div class="progress-track sub">
            <div
              class="progress-fill"
              :style="{ width: packProgress.subTotal ? (packProgress.subNow / packProgress.subTotal) * 100 + '%' : '0%' }"
            />
          </div>
        </template>
      </div>
    </BaseModal>
  </WindowFrame>
</template>

<style scoped>
.modpack-body {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

/* 整合包安装进度弹窗 */
.install-progress {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.install-state {
  font-size: 13.5px;
  font-weight: 600;
  color: var(--text);
}

.progress-track {
  height: 8px;
  border-radius: 4px;
  background: var(--bg-hover);
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  border-radius: 4px;
  background: var(--accent-grad);
  transition: width 0.3s;
}

.progress-track.sub {
  height: 6px;
}

.install-num {
  font-size: 12px;
  color: var(--text-dim);
  text-align: right;
}

.install-sub {
  font-size: 12px;
  color: var(--text-dim);
  margin-top: 4px;
  word-break: break-all;
}
</style>
