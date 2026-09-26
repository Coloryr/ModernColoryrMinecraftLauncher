<script setup lang="ts">
// 账户管理窗口：平铺 / 列表 / 详情 三种展示 + 类型筛选 + 搜索 + 添加账户
import { computed, onMounted, ref } from "vue";
import WindowFrame from "../../components/ui/WindowFrame.vue";
import BaseButton from "../../components/ui/BaseButton.vue";
import BaseModal from "../../components/ui/BaseModal.vue";
import SegmentedTabs from "../../components/ui/SegmentedTabs.vue";
import { t, tErr } from "../../lib/i18n";
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
  loginMicrosoft,
  removeAccount,
  refreshAccountToken,
  setCurrentAccount,
} from "../../lib/accountStore";
import type { AccountStoreDto } from "../../lib/bindings";
import { listen } from "@tauri-apps/api/event";
import { AccountOAuth, AccountOAuthState } from "../../lib/listens";
import { commands } from "../../lib/bindings";
import type { AccountOAuthDto, AccountOAuthStateDto } from "../../lib/bindings";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { openWindow } from "../windowManager";

// 注意：不能用顶层 await —— 会让 <script setup> 变成 async setup，
// App.vue 没有 <Suspense> 包裹，Vue 将不渲染该组件（窗口白屏）
getCurrentWindow().setTitle(t("winTitle.account")).catch(() => { /* 忽略 */ });

type ViewMode = "grid" | "list" | "detail";
const view = ref<ViewMode>("grid");

// 进入窗口时从 Rust 加载账户数据，并注册微软登录事件
onMounted(() => {
  loadAccounts();

  // 后端拿到设备码：弹出授权码窗口
  listen<AccountOAuthDto>(AccountOAuth, (data) => {
    showAdd.value = false;
    oauthCode.value = data.payload.code;
    oauthUrl.value = data.payload.url;
    showOauth.value = true;
  });

  // 登录阶段进度：waiting / xbox / xsts / token / profile / ok / fail
  listen<AccountOAuthStateDto>(AccountOAuthState, (data) => {
    const s = data.payload;
    if (s.state === "ok") {
      showOauth.value = false;
      showOauthRun.value = false;
      showToast(t("account.oauthOk"));
      loadAccounts();
    } else if (s.state === "fail") {
      showOauth.value = false;
      showOauthRun.value = false;
      showToast(s.message || t("account.oauthFail"));
    } else {
      oauthState.value = t(`account.oauthState.${s.state}`);
      // 授权码弹窗还开着（用户尚未点「打开浏览器」）时不抢焦点，只静默更新文案
      if (!showOauth.value) {
        showOauthRun.value = true;
      }
    }
  });
});

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
  /** 可选字段（如 LittleSkin 的服务器地址：留空即官方站） */
  optional?: boolean;
}

