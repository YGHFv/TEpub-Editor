/**
 * 同窗口二级页面的标签状态。
 *
 * 默认（设置里「使用多窗口打开二级页面」关闭）二级页面在主窗口内跳转，
 * 顶部因此需要一条标签栏把已经打开的页面列出来，第一个固定是工具箱。
 * 标签只属于当前窗口：放 sessionStorage，新开窗口是干净的一套。
 */
import { base } from "$app/paths";
import { usesMultiWindow } from "$lib/appSettings";

export interface SecondaryTab {
  /** 路径 + 查询串作为 id：同一本书反复打开只保留一个标签，
      但 /reader?file=A 和 /reader?file=B 是两本书，必须分开 */
  id: string;
  href: string;
  title: string;
}

const STORAGE_KEY = "tepub-secondary-tabs";

/** 路径 → 标题。二级页面的标题不读 document.title（各页设置得不一样） */
const ROUTE_TITLES: Record<string, string> = {
  "/library": "书库",
  "/editor": "TXT 编辑器",
  "/epub-editor": "EPUB 编辑器",
  "/reader": "EPUB 阅读器",
  "/epub-metadata": "EPUB 元数据",
  "/search-replace": "查找替换",
  "/batch-progress": "批量处理",
  "/toolbox/epub-editor": "EPUB 编辑器",
  "/toolbox/text-editor": "TXT 编辑器",
  "/toolbox/epub-style-library": "EPUB 样式库",
  "/toolbox/make-epub": "TXT 转 EPUB",
  "/toolbox/image-tools": "封面与横幅",
  "/toolbox/online-image-download": "EPUB 在线图片下载",
  "/toolbox/epub-advanced": "EPUB 进阶工具",
  "/toolbox/epub-process": "EPUB 处理",
  "/toolbox/epub-diagnose": "EPUB 结构诊断",
  "/toolbox/font-process": "EPUB 字体处理",
  "/mobile": "移动端工具",
};

/** 首页（工具箱）永远是第一个标签。href 带 base 前缀，部署在子路径下也能跳对 */
export const HOME_TAB: SecondaryTab = {
  id: "__home__",
  href: `${base}/`,
  title: "工具箱",
};

/** 去掉 base 前缀，得到与 ROUTE_TITLES 对齐的应用内路径 */
export function appRoutePath(pathname: string) {
  let path = pathname || "/";
  const prefix = (base || "").replace(/\/$/, "");
  if (prefix && path.startsWith(prefix))
    path = path.slice(prefix.length) || "/";
  if (path.length > 1 && path.endsWith("/")) path = path.replace(/\/+$/, "");
  return path || "/";
}

export function isHomeRoute(pathname: string) {
  const path = appRoutePath(pathname);
  return path === "/" || path === "/toolbox";
}

export function titleForRoute(pathname: string) {
  return ROUTE_TITLES[appRoutePath(pathname)] || "";
}

/** 取 ?file= 里的文件名（去扩展名）。只用于悬停提示：
    标签上要短，一长串书名会把标题行挤满 */
export function fileLabel(search: string) {
  if (!search) return "";
  const value = new URLSearchParams(search.replace(/^\?/, "")).get("file");
  if (!value) return "";
  const name = value.split(/[\\/]/).pop() || "";
  return name.replace(/\.[^.]+$/, "");
}

/** 规范化查询串：""/"?"/"?a=1" → ""/"a=1" 形式，保证 id 稳定 */
function normalizeSearch(search: string) {
  const raw = (search || "").replace(/^\?/, "");
  return raw ? `?${raw}` : "";
}

/** 标签 id：路径 + 查询串 */
export function tabIdFor(pathname: string, search = "") {
  return appRoutePath(pathname) + normalizeSearch(search);
}

/**
 * 标签标题：只显示页面名。
 * 带上书名虽然能区分同类型的两个标签，但标题行会被挤满，
 * 长书名还会互相截断成「EPUB 编辑器 · reader-fixt…」，反而更乱。
 * 书名放在悬停提示里（见 tabTooltipFor）。
 */
