<script setup lang="ts">
// 实例启动参数面板：
//   启动参数：内存大小（最小/最大 + 右侧输入）/ 窗口大小 / 使用的 Java（自定义时显示路径 + 选择文件）
//   扩展参数：GC / 自定义主类 / 附加 JVM 参数 / 附加游戏参数 / 附加 classpath（多行列表）
//            / 附加环境变量（键值对列表）
import { ref } from "vue";
import { t } from "../lib/i18n";
import type { InstanceArgs, JavaInfo } from "../lib/types";
import BaseButton from "./ui/BaseButton.vue";
import NumberStepper from "./ui/NumberStepper.vue";

const props = defineProps<{
  args: InstanceArgs;
  javas: JavaInfo[];
}>();

const emit = defineEmits<{ (e: "update:args", v: InstanceArgs): void }>();

const advancedOpen = ref(false);
const fileInput = ref<HTMLInputElement | null>(null);

function update(patch: Partial<InstanceArgs>) {
  emit("update:args", { ...props.args, ...patch });
}

function onBrowseFile(e: Event) {
  const input = e.target as HTMLInputElement;
  const file = input.files?.[0];
  if (file) {
    // 浏览器环境只能拿到 fakepath；接入 Tauri 后改用文件对话框插件
    update({ javaPath: file.name });
  }
}

// ---- 多行字符串参数 ----
type LineKey = "jvmArgs" | "gameArgs" | "classPath";

function setLine(key: LineKey, idx: number, value: string) {
  const arr = [...props.args[key]];
  arr[idx] = value;
  update({ [key]: arr } as Partial<InstanceArgs>);
}

function removeLine(key: LineKey, idx: number) {
  const arr = [...props.args[key]];
  arr.splice(idx, 1);
  update({ [key]: arr } as Partial<InstanceArgs>);
}

function addLine(key: LineKey) {
  update({ [key]: [...props.args[key], ""] } as Partial<InstanceArgs>);
}

// ---- 环境变量（键值对） ----
function setEnvKey(idx: number, value: string) {
  const arr = props.args.envVars.map((l, i) => (i === idx ? { ...l, key: value } : l));
  update({ envVars: arr });
}

function setEnvValue(idx: number, value: string) {
  const arr = props.args.envVars.map((l, i) => (i === idx ? { ...l, value } : l));
  update({ envVars: arr });
}

function removeEnv(idx: number) {
  const arr = [...props.args.envVars];
  arr.splice(idx, 1);
  update({ envVars: arr });
}

function addEnv() {
  update({ envVars: [...props.args.envVars, { key: "", value: "" }] });
}

const gcOptions = [
  { value: "auto", label: t("args.gcAuto") },
  { value: "g1gc", label: t("args.gcG1") },
  { value: "zgc", label: t("args.gcZ") },
  { value: "none", label: t("args.gcNone") },
  { value: "custom", label: t("args.gcCustom") },
];
</script>

