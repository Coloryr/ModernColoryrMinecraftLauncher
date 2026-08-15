<script setup lang="ts">
import { computed, ref } from "vue";
import { t } from "../lib/i18n";
import { openWindow } from "../windows/windowManager";
import type { Account } from "../lib/types";

const props = defineProps<{
  account: Account;
  accounts: Account[];
}>();

const emit = defineEmits<{
  (e: "update:account", account: Account): void;
}>();

const open = ref(false);

const typeText = computed(() => {
  switch (props.account.type) {
    case "microsoft":
      return t("account.microsoft");
    case "offline":
      return t("account.offline");
    default:
      return props.account.type;
  }
});

const typeClass = computed(() =>
  props.account.type === "microsoft" ? "type-ms" : "type-offline",
);

function toggle() {
  open.value = !open.value;
}

function pick(account: Account) {
  emit("update:account", account);
  open.value = false;
}
</script>

<template>
  <div class="account-wrap">
    <button class="account-btn" @click="toggle">
      <span class="avatar" :style="{ background: account.avatarColor }">
        {{ account.name.charAt(0).toUpperCase() }}
      </span>
      <span class="account-meta">
        <span class="account-name">{{ account.name }}</span>
        <span class="account-type" :class="typeClass">{{ typeText }}</span>
      </span>
      <svg class="chevron" :class="{ flip: open }" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>

    <div v-if="open" class="menu-backdrop" @click="open = false"></div>

    <div v-if="open" class="account-menu">
      <div class="menu-title">{{ t("account.switch") }}</div>
      <button
        v-for="acc in accounts"
        :key="acc.uuid"
        class="menu-item"
        :class="{ active: acc.uuid === account.uuid }"
        @click="pick(acc)"
      >
        <span class="avatar small" :style="{ background: acc.avatarColor }">
          {{ acc.name.charAt(0).toUpperCase() }}
        </span>
        <span class="menu-meta">
          <span class="menu-name">{{ acc.name }}</span>
          <span class="menu-type">{{ acc.type === "microsoft" ? t("account.microsoft") : t("account.offline") }}</span>
        </span>
      </button>
      <div
        class="menu-footer"
        @click="open = false; openWindow('account')"
      >{{ t("account.manage") }}</div>
    </div>
  </div>
</template>

<style scoped>
.account-wrap {
  position: relative;
}

/* 点击菜单外区域收起 */
.menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 190;
}

.account-btn {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 12px 6px 8px;
  border-radius: 12px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text);
  cursor: pointer;
  transition: all 0.15s;
}

.account-btn:hover {
  background: var(--bg-hover);
  border-color: var(--accent);
}

.avatar {
  width: 34px;
  height: 34px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #fff;
  font-weight: 700;
  font-size: 15px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.35);
  border: 1px solid rgba(255, 255, 255, 0.15);
  flex-shrink: 0;
}

.avatar.small {
  width: 30px;
  height: 30px;
  font-size: 13px;
}

.account-meta {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  line-height: 1.25;
  text-align: left;
}

.account-name {
  font-size: 13.5px;
  font-weight: 600;
}

.account-type {
  font-size: 11px;
  padding: 1px 7px;
  border-radius: 20px;
}

.type-ms {
  color: #8fd0ff;
  background: rgba(63, 140, 255, 0.16);
}

.type-offline {
  color: var(--text-dim);
  background: rgba(154, 163, 175, 0.14);
}

.chevron {
  color: var(--text-dim);
  transition: transform 0.15s;
}

.chevron.flip {
  transform: rotate(180deg);
}

.account-menu {
  position: absolute;
  right: 0;
  top: calc(100% + 8px);
  width: 240px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  box-shadow: 0 6px 18px rgba(0, 0, 0, 0.22);
  padding: 8px;
  z-index: 200;
}

.menu-title {
  font-size: 12px;
  color: var(--text-dim);
  padding: 6px 8px;
}

.menu-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  padding: 8px;
  border: none;
  border-radius: 9px;
  background: transparent;
  color: var(--text);
  cursor: pointer;
  text-align: left;
}

.menu-item:hover {
  background: var(--bg-hover);
}

.menu-item.active {
  background: rgba(79, 140, 255, 0.14);
  outline: 1px solid rgba(79, 140, 255, 0.4);
}

.menu-meta {
  display: flex;
  flex-direction: column;
  line-height: 1.25;
}

.menu-name {
  font-size: 13px;
  font-weight: 600;
}

.menu-type {
  font-size: 11px;
  color: var(--text-dim);
}

.menu-footer {
  font-size: 12px;
  color: var(--text-dim);
  text-align: center;
  padding: 8px;
  border-top: 1px solid var(--border);
  margin-top: 4px;
  cursor: pointer;
}

.menu-footer:hover {
  color: var(--accent);
}
</style>
