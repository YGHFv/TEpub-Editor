/**
 * 阅读页的原始文本缓存。
 *
 * 同窗口模式下切标签会让阅读页重新挂载（SvelteKit 的 goto 是 SPA 导航，
 * 但页面组件仍然会整个重建），于是每次回到阅读器标签都要重新 fetch 几百个
 * 章节与样式表 —— 一本 800 章的书实测要发 800 多次 asset 请求、2~3 秒。
 * 用户看到的就是「切回去像重新加载」。
 *
 * 这里按「源路径 + 修改时间 + 大小 + 解压目录」缓存每个文件的文本。
 * 解压目录必须参与 key：重启后 temp dir 会换，章节 HTML 里的图片/CSS 是
 * 指向它的 asset URL，换目录后旧 URL 全部失效。
 *
 * 只缓存文本，不缓存 DOM —— DOMParser 的结果不能跨页面生命周期复用。
 */

const MAX_BOOKS = 2;

/**
 * 只留最近两本：一本书的章节文本通常 1~10 MB，留太多会一直吃内存。
 * 同窗口里通常也就来回切一两本书。
 */
const caches = new Map<string, Map<string, string>>();

export function readerCacheKey(
  sourcePath: string,
  modifiedMs: number,
  size: number,
  extractDir: string,
): string {
  // 修改时间和大小都要：只比时间的话，同一毫秒内改写的文件会被误判
  return [sourcePath, modifiedMs, size, extractDir].join("|");
}

/** 取某本书的「文件路径 → 文本」缓存；没有就建一个并淘汰最旧的 */
export function bookTextCache(key: string): Map<string, string> {
  if (!key) return new Map();
  const existing = caches.get(key);
  if (existing) {
    // LRU：命中的挪到末尾
    caches.delete(key);
    caches.set(key, existing);
    return existing;
  }
  const created = new Map<string, string>();
  caches.set(key, created);
  while (caches.size > MAX_BOOKS) {
    const oldest = caches.keys().next().value;
    if (oldest === undefined) break;
    caches.delete(oldest);
  }
  return created;
}

export function clearReaderTextCache(): void {
  caches.clear();
}
