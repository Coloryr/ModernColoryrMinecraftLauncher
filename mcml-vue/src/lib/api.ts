// MCML 前端 API（真实 IPC 实现）
//
// 数据全部从 Rust 侧获取（命令见 mcml-gui/src-tauri/src/windows/），
// 按钮执行的操作也通过 IPC 调用。仅在 Tauri 环境可用（纯浏览器会报错）。
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { commands } from "./bindings";
import type {
  CollectDataDto,
  DetectedPackDto,
  DownloadItemEvent,
  DownloadStatusDto,
  DownloadTaskEvent,
  ErrorEvent,
  ExitEvent,
  FileListDto,
  InstanceArgs,
  InstanceInfo,
  JavaInfo,
  LogEvent,
  NewsItem,
  PackProgressDto,
  ProjectDetailDto,
  ProjectDto,
  StateEvent,
  VersionInfo,
} from "./bindings";
import { AddLoaderProgress, AddNameConflict, AddPackProgress, CloseBlocked, CollectChange, DownloadItem, DownloadTask, GameExit, GameLog, InstanceChange, JavaChange, LaunchError, LaunchState } from "./listens";

export interface CreateInstanceOpts {
  loader?: string;
  loaderVersion?: string | null;
  group?: string | null;
  modpackType?: string;
  source?: string;
}

