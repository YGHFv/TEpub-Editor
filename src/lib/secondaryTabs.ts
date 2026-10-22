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
  /**
   * 页面自己登记的悬停说明，只出现在 tooltip 里，不占标签宽度。
   *
   * 有些页面把文件放在查询串里（/reader?file=…），fileLabel 能直接读；
   * web 版 EPUB 编辑器是用 <input type=file> 选书的，地址里没有书，
   * 只能由页面自己上报，否则标签显示「EPUB 编辑器」、悬停也是同一句。
   *
   * 允许多行：编辑器这类页面按「书 → 当前章节/文件」分行写，
   * 原生 title 会照换行显示（多级信息一眼看全）。
   */
  detail?: string;
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
  "/mobile/make": "TXT 转 EPUB",
  "/mobile/edit": "EPUB 文件编辑",
  "/mobile/metadata": "EPUB 元数据",
  "/mobile/decrypt": "EPUB 解密",
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

/**
 * 只影响「标签身份」的时间戳参数：地址里还留着它们，页面的
 * `window.location.search` 读得到，但标签 id 里不算。
 *
 * 典型来源：手机端从文件编辑器返回元数据页时带 `refresh=Date.now()`，
 * 目的是让页面重新读一遍书。如果它进 id，每返回一次就多挂一个
 * /mobile/metadata 视图（内存里两份元数据、两份封面 URL），
 * 还会把真正在编辑的页面挤出手机端的 3 条上限。
 *
 * `ts` 太通用（时间戳、tab 序号的缩写都有人用），只留明确表示「重新读一次」
 * 的 refresh / _t。宁可多挂一个视图，也不要把两个真页面并成一个。
 */
const VOLATILE_SEARCH_KEYS = ["refresh", "_t"];

const VOLATILE_PARAM_PATTERN = new RegExp(
  `(^|&)(?:${VOLATILE_SEARCH_KEYS.join("|")})=[^&]*`,
  "g",
);

function stableSearch(search: string) {
  const raw = (search || "").replace(/^\?/, "");
  if (!raw) return "";
  // 直接在原串上按 key 抠掉，不经过 URLSearchParams：
  // 它会把 "a%20b(1)" 重新编码成 "a+b%281%29"、把 "~" 变成 "%7E"。
  // 那样「带 refresh」与「不带 refresh」会得到两个不同的 id —— 同一本书
  // 被当成两个标签，多挂一份视图，dirtyGuard 也认不出是同一个人。
  const rest = raw
    .replace(VOLATILE_PARAM_PATTERN, "")
    // 抠掉的是第一段 → 留下前导 "&"；抠掉的是最后一段 → 留下结尾的 "&"
    .replace(/^&+/, "")
    .replace(/&+$/, "")
    // 夹在两段中间时两边各留一个 "&"，收成一个
    .replace(/&&+/g, "&");
  return rest ? `?${rest}` : "";
}

/** 标签 id：路径 + 查询串（去掉 refresh 这类一次性参数） */
export function tabIdFor(pathname: string, search = "") {
  return appRoutePath(pathname) + stableSearch(search);
}