export function tabTitleFor(pathname: string) {
  return titleForRoute(pathname);
}

/** 悬停提示：页面名 + 文件名，用来区分同类型的多个标签 */
export function tabTooltipFor(pathname: string, search = "", title?: string) {
  const label = title || tabTitleFor(pathname);
  const file = fileLabel(search);
  return file ? `${label} · ${file}` : label;
}

function safeSession(): Storage | null {
  try {
    return typeof sessionStorage === "undefined" ? null : sessionStorage;
  } catch {
    return null;
  }
}

function readStored(): SecondaryTab[] {
  const store = safeSession();
  if (!store) return [];
  try {
    const raw = store.getItem(STORAGE_KEY);
    const parsed = raw ? JSON.parse(raw) : [];
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(
        (tab): tab is SecondaryTab =>
          Boolean(tab) &&
          typeof tab.id === "string" &&
          typeof tab.href === "string",
      )
      .map((tab) => {
        // id 形如 "/reader?file=..."：标题只用路径算，查询串不参与。
        // 老版本可能存过「EPUB 阅读器 · 书名」，按路径重算一遍统一成短标题
        const path = tab.id.split("?")[0];
        return { ...tab, title: tabTitleFor(path) || tab.title || path };
      });
  } catch {
    return [];
  }
}

let tabs: SecondaryTab[] = readStored();
const listeners = new Set<(tabs: SecondaryTab[]) => void>();

function commit(next: SecondaryTab[]) {
  tabs = next;
  const store = safeSession();
  try {
    store?.setItem(STORAGE_KEY, JSON.stringify(tabs));
  } catch {
    // 隐私模式写不进去：本次会话内内存态仍然有效
  }
  for (const listener of listeners) listener(tabs);
}

export function getTabs(): SecondaryTab[] {
  return tabs;
}

export function subscribeTabs(listener: (tabs: SecondaryTab[]) => void) {
  listeners.add(listener);
  listener(tabs);
  return () => {
    listeners.delete(listener);
  };
}

/**
 * 记录一次二级页面访问；同一页面已存在时只更新标题，不新增标签。
 * 查询串参与 id：「阅读器 + 某本书」才算同一个标签。
 */
export function pushTab(pathname: string, search = "", title?: string) {
  if (isHomeRoute(pathname)) return getTabs();
  const path = appRoutePath(pathname);
  const id = tabIdFor(path, search);
  const label = title || tabTitleFor(path) || path;
  const existing = tabs.find((tab) => tab.id === id);
  if (existing) {
    if (existing.title !== label) {
      commit(
        tabs.map((tab) => (tab.id === id ? { ...tab, title: label } : tab)),
      );
    }
    return getTabs();
  }
  commit([...tabs, { id, href: `${base}${id}`, title: label }]);
  return getTabs();
}

export function closeTab(id: string) {
  if (id === HOME_TAB.id) return getTabs();
  commit(tabs.filter((tab) => tab.id !== id));
  return getTabs();
}

/** 当前页在标签栏里的 id（无标签栏时也能安全调用） */
export function currentTabId(href = "") {
  const url = href || (typeof location === "undefined" ? "" : location.href);
  try {
    const parsed = new URL(url, "http://localhost");
    return tabIdFor(parsed.pathname, parsed.search);
  } catch {
    return tabIdFor(url);
  }
}

/**
 * 页面内点自己的「关闭」按钮时调用：把当前标签一起关掉，
 * 否则回到上一页后标签栏里还挂着一条已经关掉的页面。
 * 返回关闭后剩下的标签里，应当跳转过去的那一个（没有则 null）。
 */
export function closeCurrentTab() {
  const id = currentTabId();
  const remaining = closeTab(id);
  // 优先回到最近打开的另一个二级页面，没有就交给调用方回首页
  return remaining.length ? remaining[remaining.length - 1] : null;
}

export function clearTabs() {
  commit([]);
  return getTabs();
}

export { usesMultiWindow };
