// MCML 前端 API（界面开发阶段的模拟实现）
//
// 当前不依赖任何后端命令，可在纯浏览器（npm run dev）中运行，
// 方便先做界面。gui 与 core 已解耦，旧的真实实现 `./api-real` 已注释掉。
import type {
  ErrorEvent,
  ExitEvent,
  InstanceInfo,
  JavaInfo,
  LogEvent,
  StateEvent,
  VersionInfo,
} from "./types";

// ---------- 模拟数据 ----------

const MOCK_INSTANCES: InstanceInfo[] = [
  {
    uuid: "11111111-1111-4111-8111-111111111111",
    name: "我的世界 1.21.1",
    group: "原版",
    version: "1.21.1",
    versionType: "release",
    loader: "原版",
    loaderVersion: null,
    dir: "我的世界 1.21.1",
    running: false,
  },
  {
    uuid: "22222222-2222-4222-8222-222222222222",
    name: "Fabric 光影测试",
    group: "原版",
    version: "1.19.4",
    versionType: "release",
    loader: "Fabric",
    loaderVersion: "0.16.9",
    dir: "Fabric 光影测试",
    running: false,
  },
  {
    uuid: "33333333-3333-4333-8333-333333333333",
    name: "生存服务器整合包",
    group: "整合包",
    version: "1.20.1",
    versionType: "release",
    loader: "Forge",
    loaderVersion: "47.3.0",
    dir: "生存服务器整合包",
    running: false,
    modpackType: "CurseForge",
    pid: "123456",
    fid: "654321",
    lang: "zh_cn",
    logEncoding: "gbk",
  },
  {
    uuid: "44444444-4444-4444-8444-444444444444",
    name: "科技空岛整合包",
    group: "整合包",
    version: "1.18.2",
    versionType: "release",
    loader: "NeoForge",
    loaderVersion: "21.1.0",
    dir: "科技空岛整合包",
    running: false,
  },
  {
    uuid: "55555555-5555-4555-8555-555555555555",
    name: "Quilt 测试实例",
    group: "测试",
    version: "1.21",
    versionType: "snapshot",
    loader: "Quilt",
    loaderVersion: "0.27.1",
    dir: "Quilt 测试实例",
    running: false,
  },
];

const MOCK_VERSIONS: VersionInfo[] = [
  { id: "1.21.1", versionType: "release" },
  { id: "1.21", versionType: "release" },
  { id: "1.20.6", versionType: "release" },
  { id: "1.20.4", versionType: "release" },
  { id: "1.20.1", versionType: "release" },
  { id: "1.19.4", versionType: "release" },
  { id: "1.18.2", versionType: "release" },
  { id: "1.16.5", versionType: "release" },
  { id: "1.12.2", versionType: "release" },
  { id: "1.8.9", versionType: "release" },
  { id: "1.21.2-pre1", versionType: "snapshot" },
  { id: "24w40a", versionType: "snapshot" },
];

const MOCK_JAVAS: JavaInfo[] = [
  {
    name: "Temurin 21",
    path: "C:\\Program Files\\Eclipse Adoptium\\jdk-21.0.3.9-hotspot\\bin\\java.exe",
    version: "21.0.3",
    major: 21,
    javaType: "Temurin",
    arch: "X86_64",
  },
  {
    name: "OpenJDK 17",
    path: "C:\\Program Files\\Microsoft\\jdk-17.0.11.9-hotspot\\bin\\java.exe",
    version: "17.0.11",
    major: 17,
    javaType: "OpenJDK",
    arch: "X86_64",
  },
  {
    name: "Zulu 8",
    path: "C:\\Program Files\\Zulu\\zulu-8\\bin\\java.exe",
    version: "1.8.0_422",
    major: 8,
    javaType: "Zulu",
    arch: "X86_64",
  },
];

let nextMockId = 1000;
const runningUuids = new Set<string>();
const runtimeLogs: Record<string, string[]> = {};
const extraGroups = new Set<string>();

// ---------- 简易事件总线（模拟 Tauri 事件） ----------

type Handler = (payload: unknown) => void;
const listeners = new Map<string, Set<Handler>>();

function emit(event: string, payload: unknown) {
  listeners.get(event)?.forEach((fn) => fn(payload));
}

function on<T>(event: string, fn: (payload: T) => void): () => void {
  if (!listeners.has(event)) listeners.set(event, new Set());
  listeners.get(event)!.add(fn as Handler);
  return () => {
    listeners.get(event)?.delete(fn as Handler);
  };
}

function now(): string {
  const d = new Date();
  const ms = String(d.getMilliseconds()).padStart(3, "0");
  return `${d.toTimeString().slice(0, 8)}.${ms}`;
}

const delay = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

// ---------- 模拟 API ----------