/** 只按第一个 ? 拆分路径和查询串，查询值里允许继续出现 ?。 */
function splitTabId(id: string) {
  const queryAt = id.indexOf("?");
  if (queryAt < 0) return { path: id, search: "" };
  return {
    path: id.slice(0, queryAt),
    search: id.slice(queryAt + 1),
  };
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

/**
 * 悬停提示：页面名 + 文件名，用来区分同类型的多个标签。
 *
 * `detail` 是页面自己上报的说明（web 版 EPUB 编辑器用它显示书名），
 * 有它就优先用它 —— 地址里读不到文件时这是唯一的信息来源。
 */
export function tabTooltipFor(
  pathname: string,
  search = "",
  title?: string,
  detail?: string,
) {
  const label = title || tabTitleFor(pathname);
  const custom = (detail || "").trim();
  // 页面自己上报时原样用（可能已经是多行「书 ⏎ 当前文件」），
  // 不再拼「页面名 · 」—— 否则第一行会变成「EPUB 编辑器 · 书名」，
  // 多级结构被压平，和顶部标签栏的短标签重复
  if (custom) return custom;
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
    // 先把旧格式的 id 过一遍 stableSearch，再按 id 去重。
    //
    // sessionStorage 会跨版本更新存活：升级前存下的
    // "/mobile/metadata?path=A&refresh=171" 与新算出的
    // "/mobile/metadata?path=A" 是同一个页面。不归一化的话，刷新后先恢复
    // 两条旧标签、再 pushTab 出第三条 —— 手机上 3 个名额直接被一个页面吃光，
    // promote 也合并不了它们。
    const seen = new Set<string>();
    const normalized: SecondaryTab[] = [];
    for (const tab of parsed) {
      if (!tab || typeof tab.id !== "string") continue;
      const stored = splitTabId(tab.id);
      const id =
        tab.id === HOME_TAB.id
          ? tab.id
          : tabIdFor(stored.path, stored.search);
      const path = splitTabId(id).path;
      const next: SecondaryTab = {
        ...tab,
        id,
        href: tab.id === HOME_TAB.id ? tab.href : `${base}${id}`,
        // 标题只用路径算，查询串不参与；老版本可能存过
        // 「EPUB 阅读器 · 书名」，按路径重算一遍统一成短标题
        title: tabTitleFor(path) || tab.title || path,
        // detail 是本窗口临时登记的（书名/当前文件），刷新后不复原，
        // 页面挂载时会重新上报；这里清掉免得显示上一本书的名字
        detail: undefined,
      };
      // 归一化后撞车的，保留后出现的那条（它更接近用户最后看到的顺序）
      if (seen.has(id)) {
        const at = normalized.findIndex((item) => item.id === id);
        if (at >= 0) normalized.splice(at, 1);
      }
      seen.add(id);
      normalized.push(next);
    }
    return normalized;
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
 *
 * `promote` 只给手机外壳用：把重新访问的标签挪到末尾。桌面标签栏的
 * 顺序是用户自己开的顺序（跟浏览器标签一样不该自己跳），手机端不渲染
 * 标签栏、trimTabs 却按数组顺序裁，所以只有手机需要"最近使用"语义。
 */
export function pushTab(
  pathname: string,
  search = "",
  title?: string,
  detail?: string,
  options: { promote?: boolean } = {},
) {
  if (isHomeRoute(pathname)) return getTabs();
  const path = appRoutePath(pathname);
  const id = tabIdFor(path, search);
  const label = title || tabTitleFor(path) || path;
  const existing = tabs.find((tab) => tab.id === id);
  if (existing) {
    const nextDetail = detail === undefined ? existing.detail : detail;
    const next = { ...existing, title: label, detail: nextDetail || undefined };
    const changed = existing.title !== label || existing.detail !== nextDetail;
    // 手机：重新访问 = 变成最新的一个，挪到数组末尾。
    //
    // 数组顺序就是 trimTabs 与「更多」抽屉理解的"最近使用顺序"。以前这里
    // 只在原地更新，顺序停在**首次登记**的顺序上 —— 于是手机端
    // A→B→C→A→D 之后，最久没用的 B 活下来，刚刚还在看的 A 被裁掉。
    // 桌面不挪：标签栏顺序是用户自己开的顺序，跟浏览器标签一样不该自己跳。
    const needsPromote =
      options.promote === true && tabs[tabs.length - 1].id !== id;
    if (changed || needsPromote) {
      commit(
        needsPromote
          ? [...tabs.filter((tab) => tab.id !== id), next]
          : tabs.map((tab) => (tab.id === id ? next : tab)),
      );
    }
    return getTabs();
  }
  // 必须接在现有标签后面。这里曾经写成 commit([{ … }])，把整个数组替换成
  // 一个新建标签 —— 表现是顶部标签栏永远只有一条，开第二个页面就把第一个
  // 顶掉。pushTab 的语义是追加，不是替换。
  commit([
    ...tabs,
    { id, href: `${base}${id}`, title: label, detail: detail || undefined },
  ]);
  return getTabs();
}

/**
 * 只更新悬停说明，不动标题。页面在内容变化时调用（打开另一本书、切换文件）。
 * 标签不存在时什么都不做 —— 首页没有标签。
 */
export function setTabDetail(id: string, detail: string) {
  const clean = (detail || "").trim();
  const existing = tabs.find((tab) => tab.id === id);
  if (!existing || (existing.detail || "") === clean) return getTabs();
  commit(
    tabs.map((tab) =>
      tab.id === id ? { ...tab, detail: clean || undefined } : tab,
    ),
  );
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

/**
 * 只保留最近 `max` 个标签，丢掉更早的。
 *
 * 桌面上标签由用户手动关，条数天然有界。手机外壳不渲染标签条 —— 「更多」
 * 抽屉里的列表只增不减，用户在上面逛十几页之后，那个抽屉比标签栏还难用，
 * sessionStorage 里也堆着一串早就不存在的历史。手机每次登记后调一次即可。
 *
 * `keep` 里的 id 永不裁剪（当前页必须传进去，否则会把用户正在看的页面删掉）；
 * `canEvict` 给页面一次否决权 —— 带着未暂存修改的编辑器不能被静默卸载。
 * 两者都不满足时宁可不裁：多挂一个页面是内存问题，丢用户改动是数据问题。
 *
 * 所以 "max" 是软上限：所有超出的页面都在否决时，一个也不裁。这是有意的
 * —— 否决只来自"内存里真有没保存的东西"，而那些页面本来就该被用户先处理。
 */
export function trimTabs(
  max: number,
  options: { keep?: string[]; canEvict?: (id: string) => boolean } = {},
) {
  if (tabs.length <= max) return getTabs();
  const keep = new Set(options.keep || []);
  const canEvict = options.canEvict || (() => true);

  // 从最新往旧走，留下前 max 个可保留项
  const survivors: SecondaryTab[] = [];
  for (let index = tabs.length - 1; index >= 0; index -= 1) {
    const tab = tabs[index];
    const mustKeep = keep.has(tab.id) || survivors.length < max;
    if (mustKeep) {
      survivors.push(tab);
      continue;
    }
    if (!canEvict(tab.id)) {
      survivors.push(tab);
    }
  }
  if (survivors.length === tabs.length) return getTabs();
  commit(survivors.reverse());
  return getTabs();
}

export { usesMultiWindow };
