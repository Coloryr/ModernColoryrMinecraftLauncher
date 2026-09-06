// MCML 前端 API（真实 IPC 实现）
//
// 数据全部从 Rust 侧获取（命令见 mcml-gui/src-tauri/src/windows/），
// 按钮执行的操作也通过 IPC 调用。仅在 Tauri 环境可用（纯浏览器会报错）。
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DetectedPackInfo,
  DownloadItemEvent,
  DownloadTaskEvent,
  DownloadTaskInfo,
  ErrorEvent,
  ExitEvent,
  InstanceArgs,
  InstanceInfo,
  JavaInfo,
  LogEvent,
  ModpackFile,
  ModpackSearchResult,
  NewsItem,
  PackProgress,
  StateEvent,
  VersionInfo,
} from "./types";
import { MainGetInstances, MainGetGroups, MainGetInstanceArgs, MainGetInstanceLangs, MainGetJavaList, MainGetVersions, MainGetNews, MainOpenUrl, MainAddGroup, MainRemoveGroup, MainMoveGroup, MainCreateInstance, MainRenameInstance, MainUpdateInstance, MainUpdateInstanceArgs, MainDeleteInstance, MainMoveInstance, MainLaunchGame, MainStopGame, MainGetGameLog, MainGetRunning, MainRefreshVersions, MainAddJava, MainRemoveJava, MainScanJava } from "./invokes";
import { AddCreateNew, AddImportFolder, AddImportArchive, AddImportUrl, AddCancel, AddDetectArchive, AddGetModpackFiles, AddSearchModpacks, AddInstallModpack, AddGetLoaderVersions, AddGetLoaders, AddGetPackTypes, AddGetVersionTypes, AddGetSupportLoaders, AddSetCloseGuard, AddAnswerNameConflict } from "./invokes";
import { DownloadCancelTask, DownloadGetTasks } from "./invokes";
import { GameLog, LaunchState, GameExit, LaunchError, InstanceChange, CloseBlocked, AddLoaderProgress, AddNameConflict, AddPackProgress, JavaChange, DownloadItem, DownloadTask } from "./listens";

export interface CreateInstanceOpts {
  loader?: string;
  loaderVersion?: string | null;
  group?: string | null;
  modpackType?: string;
  source?: string;
}

