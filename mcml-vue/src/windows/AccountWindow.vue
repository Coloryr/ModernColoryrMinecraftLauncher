<script setup lang="ts">
// 账户管理窗口：平铺 / 列表 / 详情 三种展示 + 类型筛选 + 搜索 + 添加账户
import { computed, ref } from "vue";
import WindowFrame from "../components/ui/WindowFrame.vue";
import BaseButton from "../components/ui/BaseButton.vue";
import BaseModal from "../components/ui/BaseModal.vue";
import SegmentedTabs from "../components/ui/SegmentedTabs.vue";
import { t } from "../lib/i18n";
import {
  ACCOUNT_TYPES,
  accounts,
  addAccount,
  currentAccount,
  removeAccount,
  refreshAccountToken,
  setCurrentAccount,
  typeLabelKey,
} from "../lib/accountStore";
import { avatarImage, capeImage, skinImage } from "../lib/accountImages";
import AccountActions from "../components/AccountActions.vue";
import type { Account } from "../lib/types";

type ViewMode = "grid" | "list" | "detail";
const view = ref<ViewMode>("grid");

const VIEW_OPTIONS = computed(() => [
  { value: "grid", label: t("account.view.grid"), icon: "grid" },
  { value: "list", label: t("account.view.list"), icon: "list" },
  { value: "detail", label: t("account.view.detail"), icon: "folder" },
]);

// 类型筛选 + 搜索
const typeFilter = ref("all");
const searchText = ref("");

const filtered = computed(() => {
  const q = searchText.value.trim().toLowerCase();
  return accounts.value.filter((a) => {
    if (typeFilter.value !== "all" && a.type !== typeFilter.value) return false;
    if (q) {
      return [a.name, a.uuid, a.type].some((s) => s.toLowerCase().includes(q));
    }
    return true;
  });
});

// 添加账户弹窗（按类型显示不同输入框）
const showAdd = ref(false);
const addType = ref("offline");
const addFields = ref<Record<string, string>>({});
const showOauth = ref(false);
const oauthCode = ref("ABCD-EFGH");
const oauthUrl = ref("https://www.microsoft.com/link");

interface AddField {
  key: string;
  labelKey: string;
  password?: boolean;
}

const ADD_FIELDS: Record<string, AddField[]> = {
  offline: [{ key: "name", labelKey: "account.name" }],
  microsoft: [],
  littleskin: [
    { key: "server", labelKey: "account.server" },
    { key: "name", labelKey: "account.username" },
    { key: "pass", labelKey: "account.password", password: true },
  ],
  authlib: [
    { key: "name", labelKey: "account.username" },
    { key: "pass", labelKey: "account.password", password: true },
  ],
  nide8: [
    { key: "serverId", labelKey: "account.serverId" },
    { key: "name", labelKey: "account.username" },
    { key: "pass", labelKey: "account.password", password: true },
  ],
};

function openAdd() {
  showAdd.value = true;
  addType.value = "offline";
  addFields.value = {};
}

function onAddTypeChange(value: string) {
  addType.value = value;
  addFields.value = {};
}

function confirmAdd() {
  const fields = ADD_FIELDS[addType.value] ?? [];
  if (fields.some((f) => !addFields.value[f.key]?.trim())) {
    showToast(t("account.fieldsRequired"));
    return;
  }

  if (addType.value === "microsoft") {
    // 微软：设备码流程弹窗
    showAdd.value = false;
    oauthCode.value = "ABCD-EFGH";
    showOauth.value = true;
    return;
  }

  const name = addFields.value.name?.trim() || "Player";
  addAccount(addType.value, name);
  showAdd.value = false;
  showToast(t("account.added"));
}

// 微软授权弹窗
function openBrowser() {
  showToast(t("account.openBrowser"));
  // 模拟：等待授权后添加账户
  setTimeout(() => {
    addAccount("microsoft", "MS_User_" + Date.now().toString().slice(-4));
    showOauth.value = false;
    showToast(t("account.added"));
  }, 1200);
}

/** 双击切换当前账户 */
function switchAccount(acc: Account) {
  setCurrentAccount(acc);
  showToast(t("account.switched", { name: acc.name }));
}

function isCurrent(acc: Account): boolean {
  return acc.uuid === currentAccount.value.uuid;
}

// 图片种子
function seedOf(uuid: string): number {
  let h = 0;
  for (const c of uuid) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return h;
}

// 操作
const toast = ref("");
let toastTimer: number | undefined;

function showToast(msg: string) {
  toast.value = msg;
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => {
    toast.value = "";
  }, 2000);
}

function refreshToken(acc: Account) {
  refreshAccountToken(acc.uuid);
  showToast(t("account.refreshed"));
}