export const api = {
  async getInstances(): Promise<InstanceInfo[]> {
    return commands.main.getInstances();
  },

  async getGroups(): Promise<string[]> {
    return commands.main.getGroups();
  },

  /** 获取实例的游戏内语言列表（从资源索引查 minecraft/lang/*.json，资源未下载时为空） */
  async getInstanceLangs(uuid: string): Promise<string[]> {
    return commands.main.getInstanceLangs(uuid);
  },

  async getJavaList(): Promise<JavaInfo[]> {
    return commands.main.getJavaList();
  },

  /** 添加 Java（后端校验有效后入列表；无效返回 false） */
  async addJava(name: string, path: string): Promise<boolean> {
    return commands.main.addJava(name, path);
  },

  /** 删除指定名称的 Java */
  async removeJava(name: string): Promise<void> {
    return commands.main.removeJava(name);
  },

  /** 扫描系统已安装的 Java（耗时查询）并返回最新列表 */
  async scanJava(): Promise<JavaInfo[]> {
    return commands.main.scanJava();
  },

  async getVersions(): Promise<VersionInfo[]> {
    return commands.main.getVersions();
  },

  /** 强制刷新版本列表（清空后端缓存重新拉取） */
  async refreshVersions(): Promise<VersionInfo[]> {
    return commands.main.refreshVersions();
  },

  /** 获取 Minecraft 官方新闻（第 page 页，默认 1） */
  async getNews(page = 1): Promise<NewsItem[]> {
    return commands.main.getNews(page);
  },

  /** 用系统浏览器打开网址 */
  async openUrl(url: string): Promise<void> {
    return commands.main.openUrl(url);
  },

  /** 从头新建实例（版本 + 加载器），返回新实例 uuid */
  async addCreateNew(
    name: string,
    version: string,
    loader: string,
    loaderVersion: string | null,
    group: string | null,
  ): Promise<string> {
    return commands.add.createNew(name, version, loader, loaderVersion, group);
  },

  /** 导入文件夹为实例 */
  async addImportFolder(path: string, name: string, group: string | null): Promise<string> {
    return commands.add.importFolder(path, name, group);
  },

  /** 导入整合包压缩包（packType：CurseForge / Modrinth / McMod / 本地） */
  async addImportArchive(
    path: string,
    packType: string,
    name: string,
    group: string | null,
    unselect: string[] | null,
  ): Promise<string> {
    return commands.add.importArchive(path, packType, name, group, unselect);
  },

  /** 从网址安装实例 */
  async addImportUrl(url: string, name: string, group: string | null): Promise<string> {
    return commands.add.importUrl(url, name, group);
  },

  /** 检测压缩包的整合包类型与推荐实例名（识别失败抛错） */
  async addDetectArchive(path: string): Promise<DetectedPackDto> {
    return commands.add.detectArchive(path);
  },

  // ---------------- 在线整合包 ----------------

  /** 获取整合包下载源 ID 列表（curseforge / modrinth） */
  async getModpackSources(): Promise<string[]> {
    return commands.addResource.sourceType();
  },

  /** 获取某下载源的排序方式列表（取值即后端枚举线串，可原样回传） */
  async getModpackSorts(source: string): Promise<string[]> {
    return commands.addResource.sortType(source);
  },

  /** 获取某下载源整合包的分类（键 = 传给后端的分类值，值 = 显示名） */
  async getModpackCategories(source: string): Promise<Record<string, string>> {
    return commands.addResource.categories(source, "modpack");
  },

  /** 获取某下载源支持的游戏版本列表 */
  async getModpackVersions(source: string): Promise<string[]> {
    return commands.addResource.gameVersions(source);
  },

  /** 搜索在线整合包（page 从 0 开始；category / filter / version 传 null 表示不过滤） */
  async searchModpacks(
    source: string,
    page: number,
    sort: string,
    category: string | null,
    filter: string | null,
    version: string | null,
  ): Promise<ProjectDto> {
    return commands.addModpack.list(source, page, sort, category, filter, version);
  },

  /** 获取某整合包的可安装版本列表（version 传 null 取全部） */
  async getModpackFiles(
    source: string,
    projectId: string,
    page: number,
    version: string | null,
  ): Promise<FileListDto> {
    return commands.addModpack.file(source, projectId, page, version);
  },

  /** 获取某整合包的项目详情（Modrinth 含 markdown 正文；CurseForge 只有简介） */
  async getModpackDetail(source: string, projectId: string): Promise<ProjectDetailDto> {
    return commands.addModpack.detail(source, projectId);
  },

  /** 安装在线整合包（实例名取自整合包元数据，返回新实例 uuid） */
  async installModpack(
    source: string,
    projectId: string,
    fileId: string,
    group: string | null,
  ): Promise<string> {
    return commands.addModpack.install(source, projectId, fileId, group);
  },

  /** 取消进行中的安装任务 */
  async addCancel(): Promise<boolean> {
    return commands.add.cancel();
  },

  /** 获取加载器的可用版本列表（loader：加载器 ID，mc：游戏版本号） */
  async addLoaderVersions(loader: string, mc: string): Promise<string[]> {
    return commands.add.getLoaderVersions(loader, mc);
  },

  /** 获取加载器 ID 列表 */
  async addGetLoaders(): Promise<string[]> {
    return commands.add.getLoaders();
  },

  /** 查询指定游戏版本支持的加载器 ID 列表 */
  async addGetSupportLoaders(mc: string): Promise<string[]> {
    return commands.add.getSupportLoaders(mc);
  },

  /** 设置添加实例窗口关闭保护（enabled = true 期间拒绝关闭请求） */
  async setCloseGuard(enabled: boolean): Promise<void> {
    await commands.add.setCloseGuard(enabled);
  },

  /** 获取压缩包类型 ID 列表 */
  async addGetPackTypes(): Promise<string[]> {
    return commands.add.getPackTypes();
  },

  /** 获取游戏版本类型 ID 列表（release / snapshot / old_beta / old_alpha） */
  async addGetVersionTypes(): Promise<string[]> {
    return commands.add.getVersionTypes();
  },

  async addGroup(name: string): Promise<boolean> {
    return commands.main.addGroup(name);
  },

  async removeGroup(name: string): Promise<boolean> {
    return commands.main.removeGroup(name);
  },

  async moveGroup(name: string, index: number): Promise<boolean> {
    return commands.main.moveGroup(name, index);
  },

  async createInstance(
    name: string,
    version: string,
    opts?: CreateInstanceOpts,
  ): Promise<InstanceInfo> {
    return commands.main.createInstance(name, version, opts?.loader ?? null, opts?.loaderVersion ?? null, opts?.group ?? null, opts?.modpackType ?? null, opts?.source ?? null);
  },

  async renameInstance(uuid: string, name: string): Promise<boolean> {
    return commands.main.renameInstance(uuid, name);
  },

  /** 更新实例元信息（补丁式，Partial<InstanceInfo>） */
  async updateInstance(uuid: string, patch: Partial<InstanceInfo>): Promise<boolean> {
    return commands.main.updateInstance(uuid, patch);
  },

  /** 获取实例启动参数（核心实例读配置，遗留数据读内存缓存） */
  async getInstanceArgs(uuid: string): Promise<InstanceArgs> {
    return commands.main.getInstanceArgs(uuid);
  },

  /** 更新实例启动参数（核心实例写配置并保存） */
  async updateInstanceArgs(uuid: string, args: InstanceArgs): Promise<boolean> {
    return commands.main.updateInstanceArgs(uuid, args);
  },

  async deleteInstance(uuid: string): Promise<boolean> {
    return commands.main.deleteInstance(uuid);
  },

  async moveInstance(uuid: string, group: string | null, index: number): Promise<boolean> {
    return commands.main.moveInstance(uuid, group, index);
  },

  async launchGame(uuid: string, userName: string): Promise<void> {
    return commands.main.launchGame(uuid, userName);
  },

  async stopGame(uuid: string): Promise<void> {
    return commands.main.stopGame(uuid);
  },

  async getGameLog(uuid: string): Promise<string[]> {
    return commands.main.getGameLog(uuid);
  },

  async getRunning(): Promise<string[]> {
    return commands.main.getRunning();
  },

  /** 获取下载状态快照（任务 + 线程 + 总体速度，下载管理窗口轮询） */
  async getDownloadStatus(): Promise<DownloadStatusDto> {
    return commands.download.getStatus();
  },

  /** 全局暂停所有下载（期间新增任务同样暂停），返回被暂停的任务数 */
  async pauseAllDownloads(): Promise<number> {
    return commands.download.pauseAll();
  },

  /** 全局恢复所有下载，返回被恢复的任务数 */
  async resumeAllDownloads(): Promise<number> {
    return commands.download.resumeAll();
  },

  /** 全局停止（取消）所有下载，返回被停止的任务数 */
  async stopAllDownloads(): Promise<number> {
    return commands.download.cancelAll();
  },

  // ---------------- 收藏 ----------------

  /** 获取收藏数据（收藏项 + 分组） */
  async collectGetData(): Promise<CollectDataDto> {
    return commands.collect.getData();
  },

  /** 添加分组（重名会抛错） */
  async collectAddGroup(name: string): Promise<void> {
    return commands.collect.addGroup(name);
  },

  /** 删除分组（组内的收藏条目保留） */
  async collectRemoveGroup(name: string): Promise<void> {
    return commands.collect.removeGroup(name);
  },

  /** 清空收藏：group 为空 = 清空全部（分组保留），否则只清空该分组 */
  async collectClear(group: string | null): Promise<void> {
    return commands.collect.clear(group);
  },

  /** 移除收藏：group 为空 = 从收藏删除，否则只从该分组移除 */
  async collectRemoveItems(uuids: string[], group: string | null): Promise<void> {
    return commands.collect.removeItems(uuids, group);
  },

  /** 把收藏加入分组 */
  async collectSetGroupItems(group: string, uuids: string[]): Promise<void> {
    return commands.collect.setGroupItems(group, uuids);
  },

  /** 收藏 / 取消收藏在线项目（列表与详情里的星标） */
  async collectStar(
    source: string,
    fileType: string,
    pid: string,
    name: string,
    icon: string | null,
    url: string,
    star: boolean,
  ): Promise<void> {
    return commands.collect.star(source, fileType, pid, name, icon, url, star);
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
export function onAddPackProgress(cb: (e: PackProgressDto) => void): Promise<UnlistenFn> {
  return listen<PackProgressDto>(AddPackProgress, (e) => cb(e.payload));
}

export function answerNameConflict(id: number, answer: boolean): Promise<void> {
  return commands.add.answerNameConflict(id, answer);
}

/** 收藏变更（跨窗口同步） */
export function onCollectChange(cb: () => void): Promise<UnlistenFn> {
  return listen(CollectChange, () => cb());
}