export const api = {
  async getInstances(): Promise<InstanceInfo[]> {
    return invoke<InstanceInfo[]>(MainGetInstances);
  },

  async getGroups(): Promise<string[]> {
    return invoke<string[]>(MainGetGroups);
  },

  /** 获取实例的游戏内语言列表（从资源索引查 minecraft/lang/*.json，资源未下载时为空） */
  async getInstanceLangs(uuid: string): Promise<string[]> {
    return invoke<string[]>(MainGetInstanceLangs, { uuid });
  },

  async getJavaList(): Promise<JavaInfo[]> {
    return invoke<JavaInfo[]>(MainGetJavaList);
  },

  /** 添加 Java（后端校验有效后入列表；无效返回 false） */
  async addJava(name: string, path: string): Promise<boolean> {
    return invoke<boolean>(MainAddJava, { name, path });
  },

  /** 删除指定名称的 Java */
  async removeJava(name: string): Promise<void> {
    return invoke<void>(MainRemoveJava, { name });
  },

  /** 扫描系统已安装的 Java（耗时查询）并返回最新列表 */
  async scanJava(): Promise<JavaInfo[]> {
    return invoke<JavaInfo[]>(MainScanJava);
  },

  async getVersions(): Promise<VersionInfo[]> {
    return invoke<VersionInfo[]>(MainGetVersions);
  },

  /** 强制刷新版本列表（清空后端缓存重新拉取） */
  async refreshVersions(): Promise<VersionInfo[]> {
    return invoke<VersionInfo[]>(MainRefreshVersions);
  },

  /** 获取 Minecraft 官方新闻（第 page 页，默认 1） */
  async getNews(page = 1): Promise<NewsItem[]> {
    return invoke<NewsItem[]>(MainGetNews, { page });
  },

  /** 用系统浏览器打开网址 */
  async openUrl(url: string): Promise<void> {
    return invoke<void>(MainOpenUrl, { url });
  },

  /** 从头新建实例（版本 + 加载器），返回新实例 uuid */
  async addCreateNew(
    name: string,
    version: string,
    loader: string,
    loaderVersion: string | null,
    group: string | null,
  ): Promise<string> {
    return invoke<string>(AddCreateNew, { name, version, loader, loaderVersion, group });
  },

  /** 导入文件夹为实例 */
  async addImportFolder(path: string, name: string, group: string | null): Promise<string> {
    return invoke<string>(AddImportFolder, { path, name, group });
  },

  /** 导入整合包压缩包（packType：CurseForge / Modrinth / McMod / 本地） */
  async addImportArchive(
    path: string,
    packType: string,
    name: string,
    group: string | null,
    unselect: string[] | null,
  ): Promise<string> {
    return invoke<string>(AddImportArchive, { path, packType, name, group, unselect });
  },

  /** 从网址安装实例 */
  async addImportUrl(url: string, name: string, group: string | null): Promise<string> {
    return invoke<string>(AddImportUrl, { url, name, group });
  },

  /** 检测压缩包的整合包类型与推荐实例名（识别失败抛错） */
  async addDetectArchive(path: string): Promise<DetectedPackInfo> {
    return invoke<DetectedPackInfo>(AddDetectArchive, { path });
  },

  /** 搜索在线整合包（source：curseforge / modrinth，page 从 0 开始） */
  async searchModpacks(
    source: string,
    query: string | null,
    version: string | null,
    sort: string,
    page: number,
  ): Promise<ModpackSearchResult> {
    return invoke<ModpackSearchResult>(AddSearchModpacks, { source, query, version, sort, page });
  },

  /** 获取整合包的可安装版本列表（version 传 null 取全部） */
  async getModpackFiles(source: string, projectId: string, version: string | null): Promise<ModpackFile[]> {
    return invoke<ModpackFile[]>(AddGetModpackFiles, { source, projectId, version });
  },

  /** 安装在线整合包（实例名取自整合包元数据，返回新实例 uuid） */
  async installModpack(
    source: string,
    projectId: string,
    fileId: string,
    group: string | null,
  ): Promise<string> {
    return invoke<string>(AddInstallModpack, { source, projectId, fileId, group });
  },

  /** 取消进行中的安装任务 */
  async addCancel(): Promise<boolean> {
    return invoke<boolean>(AddCancel);
  },

  /** 获取加载器的可用版本列表（loader：加载器 ID，mc：游戏版本号） */
  async addLoaderVersions(loader: string, mc: string): Promise<string[]> {
    return invoke<string[]>(AddGetLoaderVersions, { loader, mc });
  },

  /** 获取加载器 ID 列表 */
  async addGetLoaders(): Promise<string[]> {
    return invoke<string[]>(AddGetLoaders);
  },

  /** 查询指定游戏版本支持的加载器 ID 列表 */
  async addGetSupportLoaders(mc: string): Promise<string[]> {
    return invoke<string[]>(AddGetSupportLoaders, { mc });
  },

  /** 设置添加实例窗口关闭保护（enabled = true 期间拒绝关闭请求） */
  async setCloseGuard(enabled: boolean): Promise<void> {
    await invoke(AddSetCloseGuard, { enabled });
  },

  /** 获取压缩包类型 ID 列表 */
  async addGetPackTypes(): Promise<string[]> {
    return invoke<string[]>(AddGetPackTypes);
  },

  /** 获取游戏版本类型 ID 列表（release / snapshot / old_beta / old_alpha） */
  async addGetVersionTypes(): Promise<string[]> {
    return invoke<string[]>(AddGetVersionTypes);
  },

  async addGroup(name: string): Promise<boolean> {
    return invoke<boolean>(MainAddGroup, { name });
  },

  async removeGroup(name: string): Promise<boolean> {
    return invoke<boolean>(MainRemoveGroup, { name });
  },

  async moveGroup(name: string, index: number): Promise<boolean> {
    return invoke<boolean>(MainMoveGroup, { name, index });
  },

  async createInstance(
    name: string,
    version: string,
    opts?: CreateInstanceOpts,
  ): Promise<InstanceInfo> {
    return invoke<InstanceInfo>(MainCreateInstance, {
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
    return invoke<boolean>(MainRenameInstance, { uuid, name });
  },

  /** 更新实例元信息（补丁式，Partial<InstanceInfo>） */
  async updateInstance(uuid: string, patch: Partial<InstanceInfo>): Promise<boolean> {
    return invoke<boolean>(MainUpdateInstance, { uuid, patch });
  },

  /** 获取实例启动参数（核心实例读配置，遗留数据读内存缓存） */
  async getInstanceArgs(uuid: string): Promise<InstanceArgs> {
    return invoke<InstanceArgs>(MainGetInstanceArgs, { uuid });
  },

  /** 更新实例启动参数（核心实例写配置并保存） */
  async updateInstanceArgs(uuid: string, args: InstanceArgs): Promise<boolean> {
    return invoke<boolean>(MainUpdateInstanceArgs, { uuid, args });
  },

  async deleteInstance(uuid: string): Promise<boolean> {
    return invoke<boolean>(MainDeleteInstance, { uuid });
  },

  async moveInstance(uuid: string, group: string | null, index: number): Promise<boolean> {
    return invoke<boolean>(MainMoveInstance, { uuid, group, index });
  },

  async launchGame(uuid: string, userName: string): Promise<void> {
    return invoke<void>(MainLaunchGame, { uuid, userName });
  },

  async stopGame(uuid: string): Promise<void> {
    return invoke<void>(MainStopGame, { uuid });
  },

  async getGameLog(uuid: string): Promise<string[]> {
    return invoke<string[]>(MainGetGameLog, { uuid });
  },

  async getRunning(): Promise<string[]> {
    return invoke<string[]>(MainGetRunning);
  },

  /** 获取进行中的下载任务快照（下载管理窗口） */
  async getDownloadTasks(): Promise<DownloadTaskInfo[]> {
    return invoke<DownloadTaskInfo[]>(DownloadGetTasks);
  },

  /** 取消一个下载任务（任务不存在返回 false） */
  async cancelDownloadTask(id: number): Promise<boolean> {
    return invoke<boolean>(DownloadCancelTask, { id });
  },
};

// ---------------- 事件订阅（Rust emit → 前端 listen） ----------------

export function onGameLog(cb: (e: LogEvent) => void): Promise<UnlistenFn> {
  return listen<LogEvent>(GameLog, (e) => cb(e.payload));
}
export function onLaunchState(cb: (e: StateEvent) => void): Promise<UnlistenFn> {
  return listen<StateEvent>(LaunchState, (e) => cb(e.payload));
}
export function onGameExit(cb: (e: ExitEvent) => void): Promise<UnlistenFn> {
  return listen<ExitEvent>(GameExit, (e) => cb(e.payload));
}
export function onLaunchError(cb: (e: ErrorEvent) => void): Promise<UnlistenFn> {
  return listen<ErrorEvent>(LaunchError, (e) => cb(e.payload));
}
export function onInstanceChange(cb: () => void): Promise<UnlistenFn> {
  return listen(InstanceChange, () => cb());
}
export function onJavaChange(cb: () => void): Promise<UnlistenFn> {
  return listen(JavaChange, () => cb());
}

/** 下载任务状态事件（type：add / remove / update） */
export function onDownloadTask(cb: (e: DownloadTaskEvent) => void): Promise<UnlistenFn> {
  return listen<DownloadTaskEvent>(DownloadTask, (e) => cb(e.payload));
}

/** 下载线程当前文件状态事件 */
export function onDownloadItem(cb: (e: DownloadItemEvent) => void): Promise<UnlistenFn> {
  return listen<DownloadItemEvent>(DownloadItem, (e) => cb(e.payload));
}

/** 关闭被拒绝（窗口处于关闭保护时，前端弹提示说明原因） */
export function onCloseBlocked(cb: () => void): Promise<UnlistenFn> {
  return listen(CloseBlocked, () => cb());
}

/** 加载器支持列表查询进度（step / total） */
export function onAddLoaderProgress(cb: (e: { step: number; total: number }) => void): Promise<UnlistenFn> {
  return listen<{ step: number; total: number }>(AddLoaderProgress, (e) => cb(e.payload));
}

/** 实例重名确认（kind：overwrite 覆盖 / rename 自动改名） */
export function onAddNameConflict(
  cb: (e: { id: number; kind: string; name: string }) => void,
): Promise<UnlistenFn> {
  return listen<{ id: number; kind: string; name: string }>(AddNameConflict, (e) => cb(e.payload));
}

/** 整合包安装进度（本地压缩包 / 在线整合包安装共用） */
export function onAddPackProgress(cb: (e: PackProgress) => void): Promise<UnlistenFn> {
  return listen<PackProgress>(AddPackProgress, (e) => cb(e.payload));
}

export function answerNameConflict(id: number, answer: boolean): Promise<void> {
  return invoke(AddAnswerNameConflict, { id, answer });
}
