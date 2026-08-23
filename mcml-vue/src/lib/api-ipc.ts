// MCML 前端 API（真实 IPC 实现）
//
// 数据全部从 Rust 侧获取（命令见 mcml-gui/src-tauri/src/windows/main.rs），
// 按钮执行的操作也通过 IPC 调用。仅在 Tauri 环境可用（纯浏览器会报错）。
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

export interface CreateInstanceOpts {
  loader?: string;
  loaderVersion?: string | null;
  group?: string | null;
  modpackType?: string;
  source?: string;
}

export const api = {
  /** 初始化核心：返回数据目录 */
  async initCore(localDir: string | null, userName: string): Promise<string> {
    return invoke<string>("init_core", { localDir, userName });
  },

  async getInstances(): Promise<InstanceInfo[]> {
    return invoke<InstanceInfo[]>("get_instances");
  },

  async getGroups(): Promise<string[]> {
    return invoke<string[]>("get_groups");
  },

  async getJavaList(): Promise<JavaInfo[]> {
    return invoke<JavaInfo[]>("get_java_list");
  },

  async getVersions(): Promise<VersionInfo[]> {
    return invoke<VersionInfo[]>("get_versions");
  },

  async addGroup(name: string): Promise<boolean> {
    return invoke<boolean>("add_group", { name });
  },

  async removeGroup(name: string): Promise<boolean> {
    return invoke<boolean>("remove_group", { name });
  },

  async moveGroup(name: string, index: number): Promise<boolean> {
    return invoke<boolean>("move_group", { name, index });
  },

  async createInstance(
    name: string,
    version: string,
    opts?: CreateInstanceOpts,
  ): Promise<InstanceInfo> {
    return invoke<InstanceInfo>("create_instance", {
      name,
      version,
      loader: opts?.loader,
      loaderVersion: opts?.loaderVersion,
      group: opts?.group ?? null,
      modpackType: opts?.modpackType,
      source: opts?.source,
    });
  },

  async renameInstance(uuid: string, name: string): Promise<boolean> {
    return invoke<boolean>("rename_instance", { uuid, name });
  },

  /** 更新实例元信息（补丁式，Partial<InstanceInfo>） */
  async updateInstance(uuid: string, patch: Partial<InstanceInfo>): Promise<boolean> {
    return invoke<boolean>("update_instance", { uuid, patch });
  },

  async deleteInstance(uuid: string): Promise<boolean> {
    return invoke<boolean>("delete_instance", { uuid });
  },

  async moveInstance(uuid: string, group: string | null, index: number): Promise<boolean> {
    return invoke<boolean>("move_instance", { uuid, group, index });
  },

  async launchGame(uuid: string, userName: string): Promise<void> {
    return invoke<void>("launch_game", { uuid, userName });
  },

  async stopGame(uuid: string): Promise<void> {
    return invoke<void>("stop_game", { uuid });
  },

  async getGameLog(uuid: string): Promise<string[]> {
    return invoke<string[]>("get_game_log", { uuid });
  },

  async getRunning(): Promise<string[]> {
    return invoke<string[]>("get_running");
  },
};

// ---------------- 事件订阅（Rust emit → 前端 listen） ----------------

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
  return listen("instance-change", () => cb());
}
