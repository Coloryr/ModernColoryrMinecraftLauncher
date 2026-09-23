// 文件树：把扁平路径列表解析成树，并提供收集工具
// 目录以 "/" 结尾（如 "minecraft/mods/"），其余为文件。

export interface FileNode {
  /** 完整路径（目录以 / 结尾的规范 key，不含尾部斜杠） */
  key: string;
  name: string;
  isDir: boolean;
  children: FileNode[];
  /** 目录子节点尚未加载（懒加载时首次展开会通过事件加载） */
  lazy?: boolean;
}

/** 把扁平路径列表解析为树 */
export function buildTree(paths: string[]): FileNode[] {
  const root: FileNode[] = [];
  const map = new Map<string, FileNode>();
  for (const p of paths) {
    const isDirEntry = p.endsWith("/");
    const parts = p.split("/").filter(Boolean);
    let cur: FileNode[] = root;
    let acc = "";
    for (let i = 0; i < parts.length; i++) {
      acc = acc ? `${acc}/${parts[i]}` : parts[i];
      const isLast = i === parts.length - 1;
      let node = map.get(acc);
      if (!node) {
        node = { key: acc, name: parts[i], isDir: !isLast || isDirEntry, children: [] };
        map.set(acc, node);
        cur.push(node);
      }
      cur = node.children;
    }
  }
  return root;
}

/** 收集所有文件（叶子）key */
export function collectFileKeys(nodes: FileNode[], acc: string[] = []): string[] {
  for (const n of nodes) {
    if (n.isDir) collectFileKeys(n.children, acc);
    else acc.push(n.key);
  }
  return acc;
}

/** 收集所有目录 key */
export function collectDirKeys(nodes: FileNode[], acc: string[] = []): string[] {
  for (const n of nodes) {
    if (n.isDir) {
      acc.push(n.key);
      collectDirKeys(n.children, acc);
    }
  }
  return acc;
}
