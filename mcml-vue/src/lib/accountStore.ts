// 账户共享存储（主窗口、账户选择器、账户窗口共用）
// 数据由 Rust 提供（windows/account.rs，持久化 accounts.json），
// 操作通过 IPC；跨窗口通过 account-change 事件同步。
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { Account } from "./types";

export const accounts = ref<Account[]>([]);
export const currentAccount = ref<Account | null>(null);

/** 账户类型显示名 */
export const ACCOUNT_TYPES: Array<{ value: string; labelKey: string }> = [
  { value: "offline", labelKey: "account.typeOffline" },
  { value: "microsoft", labelKey: "account.typeMicrosoft" },
  { value: "littleskin", labelKey: "account.typeLittleSkin" },
  { value: "authlib", labelKey: "account.typeAuthlib" },
  { value: "nide8", labelKey: "account.typeNide8" },
];

export function typeLabelKey(type: string): string {
  return ACCOUNT_TYPES.find((t) => t.value === type)?.labelKey ?? "account.typeOffline";
}

interface AccountStoreView {
  accounts: Account[];
  currentUuid: string | null;
}

/** 从 Rust 加载账户列表 */
export async function loadAccounts(): Promise<void> {
  try {
    const view = await invoke<AccountStoreView>("account_get_accounts");
    accounts.value = view.accounts;
    currentAccount.value =
      view.accounts.find((a) => a.uuid === view.currentUuid) ??
      view.accounts[0] ??
      null;
  } catch {
    /* 浏览器环境：保持为空 */
  }
}

/** 设置当前使用账户 */
export async function setCurrentAccount(acc: Account) {
  try {
    await invoke("account_set_current_account", { uuid: acc.uuid });
  } catch {
    /* 忽略 */
  }
  currentAccount.value = acc;
}

/** 删除账户 */
export async function removeAccount(uuid: string) {
  try {
    await invoke("account_remove_account", { uuid });
  } catch {
    /* 忽略 */
  }
  const idx = accounts.value.findIndex((a) => a.uuid === uuid);
  if (idx >= 0) accounts.value.splice(idx, 1);
  if (currentAccount.value?.uuid === uuid) {
    currentAccount.value = accounts.value[0] ?? null;
  }
}

/** 刷新账户 Token */
export async function refreshAccountToken(uuid: string) {
  try {
    await invoke("account_refresh_account_token", { uuid });
  } catch {
    /* 忽略 */
  }
  const acc = accounts.value.find((a) => a.uuid === uuid);
  if (acc) acc.tokenStatus = "valid";
}

/** 添加账户（真实由 Rust 创建），返回创建的账户；失败返回 null */
export async function addAccount(type: string, name: string): Promise<Account | null> {
  try {
    const acc = await invoke<Account>("account_add_account", { name, accountType: type });
    accounts.value.push(acc);
    if (!currentAccount.value) currentAccount.value = acc;
    return acc;
  } catch {
    return null;
  }
}

// 跨窗口同步：某个窗口改了账户后，其它窗口重新加载
listen("account-change", () => loadAccounts());
