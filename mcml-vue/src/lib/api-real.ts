// @ts-nocheck — 早期接入 mcml-core 时的真实实现参考
// 当前 gui 与 core 已解耦（不依赖 core），本文件代码全部注释掉，仅作历史参考。
// 界面数据由 `./api.ts`（模拟实现）提供；如需重新接入核心，
// 取消下方注释，并恢复 src-tauri 中对应的 Rust 命令即可。

/*
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
*/
