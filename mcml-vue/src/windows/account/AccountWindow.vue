<script setup lang="ts">
// 账户管理窗口：平铺 / 列表 / 详情 三种展示 + 类型筛选 + 搜索 + 添加账户
import { computed, onMounted, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import { t } from "../../lib/i18n";
import { showToast } from "../../lib/toast";
import AccountGrid from "./views/AccountGrid.vue";
import AccountList from "./views/AccountList.vue";
import AccountDetail from "./views/AccountDetail.vue";
import {
  ACCOUNT_TYPES,
  accounts,
  addAccount,
  currentAccount,
  loadAccounts,
  removeAccount,
  refreshAccountToken,
  setCurrentAccount,
  typeLabelKey,
} from "../../lib/accountStore";
import type { Account } from "../../lib/types";
import { listen } from "@tauri-apps/api/event";
import { AccountOAuthDto } from "../../lib/dtos/account.ts";
import { getCurrentWindow } from "@tauri-apps/api/window";

await getCurrentWindow().setTitle(t("winTitle.account"));

type ViewMode = "grid" | "list" | "detail";
const view = ref<ViewMode>("grid");

// 进入窗口时从 Rust 加载账户数据
onMounted(loadAccounts);

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
    if (typeFilter.value !== "all" && a.authType !== typeFilter.value) return false;
    if (q) {
      return [a.userName, a.uuid, a.authType].some((s) => s.toLowerCase().includes(q));
    }
    return true;
  });
});

// 添加账户弹窗（按类型显示不同输入框）
const showAdd = ref(false);
const addType = ref("offline");
const addFields = ref<Record<string, string>>({});
const showOauth = ref(false);
const showOauthRun = ref(false);
const oauthCode = ref("");
const oauthUrl = ref("");
const oauthState = ref("");

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
  showToast(t("account.switched", { name: acc.userName }));
}

// 图片种子
function seedOf(uuid: string): number {
  let h = 0;
  for (const c of uuid) h = (h * 31 + c.charCodeAt(0)) >>> 0;
  return h;
}

// 操作
function refreshToken(acc: Account) {
  refreshAccountToken(acc.uuid);
  showToast(t("account.refreshed"));
}

function relogin(acc: Account) {
  showToast(t("actions.wip", { name: acc.userName }));
}

function cancelLogin(type: string) {
  if (type == "showOauthRun") {
    showOauthRun.value = false
  }
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
  return t(typeLabelKey(acc.authType));
}

function tokenLabel(acc: Account): string {
  return acc.tokenStatus === "valid" ? t("account.tokenValid") : t("account.tokenExpired");
}

listen<AccountOAuthDto>(AccountOAuth, (data) => {
  // 微软：设备码流程弹窗
  showAdd.value = false;
  oauthCode.value = data.payload.code;
  oauthUrl.value = data.payload.url;
  showOauth.value = true;
});
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
          <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2"
            stroke-linecap="round">
            <circle cx="11" cy="11" r="7" />
            <path d="m20 20-3.5-3.5" />
          </svg>
          <input v-model="searchText" class="search-input" :placeholder="t('account.search')" spellcheck="false" />
          <button v-if="searchText" class="search-clear" @click="searchText = ''">✕</button>
        </div>
      </div>
      <div class="toolbar-right">
        <SegmentedTabs :model-value="view" :options="VIEW_OPTIONS" @update:model-value="view = $event as ViewMode" />
        <BaseButton variant="accent" size="sm" @click="openAdd">＋ {{ t("account.add") }}</BaseButton>
      </div>
    </div>

    <!-- 视图：平铺 / 列表 / 详情（见 views/ 目录） -->
    <AccountGrid v-if="view === 'grid'" :accounts="filtered" :current-uuid="currentAccount?.uuid ?? ''"
      :type-label="typeLabel" :seed-of="seedOf" @switch="switchAccount" @refresh="refreshToken" @relogin="relogin"
      @delete="deleteTarget = $event" />
    <AccountList v-else-if="view === 'list'" :accounts="filtered" :current-uuid="currentAccount?.uuid ?? ''"
      :type-label="typeLabel" :token-label="tokenLabel" :seed-of="seedOf" @switch="switchAccount"
      @refresh="refreshToken" @relogin="relogin" @delete="deleteTarget = $event" />
    <AccountDetail v-else :accounts="filtered" :current-uuid="currentAccount?.uuid ?? ''" :type-label="typeLabel"
      :token-label="tokenLabel" @switch="switchAccount" @refresh="refreshToken" @relogin="relogin"
      @delete="deleteTarget = $event" />

    <!-- 添加账户弹窗（按类型显示不同输入框） -->
    <BaseModal v-if="showAdd" :title="t('account.addTitle')" :closable="false" @close="showAdd = false; cancelLogin">
      <label class="field-label">{{ t("account.type") }}</label>
      <select v-model="addType" class="field-select"
        @change="onAddTypeChange(($event.target as HTMLSelectElement).value)">
        <option v-for="x in ACCOUNT_TYPES" :key="x.value" :value="x.value">{{ t(x.labelKey) }}</option>
      </select>

      <template v-for="f in ADD_FIELDS[addType] ?? []" :key="f.key">
        <label class="field-label">{{ t(f.labelKey) }}</label>
        <input v-model="addFields[f.key]" class="field-input" :type="f.password ? 'password' : 'text'"
          spellcheck="false" @keyup.enter="confirmAdd" />
      </template>

      <p v-if="addType === 'microsoft'" class="hint">{{ t("account.loginHint") }}</p>

      <div class="modal-actions">
        <BaseButton @click="showAdd = false">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" @click="confirmAdd">{{ t("account.add") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 微软登录：请求码 + 地址 + 打开浏览器 / 取消 -->
    <BaseModal v-if="showOauth" :title="t('account.oauthTitle')" :closable="false"
      @close="showOauth = false; cancelLogin">
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

    <BaseModal v-if="showOauthRun" :title="t('account.oauthTitle')" :closable="false" @close="showOauthRun = false">
      <p class="hint">{{ oauthState }}</p>

      <div class="modal-actions">
        <BaseButton @click="cancelLogin('showOauthRun')">{{ t("add.cancel") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 删除确认 -->
    <BaseModal v-if="deleteTarget" :title="t('account.delete')" @close="deleteTarget = null">
      <p class="delete-tip">{{ t("account.deleteConfirm", { name: deleteTarget.userName }) }}</p>
      <div class="modal-actions">
        <BaseButton @click="deleteTarget = null">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" @click="confirmDelete">{{ t("actions.confirm") }}</BaseButton>
      </div>
    </BaseModal>
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

.hint {
  font-size: 12px;
  color: var(--text-dim);
  margin: 10px 0 0;
}

.mono {
  font-family: "Cascadia Code", Consolas, monospace;
}
</style>