<template>
  <div class="args-panel">
    <!-- 启动参数 -->
    <div class="args-block">
      <!-- 内存：最小 / 最大（输入框，与窗口行对齐） -->
      <div class="args-row">
        <span class="args-label">{{ t("args.memory") }}</span>
        <span class="sub-tag">{{ t("args.minMemory") }}</span>
        <NumberStepper
          :model-value="args.minMemory"
          :min="512"
          :max="args.memory"
          :step="256"
          @update:model-value="(v: number) => update({ minMemory: v })"
        />
        <span class="sub-tag">{{ t("args.maxMemory") }}</span>
        <NumberStepper
          :model-value="args.memory"
          :min="args.minMemory"
          :max="16384"
          :step="256"
          @update:model-value="(v: number) => update({ memory: v })"
        />
        <span class="mem-unit">MB</span>
      </div>

      <!-- 窗口大小：宽 / 高 + 全屏（与内存行对齐） -->
      <div class="args-row">
        <span class="args-label">{{ t("args.windowSize") }}</span>
        <span class="sub-tag">{{ t("args.width") }}</span>
        <NumberStepper
          :model-value="args.width"
          :min="0"
          :max="9999"
          :step="10"
          @update:model-value="(v: number) => update({ width: v })"
        />
        <span class="sub-tag">{{ t("args.height") }}</span>
        <NumberStepper
          :model-value="args.height"
          :min="0"
          :max="9999"
          :step="10"
          @update:model-value="(v: number) => update({ height: v })"
        />
        <label class="chk">
          <input
            type="checkbox"
            :checked="args.fullscreen"
            @change="update({ fullscreen: ($event.target as HTMLInputElement).checked })"
          />
          {{ t("args.fullscreen") }}
        </label>
      </div>

      <div class="args-row">
        <span class="args-label">{{ t("args.java") }}</span>
        <select
          class="field-select grow"
          :value="args.javaName"
          @change="update({ javaName: ($event.target as HTMLSelectElement).value })"
        >
          <option v-for="j in javas" :key="j.name" :value="j.name">
            {{ j.name }}（Java {{ j.major }}）
          </option>
          <option value="custom">{{ t("args.customJava") }}</option>
        </select>
      </div>

      <!-- 自定义 Java 路径 -->
      <div v-if="args.javaName === 'custom'" class="args-row java-custom">
        <span class="args-label">{{ t("args.javaPath") }}</span>
        <input
          class="field-input grow"
          :value="args.javaPath"
          placeholder="C:\Program Files\Java\jdk-21\bin\java.exe"
          spellcheck="false"
          @input="update({ javaPath: ($event.target as HTMLInputElement).value })"
        />
        <BaseButton size="sm" @click="fileInput?.click()">{{ t("args.browse") }}</BaseButton>
        <input ref="fileInput" type="file" class="hidden-file" @change="onBrowseFile" />
      </div>
    </div>

    <!-- 扩展参数 -->
    <div class="args-block advanced">
      <button class="advanced-toggle" @click="advancedOpen = !advancedOpen">
        <span>{{ t("args.advanced") }}</span>
        <svg
          class="args-chevron"
          :class="{ flip: advancedOpen }"
          viewBox="0 0 24 24"
          width="13"
          height="13"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
        >
          <path d="m6 9 6 6 6-6" />
        </svg>
      </button>

      <div v-show="advancedOpen" class="advanced-body">
        <div class="args-row">
          <span class="args-label">{{ t("args.gc") }}</span>
          <select class="field-select grow" :value="args.gc" @change="update({ gc: ($event.target as HTMLSelectElement).value })">
            <option v-for="g in gcOptions" :key="g.value" :value="g.value">{{ g.label }}</option>
          </select>
        </div>

        <!-- 自定义 GC 参数 -->
        <div v-if="args.gc === 'custom'" class="args-row">
          <span class="args-label">{{ t("args.gcCustom") }}</span>
          <input
            class="field-input grow"
            :value="args.gcCustom"
            placeholder="-XX:+UseZGC -XX:ZCollectionInterval=30"
            spellcheck="false"
            @input="update({ gcCustom: ($event.target as HTMLInputElement).value })"
          />
        </div>

        <div class="args-row">
          <span class="args-label">{{ t("args.mainClass") }}</span>
          <input
            class="field-input grow"
            :value="args.mainClass"
            placeholder="net.minecraft.client.main.Main"
            spellcheck="false"
            @input="update({ mainClass: ($event.target as HTMLInputElement).value })"
          />
        </div>

        <!-- 附加 JVM 参数 -->
        <label class="args-label">{{ t("args.jvmExtra") }}</label>
        <div v-for="(line, i) in args.jvmArgs" :key="i" class="line-row">
          <input
            class="field-input grow"
            :value="line"
            placeholder="-XX:+UseG1GC"
            spellcheck="false"
            @input="setLine('jvmArgs', i, ($event.target as HTMLInputElement).value)"
          />
          <button class="line-del" title="✕" @click="removeLine('jvmArgs', i)">✕</button>
        </div>
        <button class="line-add" @click="addLine('jvmArgs')">＋ {{ t("args.addLine") }}</button>

        <!-- 附加游戏参数 -->
        <label class="args-label">{{ t("args.gameExtra") }}</label>
        <div v-for="(line, i) in args.gameArgs" :key="i" class="line-row">
          <input
            class="field-input grow"
            :value="line"
            placeholder="--server 127.0.0.1:25565"
            spellcheck="false"
            @input="setLine('gameArgs', i, ($event.target as HTMLInputElement).value)"
          />
          <button class="line-del" title="✕" @click="removeLine('gameArgs', i)">✕</button>
        </div>
        <button class="line-add" @click="addLine('gameArgs')">＋ {{ t("args.addLine") }}</button>

        <!-- 附加 classpath -->
        <label class="args-label">{{ t("args.classPath") }}</label>
        <div v-for="(line, i) in args.classPath" :key="i" class="line-row">
          <input
            class="field-input grow"
            :value="line"
            placeholder="libraries/xxx.jar"
            spellcheck="false"
            @input="setLine('classPath', i, ($event.target as HTMLInputElement).value)"
          />
          <button class="line-del" title="✕" @click="removeLine('classPath', i)">✕</button>
        </div>
        <button class="line-add" @click="addLine('classPath')">＋ {{ t("args.addLine") }}</button>

        <!-- 附加环境变量（键值对） -->
        <label class="args-label">{{ t("args.env") }}</label>
        <div v-for="(line, i) in args.envVars" :key="i" class="line-row">
          <input
            class="field-input env-key"
            :value="line.key"
            :placeholder="t('args.envKey')"
            spellcheck="false"
            @input="setEnvKey(i, ($event.target as HTMLInputElement).value)"
          />
          <input
            class="field-input grow"
            :value="line.value"
            :placeholder="t('args.envValue')"
            spellcheck="false"
            @input="setEnvValue(i, ($event.target as HTMLInputElement).value)"
          />
          <button class="line-del" title="✕" @click="removeEnv(i)">✕</button>
        </div>
        <button class="line-add" @click="addEnv">＋ {{ t("args.addLine") }}</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.args-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 14px 16px;
}