function relogin(acc: Account) {
  showToast(t("actions.wip", { name: acc.name }));
}

const deleteTarget = ref<Account | null>(null);

function confirmDelete() {
  if (!deleteTarget.value) return;
  removeAccount(deleteTarget.value.uuid);
  showToast(t("account.removed"));
  deleteTarget.value = null;
}

const TYPE_OPTIONS = computed(() => [
  { value: "all", label: t("account.all") },
  ...ACCOUNT_TYPES.map((x) => ({ value: x.value, label: t(x.labelKey) })),
]);

function typeLabel(acc: Account): string {
  return t(typeLabelKey(acc.type));
}

function tokenLabel(acc: Account): string {
  return acc.tokenStatus === "valid" ? t("account.tokenValid") : t("account.tokenExpired");
}
</script>

<template>
  <WindowFrame :title="t('account.manage')" @close="$emit('close')">
    <!-- 工具栏 -->
    <div class="toolbar">
      <div class="toolbar-left">
        <select v-model="typeFilter" class="toolbar-select">
          <option v-for="o in TYPE_OPTIONS" :key="o.value" :value="o.value">{{ o.label }}</option>
        </select>
        <div class="search-box">
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3.5-3.5" />
          </svg>
          <input v-model="searchText" class="search-input" :placeholder="t('account.search')" spellcheck="false" />
          <button v-if="searchText" class="search-clear" @click="searchText = ''">✕</button>
        </div>
      </div>
      <div class="toolbar-right">
        <SegmentedTabs
          :model-value="view"
          :options="VIEW_OPTIONS"
          @update:model-value="view = $event as ViewMode"
        />
        <BaseButton variant="accent" size="sm" @click="openAdd">＋ {{ t("account.add") }}</BaseButton>
      </div>
    </div>

    <!-- 平铺：皮肤 / 头像 / 披风 三图卡片 -->
    <div v-if="view === 'grid'" class="acc-grid">
      <div
        v-for="acc in filtered"
        :key="acc.uuid"
        class="acc-card"
        :class="{ current: isCurrent(acc) }"
        @dblclick="switchAccount(acc)"
      >
        <div class="acc-images">
          <span v-if="isCurrent(acc)" class="current-badge">{{ t("account.current") }}</span>
          <img :src="avatarImage(seedOf(acc.uuid), acc.skin)" class="img-avatar" :alt="t('account.avatar')" />
          <img :src="skinImage(seedOf(acc.uuid), acc.skin)" class="img-skin" :alt="t('account.skin')" />
          <img :src="capeImage(seedOf(acc.uuid), acc.skin)" class="img-cape" :alt="t('account.cape')" />
        </div>
        <div class="acc-head">
          <span class="acc-name">{{ acc.name }}</span>
          <span class="acc-type" :class="acc.type">{{ typeLabel(acc) }}</span>
          <AccountActions
            @refresh="refreshToken(acc)"
            @relogin="relogin(acc)"
            @delete="deleteTarget = acc"
          />
        </div>
      </div>
      <div v-if="filtered.length === 0" class="empty-tip">{{ t("account.searchEmpty") }}</div>
    </div>

    <!-- 列表：头像 / 名字 / 类型 / UUID + 操作 -->
    <div v-else-if="view === 'list'" class="acc-list">
      <div
        v-for="acc in filtered"
        :key="acc.uuid"
        class="acc-row"
        :class="{ current: isCurrent(acc) }"
        @dblclick="switchAccount(acc)"
      >
        <img :src="avatarImage(seedOf(acc.uuid), acc.skin)" class="row-avatar" alt="" />
        <div class="row-meta">
          <span class="row-name">{{ acc.name }}</span>
          <span class="row-sub">{{ typeLabel(acc) }} · {{ acc.uuid }}</span>
        </div>
        <span v-if="isCurrent(acc)" class="current-tag">{{ t("account.current") }}</span>
        <span class="token-tag" :class="acc.tokenStatus">{{ tokenLabel(acc) }}</span>
        <AccountActions
          @refresh="refreshToken(acc)"
          @relogin="relogin(acc)"
          @delete="deleteTarget = acc"
        />
      </div>
      <div v-if="filtered.length === 0" class="empty-tip">{{ t("account.searchEmpty") }}</div>
    </div>

    <!-- 详情：表格展示所有账户，操作按钮在表格左边 -->
    <div v-else class="acc-detail">
      <table class="detail-table wide">
        <thead>
          <tr>
            <th class="col-actions">{{ t("account.actions") }}</th>
            <th>{{ t("account.name") }}</th>
            <th>{{ t("account.uuid") }}</th>
            <th>{{ t("account.type") }}</th>
            <th>{{ t("account.lastLogin") }}</th>
            <th>{{ t("account.tokenStatus") }}</th>
            <th>{{ t("account.capeName") }}</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="acc in filtered"
            :key="acc.uuid"
            :class="{ current: isCurrent(acc) }"
            @dblclick="switchAccount(acc)"
          >
            <td class="col-actions">
              <AccountActions
                @refresh="refreshToken(acc)"
                @relogin="relogin(acc)"
                @delete="deleteTarget = acc"
              />
            </td>
            <td class="cell-name">
              {{ acc.name }}
              <span v-if="isCurrent(acc)" class="current-tag">{{ t("account.current") }}</span>
            </td>
            <td class="mono">{{ acc.uuid }}</td>
            <td>{{ typeLabel(acc) }}</td>
            <td>{{ acc.lastLogin }}</td>
            <td>
              <span class="token-tag" :class="acc.tokenStatus">{{ tokenLabel(acc) }}</span>
            </td>
            <td>{{ acc.name }}_cape</td>
          </tr>
        </tbody>
      </table>
      <div v-if="filtered.length === 0" class="empty-tip">{{ t("account.searchEmpty") }}</div>
    </div>

    <!-- 添加账户弹窗（按类型显示不同输入框） -->
    <BaseModal
      v-if="showAdd"
      :title="t('account.addTitle')"
      :closable="false"
      @close="showAdd = false"
    >
      <label class="field-label">{{ t("account.type") }}</label>
      <select v-model="addType" class="field-select" @change="onAddTypeChange(($event.target as HTMLSelectElement).value)">
        <option v-for="x in ACCOUNT_TYPES" :key="x.value" :value="x.value">{{ t(x.labelKey) }}</option>
      </select>

      <template v-for="f in ADD_FIELDS[addType] ?? []" :key="f.key">
        <label class="field-label">{{ t(f.labelKey) }}</label>
        <input
          v-model="addFields[f.key]"
          class="field-input"
          :type="f.password ? 'password' : 'text'"
          spellcheck="false"
          @keyup.enter="confirmAdd"
        />
      </template>

      <p v-if="addType === 'microsoft'" class="hint">{{ t("account.loginHint") }}</p>

      <div class="modal-actions">
        <BaseButton @click="showAdd = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" @click="confirmAdd">{{ t("account.add") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 微软登录：请求码 + 地址 + 打开浏览器 / 取消 -->
    <BaseModal
      v-if="showOauth"
      :title="t('account.oauthTitle')"
      :closable="false"
      @close="showOauth = false"
    >
      <label class="field-label">{{ t("account.oauthCode") }}</label>
      <div class="oauth-code">{{ oauthCode }}</div>

      <label class="field-label">{{ t("account.oauthUrl") }}</label>
      <div class="oauth-url">{{ oauthUrl }}</div>

      <p class="hint">{{ t("account.oauthHint") }}</p>

      <div class="modal-actions">
        <BaseButton @click="showOauth = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" @click="openBrowser">{{ t("account.openBrowser") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 删除确认 -->
    <BaseModal v-if="deleteTarget" :title="t('account.delete')" @close="deleteTarget = null">
      <p class="delete-tip">{{ t("account.deleteConfirm", { name: deleteTarget.name }) }}</p>
      <div class="modal-actions">
        <BaseButton @click="deleteTarget = null">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" @click="confirmDelete">{{ t("actions.confirm") }}</BaseButton>
      </div>
    </BaseModal>

    <Transition name="toast">
      <div v-if="toast" class="toast">{{ toast }}</div>
    </Transition>
  </WindowFrame>
</template>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 14px;
}

.toolbar-left {
  display: flex;
  align-items: center;
  gap: 10px;
}

.toolbar-right {
  display: flex;
  align-items: center;
  gap: 10px;
}

.toolbar-select {
  min-width: 170px;
  padding: 8px 28px 8px 12px;
  border-radius: 9px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text);
  font-size: 13px;
  outline: none;
  font-family: inherit;
  appearance: none;
  background-image: url("data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%239aa3af' stroke-width='2' stroke-linecap='round'%3E%3Cpath d='m6 9 6 6 6-6'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  background-size: 12px;
}

.grow {
  flex: 1;
  min-width: 140px;
}

.search-box {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 10px;
  height: 34px;
  border-radius: 9px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-dim);
  width: 220px;
}

.search-input {
  flex: 1;
  min-width: 0;
  border: none;
  background: transparent;
  color: var(--text);
  font-size: 12.5px;
  outline: none;
  font-family: inherit;
}

.search-clear {
  border: none;
  background: transparent;
  color: var(--text-dim);
  font-size: 12px;
  cursor: pointer;
}

/* 平铺 */
.acc-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
}

.acc-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.acc-images {
  position: relative;
  display: flex;
  align-items: flex-end;
  gap: 12px;
  justify-content: center;
  padding: 18px 0;
  background: var(--bg-side);
  border-radius: 12px;
}

/* 当前账户角标 */
.current-badge {
  position: absolute;
  top: 8px;
  left: 8px;
  font-size: 10.5px;
  padding: 2px 9px;
  border-radius: 20px;
  background: var(--accent);
  color: #fff;
  white-space: nowrap;
}

.img-avatar {
  width: 72px;
  height: 72px;
  border-radius: 8px;
  image-rendering: pixelated;
}

.img-skin {
  width: 36px;
  height: 72px;
  border-radius: 5px;
  image-rendering: pixelated;
}

.img-cape {
  width: 72px;
  height: 36px;
  border-radius: 5px;
  image-rendering: pixelated;
}

.acc-head .acc-name {
  font-size: 15px;
}

.acc-head {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}

.acc-name {
  flex: 1;
  min-width: 0;
  font-size: 14px;
  font-weight: 700;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.acc-type {
  flex-shrink: 0;
}

/* 当前选中账户 */
.acc-card.current {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}

.acc-row.current {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}

.detail-table tbody tr.current td {
  background: var(--accent-soft);
}

.current-tag {
  font-size: 10.5px;
  padding: 2px 8px;
  border-radius: 20px;
  background: var(--accent);
  color: #fff;
  white-space: nowrap;
  flex-shrink: 0;
}

.acc-type {
  font-size: 10.5px;
  padding: 2px 8px;
  border-radius: 20px;
  white-space: nowrap;
  background: var(--bg-hover);
  color: var(--text-dim);
  flex-shrink: 0;
}

.acc-type.microsoft {
  background: rgba(63, 140, 255, 0.16);
  color: #8fb0ff;
}

.acc-type.littleskin {
  background: rgba(168, 85, 247, 0.16);
  color: #c084fc;
}

.acc-type.authlib {
  background: rgba(245, 158, 11, 0.16);
  color: var(--yellow);
}

.acc-type.nide8 {
  background: rgba(6, 182, 212, 0.16);
  color: #22d3ee;
}

/* 列表 */
.acc-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.acc-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  cursor: default;
}

.row-avatar {
  width: 40px;
  height: 40px;
  border-radius: 6px;
  image-rendering: pixelated;
  flex-shrink: 0;
}

.row-meta {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 3px;
}

.row-name {
  font-size: 14px;
  font-weight: 700;
}

.row-sub {
  font-size: 11.5px;
  color: var(--text-dim);
  font-family: "Cascadia Code", Consolas, monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.token-tag {
  font-size: 11px;
  padding: 2px 9px;
  border-radius: 20px;
  white-space: nowrap;
  flex-shrink: 0;
}

.token-tag.valid {
  background: rgba(62, 207, 142, 0.14);
  color: var(--green);
}

.token-tag.expired {
  background: rgba(255, 95, 86, 0.14);
  color: var(--red);
}

/* 详情：全账户表格，操作按钮在左侧 */
.acc-detail {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.detail-table.wide {
  border-collapse: collapse;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
  width: 100%;
}

.detail-table th,
.detail-table td {
  padding: 10px 14px;
  font-size: 12.5px;
  border-bottom: 1px solid var(--border);
  text-align: left;
  white-space: nowrap;
}

.detail-table thead th {
  color: var(--text-dim);
  font-weight: 600;
  background: var(--bg-side);
}

.detail-table tbody tr:last-child th,
.detail-table tbody tr:last-child td {
  border-bottom: none;
}

.col-actions {
  width: 108px;
}

.oauth-code {
  font-size: 18px;
  font-weight: 800;
  letter-spacing: 4px;
  font-family: "Cascadia Code", Consolas, monospace;
  color: var(--accent);
  background: var(--bg-side);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px 14px;
  text-align: center;
}

.oauth-url {
  font-size: 13px;
  color: var(--accent);
  background: var(--bg-side);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 10px 14px;
  word-break: break-all;
}

.cell-name {
  font-weight: 700;
}

.hint {
  font-size: 12px;
  color: var(--text-dim);
  margin: 10px 0 0;
}

.mono {
  font-family: "Cascadia Code", Consolas, monospace;
}

.delete-tip {
  font-size: 13.5px;
  color: var(--text);
  line-height: 1.7;
  margin: 6px 0 2px;
}

.modal-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  margin-top: 18px;
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
