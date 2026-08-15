// MCML 前端类型定义（与 Rust 端序列化字段一一对应）

export interface InstanceInfo {
  uuid: string;
  name: string;
  group: string | null;
  version: string;
  /** 游戏版本类型：release / snapshot / other */
  versionType?: string;
  loader: string;
  loaderVersion: string | null;
  dir: string;
  running: boolean;
  /** 整合包平台：CurseForge / Modrinth / McMod / 无 */
  modpackType?: string;
  /** 整合包项目 ID */
  pid?: string;
  /** 整合包文件 ID */
  fid?: string;
  /** 游戏内语言 */
  lang?: string;
  /** 日志编码：utf8 / gbk */
  logEncoding?: string;
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

/** 账户信息 */
export interface Account {
  uuid: string;
  name: string;
  /** 账户类型：offline（离线）/ microsoft（微软）/ littleskin / authlib / nide8 等 */
  type: string;
  /** 头像渐变起点色（CSS） */
  avatarColor: string;
  /** 皮肤主色（SVG 生成用） */
  skin: string;
  /** 最后登录时间 */
  lastLogin: string;
  /** Token 状态：valid / expired */
  tokenStatus: string;
}

/** Minecraft 新闻条目 */
export interface NewsItem {
  id: number;
  title: string;
  date: string;
  tag: string;
  /** 新闻配图（URL 或 data URI） */
  image: string;
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