.args-block {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.advanced {
  border-top: 1px solid var(--border);
  padding-top: 10px;
}

.args-row {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.args-label {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-dim);
  flex: 0 0 84px;
}

/* 与内存行对齐用的小标签（最小/最大/宽/高） */
.sub-tag {
  flex: 0 0 28px;
  font-size: 12px;
  color: var(--text-dim);
  white-space: nowrap;
  text-align: right;
}

.grow {
  flex: 1;
  min-width: 120px;
}

.mem-unit {
  font-size: 12px;
  color: var(--text-dim);
  white-space: nowrap;
}

.chk {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12.5px;
  color: var(--text);
  cursor: pointer;
  accent-color: var(--accent);
  margin-left: 6px;
  white-space: nowrap;
}

.java-custom {
  padding-left: 96px;
}

.hidden-file {
  display: none;
}

.advanced-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-size: 13px;
  font-weight: 600;
  font-family: inherit;
  cursor: pointer;
  padding: 4px 0;
}

.advanced-toggle:hover {
  color: var(--text);
}

.advanced-body {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.args-chevron {
  transition: transform 0.15s;
}

.args-chevron.flip {
  transform: rotate(180deg);
}

/* 多行参数 */
.line-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.env-key {
  flex: 0 0 140px;
}

.line-del {
  width: 28px;
  height: 28px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  cursor: pointer;
  flex-shrink: 0;
  transition: all 0.12s;
}

.line-del:hover {
  color: var(--red);
  border-color: var(--red);
  background: rgba(255, 95, 86, 0.1);
}

.line-add {
  align-self: flex-start;
  padding: 4px 12px;
  border-radius: 8px;
  border: 1px dashed var(--border);
  background: transparent;
  color: var(--text-dim);
  font-size: 12.5px;
  font-family: inherit;
  cursor: pointer;
  transition: all 0.12s;
  margin-top: -2px;
}

.line-add:hover {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}
</style>
