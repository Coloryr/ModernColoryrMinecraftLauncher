// 账户共享存储（主窗口、账户选择器、账户窗口共用）
import { ref } from "vue";
import type { Account } from "./types";

export const accounts = ref<Account[]>([
  {
    uuid: "acc-1",
    name: "Player",
    type: "offline",
    avatarColor: "linear-gradient(135deg, #3f8cff, #5f6cff)",
    skin: "#3f8cff",
    lastLogin: "2026-04-15 21:30",
    tokenStatus: "valid",
  },
  {
    uuid: "acc-2",
    name: "Notch_MS",
    type: "microsoft",
    avatarColor: "linear-gradient(135deg, #34d399, #22d3ee)",
    skin: "#34d399",
    lastLogin: "2026-04-15 19:05",
    tokenStatus: "valid",
  },
  {
    uuid: "acc-3",
    name: "LittleSkin_User",
    type: "littleskin",
    avatarColor: "linear-gradient(135deg, #a855f7, #ec4899)",
    skin: "#a855f7",
    lastLogin: "2026-04-12 20:44",
    tokenStatus: "valid",
  },
  {
    uuid: "acc-4",
    name: "CustomServer",
    type: "authlib",
    avatarColor: "linear-gradient(135deg, #f59e0b, #ef4444)",
    skin: "#f59e0b",
    lastLogin: "2026-04-01 12:10",
    tokenStatus: "expired",
  },
  {
    uuid: "acc-5",
    name: "UnifiedUser",
    type: "nide8",
    avatarColor: "linear-gradient(135deg, #06b6d4, #6366f1)",
    skin: "#06b6d4",
    lastLogin: "2026-03-28 08:22",
    tokenStatus: "expired",
  },
]);

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

/** 当前选中（使用中）的账户 */
export const currentAccount = ref<Account>(accounts.value[0]);

export function setCurrentAccount(acc: Account) {
  const target = accounts.value.find((a) => a.uuid === acc.uuid);
  if (target) currentAccount.value = target;
}

export function removeAccount(uuid: string) {
  const idx = accounts.value.findIndex((a) => a.uuid === uuid);
  if (idx >= 0) accounts.value.splice(idx, 1);
}

export function refreshAccountToken(uuid: string) {
  const acc = accounts.value.find((a) => a.uuid === uuid);
  if (acc) acc.tokenStatus = "valid";
}

/** 添加账户（模拟） */
export function addAccount(type: string, name: string) {
  const palette = [
    ["#3f8cff", "#5f6cff"],
    ["#34d399", "#22d3ee"],
    ["#a855f7", "#ec4899"],
    ["#f59e0b", "#ef4444"],
    ["#06b6d4", "#6366f1"],
  ];
  const [c1, c2] = palette[accounts.value.length % palette.length];
  accounts.value.push({
    uuid: `acc-${Date.now()}`,
    name,
    type,
    avatarColor: `linear-gradient(135deg, ${c1}, ${c2})`,
    skin: c1,
    lastLogin: "刚刚",
    tokenStatus: "valid",
  });
}
