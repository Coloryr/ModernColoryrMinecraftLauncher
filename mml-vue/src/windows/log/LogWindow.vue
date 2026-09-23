<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from "vue";
import * as monaco from "monaco-editor";
import EditorWorker from "monaco-editor/editor/editor.worker.js?worker";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import { t } from "../../lib/i18n";
import { commands } from "../../lib/bindings";

// Monaco worker（纯文本日志只需要基础 worker）
(self as unknown as { MonacoEnvironment: monaco.Environment }).MonacoEnvironment = {
  getWorker: () => new EditorWorker(),
};

/** 日志视图 */
type LogView = "runtime" | "history" | "errors";

const VIEWS: Array<{ value: LogView; labelKey: string }> = [
  { value: "runtime", labelKey: "winLog.runtime" },
  { value: "history", labelKey: "winLog.history" },
  { value: "errors", labelKey: "winLog.errors" },
];

const view = ref<LogView>("runtime");
const loading = ref(false);
const emptyTip = ref("");

const container = ref<HTMLDivElement | null>(null);
let editor: monaco.editor.IStandaloneCodeEditor | null = null;

/** 按视图读取日志并填充编辑器 */
async function load() {
  loading.value = true;
  try {
    const fetcher =
      view.value === "runtime"
        ? commands.log.runtime
        : view.value === "history"
          ? commands.log.history
          : commands.log.errors;
    const text = (await fetcher()) ?? "";
    emptyTip.value = text.trim() ? "" : t("winLog.empty");
    editor?.getModel()?.setValue(text);
    editor?.revealLine(editor?.getModel()?.getLineCount() ?? 1);
  } catch (e) {
    emptyTip.value = String(e);
    editor?.getModel()?.setValue("");
  } finally {
    loading.value = false;
  }
}

function pick(view_: LogView) {
  if (view.value === view_) return;
  view.value = view_;
  load();
}

onMounted(() => {
  if (!container.value) return;
  editor = monaco.editor.create(container.value, {
    value: "",
    language: "plaintext",
    readOnly: true,
    automaticLayout: true,
    minimap: { enabled: false },
    fontFamily: "'Cascadia Mono', Consolas, monospace",
    fontSize: 12.5,
    lineNumbers: "off",
    scrollBeyondLastLine: false,
    wordWrap: "on",
    theme: themeName(),
  });
  load();
});

onBeforeUnmount(() => {
  editor?.dispose();
});

/** 按应用主题选 Monaco 配色 */
function themeName(): string {
  return document.documentElement.dataset.theme === "Light" ? "vs" : "vs-dark";
}
</script>

<template>
  <WindowFrame :title="t('features.log')" @close="$emit('close')">
    <div class="log-layout">
      <!-- 视图切换 + 刷新 -->
      <div class="log-side">
        <button
          v-for="v in VIEWS"
          :key="v.value"
          class="view-btn"
          :class="{ active: view === v.value }"
          @click="pick(v.value)"
        >
          {{ t(v.labelKey) }}
        </button>

        <button class="view-btn refresh" :disabled="loading" @click="load">
          {{ loading ? t("winLog.loading") : t("winLog.refresh") }}
        </button>
      </div>

      <!-- Monaco 只读编辑器 -->
      <div class="log-main">
        <div ref="container" class="editor-host"></div>
        <p v-if="emptyTip" class="empty-tip">{{ emptyTip }}</p>
      </div>
    </div>
  </WindowFrame>
</template>

<style scoped>
.log-layout {
  display: flex;
  gap: 14px;
  height: 100%;
}

.log-side {
  width: 130px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.view-btn {
  padding: 8px 12px;
  font-size: 13px;
  text-align: left;
  color: var(--text-dim);
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 9px;
  cursor: pointer;
}

.view-btn.active {
  color: var(--text);
  border-color: var(--accent, var(--border));
}

.view-btn.refresh {
  margin-top: auto;
}

.view-btn:disabled {
  opacity: 0.6;
  cursor: default;
}

.log-main {
  flex: 1;
  position: relative;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}

.editor-host {
  position: absolute;
  inset: 0;
}

.empty-tip {
  position: absolute;
  top: 14px;
  left: 0;
  right: 0;
  text-align: center;
  font-size: 12.5px;
  color: var(--text-dim);
  pointer-events: none;
}
</style>
