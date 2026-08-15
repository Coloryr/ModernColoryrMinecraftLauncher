// @ts-nocheck — 真实实现仅作参考：依赖 @tauri-apps/api，接入后端时启用
// MCML 前端 API 封装（真实实现，接入后端时启用）
//
// 当前 `src/lib/api.ts` 是界面开发阶段的模拟实现；
// 恢复后端后，把 `App.vue` 的导入从 `./lib/api` 改回本文件即可，
// 同时恢复 `src-tauri` 中对应的 Rust 命令。
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  ErrorEvent,
  ExitEvent,
  InstanceInfo,
  JavaInfo,
  LogEvent,
  StateEvent,
  VersionInfo,
} from "./types";

export const api = {
  initCore: (localDir: string | null, userName: string) =>
    invoke<string>("init_core", { localDir, userName }),
  getInstances: () => invoke<InstanceInfo[]>("get_instances"),
  getJavaList: () => invoke<JavaInfo[]>("get_java_list"),
  getVersions: () => invoke<VersionInfo[]>("get_versions"),
  createInstance: (name: string, version: string) =>
    invoke<InstanceInfo>("create_instance", { name, version }),
  launchGame: (uuid: string, userName: string) =>
    invoke<void>("launch_game", { uuid, userName }),
  stopGame: (uuid: string) => invoke<void>("stop_game", { uuid }),
  getGameLog: (uuid: string) => invoke<string[]>("get_game_log", { uuid }),
  getRunning: () => invoke<string[]>("get_running_instances"),
};

export function onGameLog(cb: (e: LogEvent) => void): Promise<UnlistenFn> {
  return listen<LogEvent>("game-log", (e) => cb(e.payload));
}
export function onLaunchState(cb: (e: StateEvent) => void): Promise<UnlistenFn> {
  return listen<StateEvent>("launch-state", (e) => cb(e.payload));
}
export function onGameExit(cb: (e: ExitEvent) => void): Promise<UnlistenFn> {
  return listen<ExitEvent>("game-exit", (e) => cb(e.payload));
}
export function onLaunchError(cb: (e: ErrorEvent) => void): Promise<UnlistenFn> {
  return listen<ErrorEvent>("launch-error", (e) => cb(e.payload));
}
export function onInstanceChange(cb: () => void): Promise<UnlistenFn> {
  return listen("instance-change", cb);
}
