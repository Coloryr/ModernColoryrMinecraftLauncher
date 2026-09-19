// MCML 前端类型定义（与 Rust 端序列化字段一一对应）

export interface InstanceInfo {
  uuid: string;
  name: string;
  group: string | null;
  version: string;
  /** 游戏版本类型 ID：release / snapshot / other / all */
  versionType?: string;
  /** 加载器 ID：normal / forge / fabric / quilt / neoforge / optifine / liteloader / custom */
  loader: string;
  loaderVersion: string | null;
  dir: string;
  running: boolean;
  /** 整合包类型 ID：curseforge / modrinth / mcmod / serverpack / none（null = 无） */
  modpackType?: string | null;
  /** 整合包项目 ID */
  pid?: string | null;
  /** 整合包文件 ID */
  fid?: string | null;
  /** 在线网络整合包地址（ServerPack） */
  serverUrl?: string | null;
  /** 游戏内语言 */
  lang?: string;
  /** 日志编码：utf8 / gbk */
  logEncoding?: string;
  /** 来源：导入的压缩包 / 文件夹 / 在线网址 */
  source?: string;
}

export interface JavaInfo {
  name: string;
  path: string;
  version: string;
  major: number;
  javaType: string;
  arch: string;
}

export interface VersionInfo {
  id: string;
  versionType: string;
}

export interface LogEvent {
  uuid: string;
  time: string;
  text: string;
  clear: boolean;
}

export interface StateEvent {
  uuid: string;
  state: string;
}

export interface ExitEvent {
  uuid: string;
  code: number;
}

export interface ErrorEvent {
  uuid: string | null;
  message: string;
}

// ---------- 主界面 UI 数据 ----------

/** 账户信息（字段与 Rust 端 AccountStoreDto 一一对应） */
export interface Account {
  uuid: string;
  /** 玩家用户名 */
  userName: string;
  /** 账户类型：offline（离线）/ microsoft（微软）/ littleskin / authlib / nide8 等 */
  authType: string;
  /** 最后登录时间 */
  loginTime: string;
  /** 头像渐变起点色（CSS，无皮肤时回退占位图） */
  avatarColor: string;
  /** 皮肤主色（SVG 生成用） */
  skin: string;
  /** Token 状态：valid / expired */
  tokenStatus: string;
  /** 皮肤渲染头像（mcml-skin-draw 生成的 data URI PNG）；无皮肤为 null */
  avatar: string | null;
}

/** Minecraft 新闻条目 */
export interface NewsItem {
  id: number;
  title: string;
  date: string;
  tag: string;
  /** 新闻配图（URL 或 data URI） */
  image: string;
  /** 原文链接（系统浏览器打开） */
  url: string;
}

/** 启动设置（模拟） */
export interface LaunchSettings {
  memory: number;
  javaName: string;
  gameArgs: string;
}

/** 附加环境变量（键值对） */
export interface EnvVarLine {
  key: string;
  value: string;
}

/** 实例启动参数（内存 / 窗口 / Java + 扩展参数 + 自定义执行 + 代理） */
export interface InstanceArgs {
  /** 最大内存（MB） */
  memory: number;
  /** 最小内存（MB） */
  minMemory: number;
  fullscreen: boolean;
  width: number;
  height: number;
  /** 使用的 Java 名；"custom" 表示自定义路径 */
  javaName: string;
  /** 自定义 Java 路径 */
  javaPath: string;
  /** GC 回收器：auto / g1gc / zgc / none / custom */
  gc: string;
  /** 自定义 GC 参数（gc 为 custom 时使用） */
  gcCustom: string;
  /** 自定义主类 */
  mainClass: string;
  /** 附加 JVM 参数（多行） */
  jvmArgs: string[];
  /** 附加游戏参数（多行） */
  gameArgs: string[];
  /** 附加 classpath（多行） */
  classPath: string[];
  /** 附加环境变量（键值对列表） */
  envVars: EnvVarLine[];
  /** 游戏内语言 */
  lang: string;
  /** 日志编码：utf8 / gbk */
  logEncoding: string;
  /** 启动前执行 */
  preEnabled: boolean;
  preCmd: string;
  /** 启动后执行 */
  postEnabled: boolean;
  postCmd: string;
  /** 游戏内代理 */
  proxyIp: string;
  proxyPort: number;
  proxyUser: string;
  proxyPass: string;
  /** 自动加入服务器 */
  serverIp: string;
  serverPort: number;
  joinServer: boolean;
}

// ---------- 下载管理（mcml_downloader） ----------

/** 下载任务快照（download_get_status 查询） */
export interface DownloadTaskInfo {
  id: number;
  /** 文件总数 */
  total: number;
  completed: number;
  failed: number;
  /** 总大小（字节，元信息未知为 0） */
  allBytes: number;
  /** 已下载字节 */
  nowBytes: number;
  /** 已进行时间（毫秒） */
  elapsedMs: number;
  /** 是否暂停 */
  paused: boolean;
}

/** 下载线程当前状态（含当前文件进度与速度） */
export interface DownloadThreadInfo {
  thread: number;
  name: string;
  /** 状态 ID：wait / getinfo / download / pause / init / action / done / error */
  state: string;
  /** 当前文件进度百分比 */
  progress: number;
  nowBytes: number;
  allBytes: number;
  /** 下载速度（字节/秒） */
  speed: number;
}

/** 下载状态快照（任务 + 线程 + 总体速度） */
export interface DownloadStatus {
  tasks: DownloadTaskInfo[];
  threads: DownloadThreadInfo[];
  /** 总体下载速度（字节/秒） */
  speed: number;
  /** 是否处于全局暂停 */
  paused: boolean;
}

/** 下载任务状态事件（type：add / remove / update） */
export interface DownloadTaskEvent {
  type: string;
  id: number;
  progress: number;
}

/** 下载线程当前文件事件 */
export interface DownloadItemEvent {
  thread: number;
  name: string;
  /** 状态 ID：wait / getinfo / download / pause / init / action / done / error */
  state: string;
}

/** 压缩包检测结果（packType 为压缩包类型 ID，name 为推荐实例名） */
export interface DetectedPackInfo {
  packType: string;
  name: string;
}

/** 在线整合包搜索结果条目（id 为项目 ID，来源内唯一） */
export interface ModpackItem {
  id: string;
  name: string;
  desc: string;
  icon: string;
  author: string;
  downloads: number;
}

/** 在线整合包搜索结果分页 */
export interface ModpackSearchResult {
  items: ModpackItem[];
  page: number;
  total: number;
}

/** 整合包可安装版本（id 为文件/版本 ID，安装时回传） */
export interface ModpackFile {
  id: string;
  name: string;
  fileName: string;
  date: string;
  size: number;
}

/** 整合包安装进度（state：downloadPack / readInfo / getInfo / downloadFile / extract / done） */
export interface PackProgress {
  state: string;
  now: number;
  total: number;
  subText: string | null;
  subNow: number;
  subTotal: number;
}

/** 收藏项（uuid 是后端 collect.json 里的字典 key，增删改都用它寻址） */
export interface CollectItem {
  uuid: string;
  name: string;
  /** 下载源：curseforge / modrinth */
  source: string;
  /** 资源类型：modpack / mod / resourcepack / shaderpack … */
  fileType: string;
  pid: string;
  /** 图标地址（URL） */
  icon: string | null;
  /** 项目网页地址 */
  url: string;
}

/** 收藏窗口数据：收藏项 + 分组（分组名 → 收藏项 uuid 列表） */
export interface CollectData {
  items: CollectItem[];
  groups: Record<string, string[]>;
}