const ADD_FIELDS: Record<string, AddField[]> = {
  offline: [{ key: "name", labelKey: "account.name" }],
  microsoft: [],
  littleskin: [
    { key: "name", labelKey: "account.username" },
    { key: "pass", labelKey: "account.password", password: true },
  ],
  authlib: [
    { key: "server", labelKey: "account.server" },
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

/** 添加弹窗是否在处理中（防止重复提交） */
const adding = ref(false);

async function confirmAdd() {
  if (adding.value) return;
  const fields = ADD_FIELDS[addType.value] ?? [];
  if (fields.some((f) => !f.optional && !addFields.value[f.key]?.trim())) {
    showToast(t("account.fieldsRequired"));
    return;
  }

  adding.value = true;
  try {
    if (addType.value === "microsoft") {
      // 触发后端设备码流程，弹窗由 account-oauth 事件接管；失败提示后弹窗保留
      await loginMicrosoft();
    } else {
      const server = (addFields.value.server ?? addFields.value.serverId)?.trim();
      await addAccount(
        addType.value,
        addFields.value.name!.trim(),
        server || undefined,
        addFields.value.pass,
      );
      showAdd.value = false;
      showToast(t("account.added"));
    }
  } catch (e) {
    showToast(String(e));
  } finally {
    adding.value = false;
  }
}

// 微软授权弹窗：打开浏览器并切换到进度窗口
async function openBrowser() {
  await commands.account.openBrowser(oauthUrl.value).catch(() => { /* 忽略 */ });
  showOauth.value = false;
  showOauthRun.value = true;
  oauthState.value = t("account.oauthState.waiting");
}

/** 复制到剪贴板：优先异步 Clipboard API，失败回退 execCommand */
async function copyText(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.style.position = "fixed";
    ta.style.opacity = "0";
    document.body.appendChild(ta);
    ta.select();
    const ok = document.execCommand("copy");
    ta.remove();
    return ok;
  }
}

/** 点击登录码 / 网址复制 */
async function copyOauthValue(value: string) {
  if (!value) return;
  showToast(await copyText(value) ? t("account.copied") : t("account.copyFail"));
}

/** 双击切换当前账户 */
function switchAccount(acc: AccountStoreDto) {
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
function refreshToken(acc: AccountStoreDto) {
  refreshAccountToken(acc.uuid);
  showToast(t("account.refreshed"));
}

/** 重新登录：微软账户直接走设备码流程；其余类型弹窗重输密码重新认证 */
function relogin(acc: AccountStoreDto) {
  if (acc.authType === "microsoft") {
    loginMicrosoft().catch((e) => showToast(String(e)));
    return;
  }
  reloginTarget.value = acc;
  reloginFields.value = { server: acc.server ?? "", name: acc.userName, pass: "" };
}

const reloginTarget = ref<AccountStoreDto | null>(null);
const reloginFields = ref({ server: "", name: "", pass: "" });

/** 防止重复提交 */
const relogining = ref(false);

async function confirmRelogin() {
  const acc = reloginTarget.value;
  if (!acc || relogining.value) return;
  if (!reloginFields.value.name.trim() || !reloginFields.value.pass.trim()) {
    showToast(t("account.fieldsRequired"));
    return;
  }

  relogining.value = true;
  try {
    // 同 uuid + 类型会覆盖旧账户数据，等效重新认证
    const server = reloginFields.value.server.trim();
    await addAccount(acc.authType, reloginFields.value.name.trim(), server || undefined, reloginFields.value.pass);
    reloginTarget.value = null;
    showToast(t("account.reloginOk"));
  } catch (e) {
    showToast(String(e));
  } finally {
    relogining.value = false;
  }
}

// 取消微软登录：通知后端终止轮询并关闭弹窗
async function cancelLogin() {
  await commands.account.cancelOAuth().catch(() => { /* 忽略 */ });
  showOauth.value = false;
  showOauthRun.value = false;
}

const deleteTarget = ref<AccountStoreDto | null>(null);

function confirmDelete() {
  if (!deleteTarget.value) return;
  removeAccount(deleteTarget.value.uuid);
  showToast(t("account.removed"));
  deleteTarget.value = null;
}

// 编辑离线账户：改名 / 改 UUID（UUID 右边可一键随机）
const editTarget = ref<AccountStoreDto | null>(null);
const editFields = ref({ name: "", uuid: "" });
const editing = ref(false);

function openEdit(acc: AccountStoreDto) {
  editTarget.value = acc;
  editFields.value = { name: acc.userName, uuid: acc.uuid };
}

/** 随机 UUID v4（crypto.randomUUID 不可用时手动拼） */
function randomUuid() {
  if (typeof crypto !== "undefined" && "randomUUID" in crypto) {
    editFields.value.uuid = crypto.randomUUID();
    return;
  }
  const hex = (n: number) =>
    Array.from({ length: n }, () => Math.floor(Math.random() * 16).toString(16)).join("");
  editFields.value.uuid = `${hex(8)}-${hex(4)}-4${hex(3)}-${hex(4)}-${hex(12)}`;
}

async function confirmEdit() {
  const acc = editTarget.value;
  if (!acc || editing.value) return;
  if (!editFields.value.name.trim()) {
    showToast(t("err.nameEmpty"));
    return;
  }
  editing.value = true;
  try {
    await commands.account.editOffline(acc.uuid, editFields.value.name, editFields.value.uuid);
    editTarget.value = null;
    showToast(t("account.edited"));
    loadAccounts();
  } catch (e) {
    showToast(tErr(e));
  } finally {
    editing.value = false;
  }
}

const TYPE_OPTIONS = computed(() => [
  { value: "all", label: t("account.all") },
  ...ACCOUNT_TYPES.map((x) => ({ value: x.value, label: t(x.labelKey) })),
]);

function tokenLabel(acc: AccountStoreDto): string {
  return acc.tokenStatus === "valid" ? t("account.tokenValid") : t("account.tokenExpired");
}
</script>

<template>
  <WindowFrame :title="t('account.manage')" :body-fill="view === 'detail'" @close="$emit('close')">
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
        <!-- 皮肤查看从账户管理进入（不再放主页面顶栏） -->
        <BaseButton size="sm" @click="openWindow('skin')">{{ t("account.viewSkin") }}</BaseButton>
        <BaseButton variant="accent" size="sm" @click="openAdd">＋ {{ t("account.add") }}</BaseButton>
      </div>
    </div>

    <!-- 视图：平铺 / 列表 / 详情（见 views/ 目录）；详情模式下视图区自己管理滚动 -->
    <div class="view-area" :class="{ fill: view === 'detail' }">
      <AccountGrid v-if="view === 'grid'" :accounts="filtered" :current-uuid="currentAccount?.uuid ?? ''"
        @switch="switchAccount" @refresh="refreshToken" @relogin="relogin"
        @edit="openEdit" @delete="deleteTarget = $event" />
      <AccountList v-else-if="view === 'list'" :accounts="filtered" :current-uuid="currentAccount?.uuid ?? ''"
        :token-label="tokenLabel" :seed-of="seedOf" @switch="switchAccount"
        @refresh="refreshToken" @relogin="relogin" @edit="openEdit" @delete="deleteTarget = $event" />
      <AccountDetail v-else :accounts="filtered" :current-uuid="currentAccount?.uuid ?? ''"
        :token-label="tokenLabel" @switch="switchAccount" @refresh="refreshToken" @relogin="relogin"
        @edit="openEdit" @delete="deleteTarget = $event" />
    </div>

    <!-- 添加账户弹窗（按类型显示不同输入框）：不遮标题栏、点空白不关闭 -->
    <BaseModal
      v-if="showAdd"
      :title="t('account.addTitle')"
      :closable="false"
      below-titlebar
      :overlay-close="false"
      @close="showAdd = false"
    >
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
    <BaseModal
      v-if="showOauth"
      :title="t('account.oauthTitle')"
      :closable="false"
      below-titlebar
      :overlay-close="false"
      @close="cancelLogin"
    >
      <label class="field-label">{{ t("account.oauthCode") }}</label>
      <div class="oauth-code copyable" :title="t('account.copy')" @click="copyOauthValue(oauthCode)">
        {{ oauthCode }}
        <svg class="copy-ico" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <rect x="9" y="9" width="12" height="12" rx="2" />
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
        </svg>
      </div>

      <label class="field-label">{{ t("account.oauthUrl") }}</label>
      <div class="oauth-url copyable" :title="t('account.copy')" @click="copyOauthValue(oauthUrl)">
        <span class="oauth-url-text">{{ oauthUrl }}</span>
        <svg class="copy-ico" viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
          <rect x="9" y="9" width="12" height="12" rx="2" />
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
        </svg>
      </div>

      <p class="hint">{{ t("account.oauthHint") }}</p>

      <div class="modal-actions">
        <BaseButton @click="cancelLogin">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" @click="openBrowser">{{ t("account.openBrowser") }}</BaseButton>
      </div>
    </BaseModal>

    <BaseModal
      v-if="showOauthRun"
      :title="t('account.oauthTitle')"
      :closable="false"
      below-titlebar
      :overlay-close="false"
      @close="cancelLogin"
    >
      <p class="hint">{{ oauthState }}</p>

      <div class="modal-actions">
        <BaseButton @click="cancelLogin">{{ t("add.cancel") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 删除确认 -->
    <BaseModal v-if="deleteTarget" :title="t('account.delete')" below-titlebar @close="deleteTarget = null">
      <p class="delete-tip">{{ t("account.deleteConfirm", { name: deleteTarget.userName }) }}</p>
      <div class="modal-actions">
        <BaseButton @click="deleteTarget = null">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="danger" @click="confirmDelete">{{ t("actions.confirm") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 重新登录（非微软账户）：重输密码重新认证，服务器地址预填 -->
    <BaseModal
      v-if="reloginTarget"
      :title="t('account.reloginTitle')"
      :closable="false"
      below-titlebar
      :overlay-close="false"
      @close="reloginTarget = null"
    >
      <template v-if="reloginTarget.authType === 'nide8'">
        <label class="field-label">{{ t("account.serverId") }}</label>
        <input v-model="reloginFields.server" class="field-input" spellcheck="false" />
      </template>
      <template v-else>
        <label class="field-label">{{ t("account.server") }}</label>
        <input v-model="reloginFields.server" class="field-input" spellcheck="false" />
      </template>

      <label class="field-label">{{ t("account.username") }}</label>
      <input v-model="reloginFields.name" class="field-input" spellcheck="false" />

      <label class="field-label">{{ t("account.password") }}</label>
      <input v-model="reloginFields.pass" class="field-input" type="password" @keyup.enter="confirmRelogin" />

      <p class="hint">{{ t("account.reloginHint") }}</p>

      <div class="modal-actions">
        <BaseButton @click="reloginTarget = null">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" @click="confirmRelogin">{{ t("actions.confirm") }}</BaseButton>
      </div>
    </BaseModal>

    <!-- 编辑离线账户：改名 / 改 UUID -->
    <BaseModal
      v-if="editTarget"
      :title="t('account.editOffline')"
      :closable="false"
      below-titlebar
      :overlay-close="false"
      @close="editTarget = null"
    >
      <label class="field-label">{{ t("account.name") }}</label>
      <input v-model="editFields.name" class="field-input" spellcheck="false" />

      <label class="field-label">{{ t("account.uuid") }}</label>
      <div class="uuid-row">
        <input v-model="editFields.uuid" class="field-input grow" spellcheck="false" />
        <BaseButton size="sm" @click="randomUuid">{{ t("account.randomUuid") }}</BaseButton>
      </div>

      <div class="modal-actions">
        <BaseButton @click="editTarget = null">{{ t("add.cancel") }}</BaseButton>
        <BaseButton variant="primary" @click="confirmEdit">{{ t("actions.confirm") }}</BaseButton>
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

/* 详情模式下视图区占满剩余高度，滚动交给表格容器 */
.view-area.fill {
  flex: 1;
  min-height: 0;
  overflow: hidden;
  display: flex;
  flex-direction: column;
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

/* UUID 输入 + 随机按钮 */
.uuid-row {
  display: flex;
  align-items: center;
  gap: 8px;
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

/* 登录码 / 网址可点击复制 */
.copyable {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  cursor: pointer;
  user-select: text;
  transition: border-color 0.15s;
}

.copyable:hover {
  border-color: var(--accent);
}

.oauth-url-text {
  text-align: left;
}

.copy-ico {
  flex-shrink: 0;
  opacity: 0.55;
}

.copyable:hover .copy-ico {
  opacity: 1;
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