export const api = {
  async initCore(localDir: string | null, _userName: string): Promise<string> {
    await delay(400);
    return localDir ?? "C:\\Users\\demo\\AppData\\Roaming\\com.coloryr.mcml";
  },

  async getInstances(): Promise<InstanceInfo[]> {
    await delay(150);
    return MOCK_INSTANCES.map((i) => ({ ...i, running: runningUuids.has(i.uuid) }));
  },

  async getJavaList(): Promise<JavaInfo[]> {
    await delay(120);
    return [...MOCK_JAVAS];
  },

  async getVersions(): Promise<VersionInfo[]> {
    await delay(300);
    return [...MOCK_VERSIONS];
  },

  /** 获取全部分组名（实例自带分组 + 手动添加的空分组） */
  async getGroups(): Promise<string[]> {
    await delay(80);
    const names = new Set<string>();
    for (const inst of MOCK_INSTANCES) {
      if (inst.group) names.add(inst.group);
    }
    for (const g of extraGroups) names.add(g);
    return [...names];
  },

  /** 添加空分组，成功返回 true */
  async addGroup(name: string): Promise<boolean> {
    await delay(200);
    const n = name.trim();
    if (!n) return false;
    const exists =
      extraGroups.has(n) || MOCK_INSTANCES.some((i) => i.group === n);
    if (exists) return false;
    extraGroups.add(n);
    emit("instance-change", { type: "group" });
    return true;
  },

  async createInstance(name: string, version: string): Promise<InstanceInfo> {
    await delay(600);
    const inst: InstanceInfo = {
      uuid: `mock-${nextMockId++}`,
      name,
      group: null,
      version,
      versionType: "release",
      loader: "原版",
      loaderVersion: null,
      dir: name,
      running: false,
    };
    MOCK_INSTANCES.unshift(inst);
    emit("instance-change", { type: "add" });
    return inst;
  },

  /** 重命名实例 */
  async renameInstance(uuid: string, name: string): Promise<boolean> {
    await delay(200);
    const inst = MOCK_INSTANCES.find((i) => i.uuid === uuid);
    if (!inst) return false;
    const n = name.trim();
    if (!n) return false;
    inst.name = n;
    inst.dir = n;
    emit("instance-change", { type: "edit" });
    return true;
  },

  /** 更新实例元信息（版本 / 加载器 / 分组 / 整合包 / 语言等） */
  async updateInstance(uuid: string, patch: Partial<InstanceInfo>): Promise<boolean> {
    await delay(120);
    const inst = MOCK_INSTANCES.find((i) => i.uuid === uuid);
    if (!inst) return false;
    Object.assign(inst, patch);
    emit("instance-change", { type: "edit" });
    return true;
  },

  /** 删除实例 */
  async deleteInstance(uuid: string): Promise<boolean> {
    await delay(200);
    const idx = MOCK_INSTANCES.findIndex((i) => i.uuid === uuid);
    if (idx < 0) return false;
    MOCK_INSTANCES.splice(idx, 1);
    runningUuids.delete(uuid);
    emit("instance-change", { type: "remove" });
    return true;
  },

  async launchGame(uuid: string, _userName: string): Promise<void> {
    runningUuids.add(uuid);
    simulateLaunch(uuid);
  },

  async stopGame(uuid: string): Promise<void> {
    runningUuids.delete(uuid);
    emit("game-exit", { uuid, code: 0 } as ExitEvent);
  },

  async getGameLog(uuid: string): Promise<string[]> {
    return [...(runtimeLogs[uuid] ?? [])];
  },

  async getRunning(): Promise<string[]> {
    return [...runningUuids];
  },
};

// ---------- 模拟启动流程 ----------

function pushLog(uuid: string, text: string) {
  (runtimeLogs[uuid] ??= []).push(text);
  emit("game-log", { uuid, time: now(), text, clear: false } as LogEvent);
}

function simulateLaunch(uuid: string) {
  runtimeLogs[uuid] = [];
  emit("game-log", { uuid, time: now(), text: "", clear: true } as LogEvent);

  const setState = (state: string, text: string) => {
    emit("launch-state", { uuid, state } as StateEvent);
    pushLog(uuid, `[状态] ${text}`);
  };

  setState("login", "登录账户");
  setTimeout(() => pushLog(uuid, "[登录] 用时 0.34 秒"), 200);

  setState("readinfo", "读取版本信息");
  setState("check", "检查游戏文件");
  setTimeout(() => pushLog(uuid, "[检查游戏文件] 用时 1.02 秒"), 600);

  setState("jvm", "准备启动参数");
  setTimeout(
    () => pushLog(uuid, "[Java] C:\\Program Files\\Eclipse Adoptium\\jdk-21.0.3.9-hotspot\\bin\\java.exe"),
    800,
  );
  setTimeout(() => pushLog(uuid, "[启动参数] -Xmx4096m -Xms512m --add-opens java.base/java.lang=ALL-UNNAMED ..."), 900);

  setState("end", "启动完成");
  setTimeout(() => pushLog(uuid, "[启动] 用时 3.12 秒"), 1100);
  setTimeout(() => pushLog(uuid, "游戏进程已启动"), 1200);
  setTimeout(() => pushLog(uuid, "[Render thread/INFO]: Backend library version: 0.0.0"), 1600);
  setTimeout(() => pushLog(uuid, "[Render thread/INFO]: Environment: Environment[authSession=..., accounts=1]"), 1900);

  // 持续输出模拟游戏日志，直到停止
  const fakeLines = [
    "[Render thread/INFO]: Reloading ResourceManager: Default",
    "[Render thread/INFO]: Loaded 1247 recipes",
    "[Render thread/INFO]: Preparing start region for dimension minecraft:overworld",
    "[Server thread/INFO]: Done (5.238s)! For help, type \"help\"",
    "[Render thread/INFO]: Time elapsed: 1234 ms",
  ];
  let line = 0;
  const timer = setInterval(() => {
    if (!runningUuids.has(uuid)) {
      clearInterval(timer);
      return;
    }
    pushLog(uuid, fakeLines[line % fakeLines.length]);
    line++;
  }, 2500);
}

// ---------- 事件订阅（模拟） ----------

export function onGameLog(cb: (e: LogEvent) => void): () => void {
  return on<LogEvent>("game-log", cb);
}
export function onLaunchState(cb: (e: StateEvent) => void): () => void {
  return on<StateEvent>("launch-state", cb);
}
export function onGameExit(cb: (e: ExitEvent) => void): () => void {
  return on<ExitEvent>("game-exit", cb);
}
export function onLaunchError(cb: (e: ErrorEvent) => void): () => void {
  return on<ErrorEvent>("launch-error", cb);
}
export function onInstanceChange(cb: () => void): () => void {
  return on("instance-change", cb);
}
