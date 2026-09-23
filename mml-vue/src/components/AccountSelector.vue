<script setup lang="ts">
import { computed, ref } from "vue";
import { t } from "../lib/i18n";
import { openWindow } from "../windows/windowManager";
import type { AccountStoreDto } from "../lib/bindings";

const props = defineProps<{
  account: AccountStoreDto | null;
  accounts: AccountStoreDto[];
}>();

const emit = defineEmits<{
  (e: "update:account", account: AccountStoreDto): void;
}>();

const open = ref(false);

const typeText = computed(() => {
  if (!props.account) return "";
  switch (props.account.authType) {
    case "microsoft":
      return t("account.microsoft");
    case "offline":
      return t("account.offline");
    default:
      return props.account.authType;
  }
});

const typeClass = computed(() =>
  props.account?.authType === "microsoft" ? "type-ms" : "type-offline",
);

function toggle() {
  open.value = !open.value;
}

function pick(account: AccountStoreDto) {
  emit("update:account", account);
  open.value = false;
}
</script>

<template>
  <div class="account-wrap">
    <button class="account-btn" @click="toggle">
      <template v-if="account">
        <span class="avatar" :style="{ background: account.avatarColor }">
          {{ account.userName.charAt(0).toUpperCase() }}
        </span>
        <span class="account-meta">
          <span class="account-name">{{ account.userName }}</span>
          <span class="account-type" :class="typeClass">{{ typeText }}</span>
        </span>
      </template>
      <template v-else>
        <!-- 加号用 SVG 而非全角“＋”字符：中文字体把该字形画在 em 框偏上位置，
             盒子居中了笔画仍显偏上，SVG 由 flex 居中不受字体度量影响 -->
        <span class="avatar placeholder">
          <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round">
            <path d="M12 5v14M5 12h14" />
          </svg>
        </span>
        <span class="account-meta">
          <span class="account-name dim">{{ t("account.noAccount") }}</span>
        </span>
      </template>
      <svg class="chevron" :class="{ flip: open }" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2">
        <path d="m6 9 6 6 6-6" />
      </svg>
    </button>

    <!-- data-no-drag：遮罩在顶栏（标题栏拖拽区）里，不标记的话按下会被
         startDragging 接管，click 事件到不了，菜单就收不起来 -->
    <div v-if="open" class="menu-backdrop" data-no-drag @click="open = false"></div>

    <!-- data-no-drag：菜单也在标题栏拖拽区里，footer 等非 button 区域
         的 click 会被 startDragging 吞掉，须整体豁免 -->
    <Transition name="drop">
      <div v-if="open" class="account-menu" data-no-drag>
        <div class="menu-title">{{ t("account.switch") }}</div>
        <div v-if="accounts.length === 0" class="empty-tip">{{ t("account.noAccount") }}</div>
        <button
          v-for="acc in accounts"
          :key="acc.uuid"
          class="menu-item"
          :class="{ active: acc.uuid === account?.uuid }"
          @click="pick(acc)"
        >
          <span class="avatar small" :style="{ background: acc.avatarColor }">
            {{ acc.userName.charAt(0).toUpperCase() }}
          </span>
          <span class="menu-meta">
            <span class="menu-name">{{ acc.userName }}</span>
            <span class="menu-type">{{ acc.authType === "microsoft" ? t("account.microsoft") : t("account.offline") }}</span>
          </span>
        </button>
        <div
          class="menu-footer"
          @click="open = false; openWindow('account')"
        >{{ t("account.manage") }}</div>
      </div>
    </Transition>
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

/* 未选择账户的占位头像：底色用暗色，＋号才不至白字落在亮背景上看不见 */
.avatar.placeholder {
  background: var(--bg-hover);
  border: 1px dashed var(--border);
  color: var(--text-dim);
  box-shadow: none;
}

/* 未选择账户的文字弱化 */
.account-name.dim {
  color: var(--text-dim);
  font-weight: 500;
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
  /* 菜单贴右缘，展开时以右上角为原点，避免向左"甩" */
  transform-origin: top right;
}

.menu-title {
  font-size: 12px;
  color: var(--text-dim);
  padding: 6px 8px;
}

/* 菜单空状态（还没有任何账户） */
.empty-tip {
  font-size: 12px;
  color: var(--text-dim);
  text-align: center;
  padding: 10px 8px;
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
