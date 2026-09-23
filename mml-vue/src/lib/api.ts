// M²L 前端 API（真实 IPC 实现）
//
// 数据全部从 Rust 侧获取（命令见 mml-gui/src-tauri/src/windows/），
// 按钮执行的操作也通过 IPC 调用。仅在 Tauri 环境可用（纯浏览器会报错）。
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { commands } from "./bindings";
import type {
  BlockItemDto,
  BlockStatusDto,
  CollectDataDto,
  DataPackItemDto,
  DetectedPackDto,
  LogLine,
  DownloadItemEvent,
  DownloadStatusDto,
  DownloadTaskEvent,
  ErrorEvent,
  ExitEvent,
  FileListDto,
  InstanceArgs,
  InstanceInfo,
  JavaInfo,
  LoadState,
  LogEvent,
  ModItemDto,
  ModPackStatusDto,
  NewsItem,
  PackItemDto,
  PackProgressDto,
  ProjectDetailDto,
  ProjectDto,
  ResourceSaveDto,
  ResourceStatusDto,
  SaveItemDto,
  ScreenshotItemDto,
  ServerItemDto,
  ShaderItemDto,
  SchematicItemDto,
  StateEvent,
  VersionInfo,
} from "./bindings";
import { AddLoaderProgress, AddModpackStatus, AddNameConflict, AddPackProgress, AddResourceStatus, CloseBlocked, CollectChange, DownloadItem, DownloadTask, GameExit, GameLog, InstanceChange, JavaChange, LaunchError, LaunchState, MainBlockRender } from "./listens";

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

  /** 安装在线整合包（多任务：命令立即返回，任务后台安装，进度走 add-modpack-status 事件） */
  async installModpack(
    source: string,
    projectId: string,
    fileId: string,
    group: string | null,
  ): Promise<void> {
    return commands.addModpack.install(source, projectId, fileId, group);
  },

  /** 查询整合包安装任务总览（挂载时同步一次，之后靠事件） */
  async getModpackStatus(): Promise<ModPackStatusDto> {
    return commands.addModpack.status();
  },

  // ---------------- 在线资源（实例设置 → 添加资源） ----------------

  /** 获取某下载源某资源类型的分类（键 = 传给后端的分类值，值 = 显示名） */
  async getResourceCategories(source: string, fileType: string): Promise<Record<string, string>> {
    return commands.addResource.categories(source, fileType);
  },

  /** 获取某下载源的排序方式列表（取值即后端枚举线串，可原样回传） */
  async getResourceSorts(source: string): Promise<string[]> {
    return commands.addResource.sortType(source);
  },

  /** 获取某下载源支持的游戏版本列表 */
  async getResourceVersions(source: string): Promise<string[]> {
    return commands.addResource.gameVersions(source);
  },

  /** 搜索在线资源（page 从 0 开始；category / filter / version / loader 传 null 表示不过滤） */
  async searchResources(
    game: string,
    source: string,
    fileType: string,
    page: number,
    sort: string,
    category: string | null,
    filter: string | null,
    version: string | null,
    loader: string | null,
  ): Promise<ProjectDto> {
    return commands.addResource.list(
      game,
      source,
      fileType,
      page,
      sort,
      category,
      filter,
      version,
      loader,
    );
  },

  /** 获取某资源的可下载版本列表（page 从 0 开始；version / loader 传 null 表示不过滤） */
  async getResourceFiles(
    game: string,
    source: string,
    projectId: string,
    fileType: string,
    page: number,
    version: string | null,
    loader: string | null,
  ): Promise<FileListDto> {
    return commands.addResource.file(game, source, projectId, fileType, page, version, loader);
  },

  /** 获取实例的存档列表（下载数据包时选择目标存档） */
  async getResourceSaves(game: string): Promise<ResourceSaveDto[]> {
    return commands.addResource.saves(game);
  },

  /** 下载资源到实例（fileType = mod / resourcepack / shaderpack / save / dataPacks；
   *  数据包必须传 world = 存档文件夹名；命令立即返回，下载走全局下载器） */
  async downloadResource(
    game: string,
    source: string,
    projectId: string,
    fileId: string,
    fileType: string,
    world: string | null,
  ): Promise<void> {
    return commands.addResource.download(game, source, projectId, fileId, fileType, world);
  },

  /** 查询资源下载任务总览（挂载时同步一次，之后靠事件） */
  async getResourceStatus(): Promise<ResourceStatusDto> {
    return commands.addResource.status();
  },

  /** 取消一个进行中的整合包安装任务（pid + fid 定位） */
  async cancelModpackInstall(pid: string, fid: string): Promise<void> {
    return commands.addModpack.cancel(pid, fid);
  },

  /** 清除已结束（完成 / 失败 / 取消）的整合包安装任务 */
  async clearModpackDone(): Promise<void> {
    return commands.addModpack.clearDone();
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

  async getGameLog(uuid: string): Promise<LogLine[]> {
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
/** 实例数据变更（type：add / edit / remove / group） */
export function onInstanceChange(cb: (type: string) => void): Promise<UnlistenFn> {
  return listen<{ type: string }>(InstanceChange, (e) => cb(e.payload.type));
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

/** 整合包安装任务总览（多任务进度条，下载整合包窗口 / 主窗口共用） */
export function onAddModpackStatus(cb: (e: ModPackStatusDto) => void): Promise<UnlistenFn> {
  return listen<ModPackStatusDto>(AddModpackStatus, (e) => cb(e.payload));
}

/** 资源下载任务总览事件（任务增删 / 进度变化 / 移除时广播） */
export function onAddResourceStatus(cb: (e: ResourceStatusDto) => void): Promise<UnlistenFn> {
  return listen<ResourceStatusDto>(AddResourceStatus, (e) => cb(e.payload));
}

export function answerNameConflict(id: number, answer: boolean): Promise<void> {
  return commands.add.answerNameConflict(id, answer);
}

/** 收藏变更（跨窗口同步） */
export function onCollectChange(cb: () => void): Promise<UnlistenFn> {
  return listen(CollectChange, () => cb());
}

// ==================== 方块列表 ====================

/** 获取方块列表（按语言翻译显示名） */
export function getBlockList(lang: string): Promise<BlockItemDto[]> {
  return commands.main.blockList(lang);
}

/** 获取方块贴图渲染状态 */
export function getBlockStatus(): Promise<BlockStatusDto> {
  return commands.main.blockStatus();
}

/** 开始渲染方块贴图（force = 全量重渲染；已在进行返回 false） */
export function blockRenderStart(force: boolean): Promise<boolean> {
  return commands.main.blockRenderStart(force);
}

/** 把方块贴图设为实例图标 */
export function blockSetIcon(uuid: string, id: string): Promise<boolean> {
  return commands.main.blockSetIcon(uuid, id);
}

/** 按用户名或UUID添加皮肤方块（同名覆盖），返回方块ID */
export function blockSkinAdd(input: string): Promise<string> {
  return commands.main.blockSkinAdd(input);
}

/** 删除皮肤方块（名字即皮肤方块显示名） */
export function blockSkinRemove(name: string): Promise<void> {
  return commands.main.blockSkinRemove(name);
}

/** 方块渲染状态事件（进度 / 结束） */
export function onBlockRender(cb: (e: BlockStatusDto) => void): Promise<UnlistenFn> {
  return listen<BlockStatusDto>(MainBlockRender, (e) => cb(e.payload));
}

/** mml-image 协议访问前缀（拼实例图标等本地图片地址用） */
export function getImageBaseUrl(): Promise<string> {
  return commands.main.imageBaseUrl();
}

/** 核心加载状态（启动兜底：load-done 事件可能在页面监听前就发出） */
export function getLoadState(): Promise<LoadState> {
  return commands.main.loadState();
}

// ==================== 实例资源管理 ====================

/** 模组列表（解析 jar 元数据，条目多时耗时数秒） */
export function listMods(uuid: string): Promise<ModItemDto[]> {
  return commands.resource.listMods(uuid);
}
/** 启用模组（去掉 .disable / .disabled 后缀） */
export function enableMod(uuid: string, modUuid: string): Promise<void> {
  return commands.resource.modEnable(uuid, modUuid);
}
/** 禁用模组（追加 .disable 后缀） */
export function disableMod(uuid: string, modUuid: string): Promise<void> {
  return commands.resource.modDisable(uuid, modUuid);
}
/** 删除模组（进回收站） */
export function deleteMod(uuid: string, modUuid: string): Promise<void> {
  return commands.resource.deleteMod(uuid, modUuid);
}

/** 材质包列表 */
export function listResourcepacks(uuid: string): Promise<PackItemDto[]> {
  return commands.resource.listResourcepacks(uuid);
}
/** 删除材质包（进回收站） */
export function deleteResourcepack(uuid: string, file: string): Promise<void> {
  return commands.resource.deleteResourcepack(uuid, file);
}

/** 存档列表 */
export function listSaves(uuid: string): Promise<SaveItemDto[]> {
  return commands.resource.listSaves(uuid);
}
/** 删除存档（进回收站） */
export function deleteSave(uuid: string, dir: string): Promise<void> {
  return commands.resource.deleteSave(uuid, dir);
}
/** 备份存档（zip 到实例备份目录），返回备份文件名 */
export function backupSave(uuid: string, dir: string): Promise<string> {
  return commands.resource.backupSave(uuid, dir);
}

/** 截图列表 */
export function listScreenshots(uuid: string): Promise<ScreenshotItemDto[]> {
  return commands.resource.listScreenshots(uuid);
}
/** 删除截图（进回收站） */
export function deleteScreenshot(uuid: string, name: string): Promise<void> {
  return commands.resource.deleteScreenshot(uuid, name);
}
/** 清空全部截图（进回收站） */
export function clearScreenshots(uuid: string): Promise<void> {
  return commands.resource.clearScreenshots(uuid);
}

/** 打开资源目录（name 为空打开目录本身，否则资源管理器选中该文件；
 * datapacks 类别需传 parent = 存档目录名） */
export function openResourceFolder(
  uuid: string,
  kind: string,
  name: string | null,
  parent: string | null = null,
): Promise<void> {
  return commands.resource.openFolder(uuid, kind, name, parent);
}

// ==================== 服务器 ====================

/** 服务器列表（servers.dat） */
export function listServers(uuid: string): Promise<ServerItemDto[]> {
  return commands.resource.listServers(uuid);
}
/** 添加服务器 */
export function addServer(uuid: string, name: string, ip: string): Promise<void> {
  return commands.resource.serverAdd(uuid, name, ip);
}
/** 编辑服务器（按原 name + ip 定位） */
export function updateServer(
  uuid: string,
  name: string,
  ip: string,
  newName: string,
  newIp: string,
  acceptTextures: boolean,
): Promise<void> {
  return commands.resource.serverUpdate(uuid, name, ip, newName, newIp, acceptTextures);
}
/** 删除服务器 */
export function deleteServer(uuid: string, name: string, ip: string): Promise<void> {
  return commands.resource.serverDelete(uuid, name, ip);
}

// ==================== 光影包 ====================

/** 光影包列表（selected 标出当前启用的包） */
export function listShaderpacks(uuid: string): Promise<ShaderItemDto[]> {
  return commands.resource.listShaderpacks(uuid);
}
/** 启用 / 停用光影包（null = 停用，options.txt 写 OFF） */
export function setShader(uuid: string, file: string | null): Promise<void> {
  return commands.resource.shaderSet(uuid, file);
}
/** 删除光影包（进回收站） */
export function deleteShaderpack(uuid: string, file: string): Promise<void> {
  return commands.resource.deleteShaderpack(uuid, file);
}

// ==================== 结构文件 ====================

/** 结构文件列表 */
export function listSchematics(uuid: string): Promise<SchematicItemDto[]> {
  return commands.resource.listSchematics(uuid);
}
/** 删除结构文件（进回收站） */
export function deleteSchematic(uuid: string, file: string): Promise<void> {
  return commands.resource.deleteSchematic(uuid, file);
}

// ==================== 数据包（存档子页） ====================

/** 存档的数据包列表 */
export function listDatapacks(uuid: string, dir: string): Promise<DataPackItemDto[]> {
  return commands.resource.listDatapacks(uuid, dir);
}
/** 切换数据包启用状态 */
export function toggleDatapack(uuid: string, dir: string, name: string): Promise<void> {
  return commands.resource.datapackToggle(uuid, dir, name);
}
/** 删除数据包（清 level.dat 引用 + 文件进回收站） */
export function deleteDatapack(uuid: string, dir: string, name: string): Promise<void> {
  return commands.resource.datapackDelete(uuid, dir, name);
}
