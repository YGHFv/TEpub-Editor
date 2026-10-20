/**
 * 阅读页的跨标签缓存。
 *
 * 同窗口模式下切标签会让阅读页重新挂载（SvelteKit 的 goto 是 SPA 导航，
 * 但页面组件仍然会整个重建），于是每次回到阅读器标签都要把整本书重新
 * 走一遍流水线 —— 一本 800 章的书要发 800 多次 asset 请求、800 次
 * DOMParser、再逐章序列化，实测 4~9 秒。用户看到的就是「切回去像重新加载」。
 *
 * 两层缓存，键都是 `readerCacheKey(...)`：
 * - 文本层：每个章节 / 样式表的原始文本，省掉网络与 IPC；
 * - 快照层：解析 + 序列化之后的成品（每章 section HTML、每章背景样式、
 *   spine、目录、书名），省掉 DOMParser 与 CSS 处理。
 *
 * 解压目录必须参与 key：重启后 temp dir 会换，章节 HTML 里的图片/CSS 是
 * 指向它的 asset URL，换目录后旧 URL 全部失效。
 *
 * 只缓存「与阅读进度无关」的东西：排版仍要重新测量（视图尺寸、字号都可能
 * 变），当前页仍要从 localStorage 的进度里恢复。
 */

const MAX_BOOKS = 2;

export interface CachedSpineEntry {
  path: string;
  title: string;
}

/** 目录节点（与 reader 页的 TocNode 同构，避免循环依赖不在这里 import） */
export interface CachedTocNode {
  title: string;
  spineIdx: number;
  children: CachedTocNode[];
}

/** 一本书解析完成后的成品。全部是「书 + 解压目录」的纯函数 */
export interface ReaderBookSnapshot {
  spine: CachedSpineEntry[];
  toc: CachedTocNode[];
  title: string;
  author: string;
  chapterFullHtml: string[];
  chapterBgStyle: string[];
}

/**
 * 只留最近两本：快照里是整本的章节 HTML（1~10 MB 字符串），
 * 留太多会一直吃内存。同窗口里通常也就来回切一两本书。
 */
const textCaches = new Map<string, Map<string, string>>();
const snapshots = new Map<string, ReaderBookSnapshot>();

export function readerCacheKey(
  sourcePath: string,
  modifiedMs: number,
  size: number,
  extractDir: string,
): string {
  // 修改时间和大小都要：只比时间的话，同一毫秒内改写的文件会被误判
  return [sourcePath, modifiedMs, size, extractDir].join("|");
}

/** 把命中/插入的条挪到 Map 末尾，实现 LRU 淘汰 */
function touch<V>(map: Map<string, V>, key: string, value: V): V {
  map.delete(key);
  map.set(key, value);
  while (map.size > MAX_BOOKS) {
    const oldest = map.keys().next().value;
    if (oldest === undefined) break;
    map.delete(oldest);
  }
  return value;
}

/** 取某本书的「文件路径 → 文本」缓存；没有就建一个并淘汰最旧的 */
export function bookTextCache(key: string): Map<string, string> {
  if (!key) return new Map();
  const existing = textCaches.get(key);
  if (existing) return touch(textCaches, key, existing);
  return touch(textCaches, key, new Map<string, string>());
}

/** 取整本解析快照；没有返回 null（调用方走完整解析流程） */
export function readReaderCache(key: string): ReaderBookSnapshot | null {
  if (!key) return null;
  const hit = snapshots.get(key);
  return hit ? touch(snapshots, key, hit) : null;
}

export function writeReaderCache(
  key: string,
  snapshot: ReaderBookSnapshot,
): void {
  if (!key) return;
  touch(snapshots, key, snapshot);
}

export function clearReaderCache(): void {
  textCaches.clear();
  snapshots.clear();
}
