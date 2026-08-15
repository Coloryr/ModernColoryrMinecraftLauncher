<script setup lang="ts">
// 游戏内代理：地址 / 端口 / 用户名 / 密码
import { t } from "../lib/i18n";
import type { InstanceArgs } from "../lib/types";
import NumberStepper from "./ui/NumberStepper.vue";

const props = defineProps<{ args: InstanceArgs }>();

const emit = defineEmits<{ (e: "update:args", v: InstanceArgs): void }>();

function update(patch: Partial<InstanceArgs>) {
  emit("update:args", { ...props.args, ...patch });
}
</script>

<template>
  <div class="proxy-panel">
    <div class="proxy-row">
      <span class="proxy-label">{{ t("proxy.ip") }}</span>
      <input
        class="field-input grow"
        :value="args.proxyIp"
        placeholder="127.0.0.1"
        spellcheck="false"
        @input="update({ proxyIp: ($event.target as HTMLInputElement).value })"
      />
      <span class="proxy-label small">{{ t("proxy.port") }}</span>
      <NumberStepper
        :model-value="args.proxyPort"
        :min="1"
        :max="65535"
        :step="1"
        @update:model-value="(v: number) => update({ proxyPort: v })"
      />
    </div>

    <div class="proxy-row">
      <span class="proxy-label">{{ t("proxy.user") }}</span>
      <input
        class="field-input grow"
        :value="args.proxyUser"
        placeholder="user"
        spellcheck="false"
        @input="update({ proxyUser: ($event.target as HTMLInputElement).value })"
      />
      <span class="proxy-label small">{{ t("proxy.password") }}</span>
      <input
        class="field-input grow"
        type="password"
        :value="args.proxyPass"
        placeholder="••••••"
        spellcheck="false"
        @input="update({ proxyPass: ($event.target as HTMLInputElement).value })"
      />
    </div>
  </div>
</template>

<style scoped>
.proxy-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 14px 16px;
}

.proxy-row {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.proxy-label {
  font-size: 12.5px;
  font-weight: 600;
  color: var(--text-dim);
  min-width: 66px;
}

.proxy-label.small {
  min-width: 0;
  margin-left: 4px;
}

.grow {
  flex: 1;
  min-width: 110px;
}
</style>
