// 账户共享存储（主窗口、账户选择器、账户窗口共用）
// 数据由 Rust 提供（windows/account.rs，持久化 accounts.json），
// 操作通过 IPC；跨窗口通过 account-change 事件同步。
import { ref } from "vue";
import { listen } from "@tauri-apps/api/event";
import type { AccountStoreDto } from "./bindings";
import { AccountChange } from "./listens";
import { commands } from "./bindings";

export const accounts = ref<AccountStoreDto[]>([]);
export const currentAccount = ref<AccountStoreDto | null>(null);

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

/** 从 Rust 加载账户列表 */
export async function loadAccounts(): Promise<void> {
  try {
    const view = await commands.account.getAccounts();
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
export async function setCurrentAccount(acc: AccountStoreDto) {
  try {
    await commands.account.setCurrentAccount(acc.uuid);
  } catch {
    /* 忽略 */
  }
  currentAccount.value = acc;
}

/** 删除账户 */
export async function removeAccount(uuid: string) {
  try {
    await commands.account.removeAccount(uuid);
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
    await commands.account.refreshAccountToken(uuid);
  } catch {
    /* 忽略 */
  }
  const acc = accounts.value.find((a) => a.uuid === uuid);
  if (acc) acc.tokenStatus = "valid";
}

/** 添加账户（真实由 Rust 创建），返回创建的账户；失败返回 null */
export async function addAccount(type: string, name: string): Promise<AccountStoreDto | null> {
  try {
    const acc = await commands.account.addAccount(name, type);
    accounts.value.push(acc);
    if (!currentAccount.value) currentAccount.value = acc;
    return acc;
  } catch {
    return null;
  }
}

/** 微软登录：触发后端设备码流程；弹窗交互与结果经 account-oauth / account-oauth-state 事件通知 */
export async function loginMicrosoft(): Promise<void> {
  try {
    await commands.account.addAccount("", "microsoft");
  } catch {
    /* 进度与错误经 account-oauth-state 事件通知 */
  }
}

// 跨窗口同步：某个窗口改了账户后，其它窗口重新加载
listen(AccountChange, () => loadAccounts());
