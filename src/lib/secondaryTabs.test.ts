// @vitest-environment happy-dom
import { describe, expect, it, vi } from "vitest";

vi.mock("$app/paths", () => ({ base: "" }));

import {
  HOME_TAB,
  appRoutePath,
  clearTabs,
  closeTab,
  getTabs,
  isHomeRoute,
  pushTab,
  setTabDetail,
  subscribeTabs,
  tabIdFor,
  tabTitleFor,
  tabTooltipFor,
  titleForRoute,
  trimTabs,
} from "./secondaryTabs";

// the module keeps its list in memory and mirrors to sessionStorage;
// happy-dom provides a real sessionStorage so pushes persist across imports.

describe("secondary tab path handling", () => {
  it("strips the trailing slash and keeps the root path", () => {
    expect(appRoutePath("/")).toBe("/");
    expect(appRoutePath("/library/")).toBe("/library");
    expect(appRoutePath("")).toBe("/");
  });

  it("treats the toolbox root and /toolbox as the home route", () => {
    expect(isHomeRoute("/")).toBe(true);
    expect(isHomeRoute("/toolbox")).toBe(true);
    expect(isHomeRoute("/library")).toBe(false);
  });

  it("maps known routes to readable titles", () => {
    expect(titleForRoute("/toolbox/epub-style-library")).toBe("EPUB 样式库");
    expect(titleForRoute("/not-a-route")).toBe("");
  });

  it("ignores one-shot refresh params when computing the tab id", () => {
    // /mobile/edit 返回元数据页时带 refresh=Date.now()，只为让页面重读一遍书。
    // 它要是进 id，每返回一次就多挂一份元数据视图（内存里两份元数据 +
    // 两份封面 URL），还会把正在编辑的页面挤出手机端的 3 条上限。
    expect(tabIdFor("/mobile/metadata", "?path=a.epub&refresh=171")).toBe(
      "/mobile/metadata?path=a.epub",
    );
    expect(tabIdFor("/mobile/metadata", "?path=a.epub&refresh=171")).toBe(
      tabIdFor("/mobile/metadata", "?path=a.epub&refresh=999"),
    );
    // 真正区分页面的参数必须保留
    expect(tabIdFor("/mobile/metadata", "?path=a.epub")).not.toBe(
      tabIdFor("/mobile/metadata", "?path=b.epub"),
    );
    // 没有一次性参数时原样保留顺序与编码
    expect(tabIdFor("/reader", "?file=%E4%B9%A6.epub")).toBe(
      "/reader?file=%E4%B9%A6.epub",
    );
  });

  it("strips volatile params without re-encoding the rest", () => {
    // 直接删 key、不过 URLSearchParams：后者会把 "a%20b(1)" 重编码成
    // "a+b%281%29"，于是「带 refresh」与「不带 refresh」算出两个 id，
    // 同一本书被当成两个标签（多挂一份视图，dirtyGuard 也认不出）。
    expect(tabIdFor("/reader", "?file=a%20b(1).epub&refresh=1")).toBe(
      "/reader?file=a%20b(1).epub",
    );
    expect(tabIdFor("/reader", "?file=a%20b(1).epub&refresh=1")).toBe(
      tabIdFor("/reader", "?file=a%20b(1).epub"),
    );
    // 参数在中间时不能留下 "&&"
    expect(tabIdFor("/reader", "?a=1&_t=9&b=2")).toBe("/reader?a=1&b=2");
    // 只有一次性参数 → 没有查询串
    expect(tabIdFor("/reader", "?refresh=9")).toBe("/reader");
    // "ts" 不在名单里：它太通用，误伤比多挂一个视图更糟
    expect(tabIdFor("/reader", "?file=a.epub&ts=9")).toBe(
      "/reader?file=a.epub&ts=9",
    );
  });

  it("keeps the file name in the tab id but not in the visible title", () => {
    // 同窗口开两本书时要能区分：id 带查询串；但标签上只写页面名，
    // 书名放悬停提示，否则标题行会被长书名挤满
    expect(tabIdFor("/reader", "?file=E%3A%5CBooks%5Ca.epub")).toBe(
      "/reader?file=E%3A%5CBooks%5Ca.epub",
    );
    expect(tabIdFor("/reader", "")).toBe("/reader");
    expect(tabTitleFor("/reader")).toBe("EPUB 阅读器");
    expect(
      tabTooltipFor("/reader", "?file=E%3A%5CBooks%5C%E4%B9%A6.epub"),
    ).toBe("EPUB 阅读器 · 书");
    // 没有 file 参数时提示就是页面名，不要出现尾部空分隔符
    expect(tabTooltipFor("/library", "?tab=recent")).toBe("书库");
  });
});

describe("secondary tab state", () => {
  it("registers a page once and ignores the home route", () => {
    pushTab("/library");
    pushTab("/library");
    pushTab("/");
    const ids = getTabs().map((tab) => tab.id);
    expect(ids.filter((id) => id === "/library")).toHaveLength(1);
    expect(ids).not.toContain(HOME_TAB.id);
  });

  it("notifies subscribers and stops after unsubscribe", () => {
    const seen: number[] = [];
    const unsubscribe = subscribeTabs((tabs) => seen.push(tabs.length));
    pushTab("/editor");
    unsubscribe();
    pushTab("/reader");
    // 订阅时立刻回调一次，之后只有 pushTab 触发的那一次
    expect(seen).toHaveLength(2);
  });

  it("removes a tab by id but never the home tab", () => {
    pushTab("/reader");
    closeTab("/reader");
    expect(getTabs().some((tab) => tab.id === "/reader")).toBe(false);
    closeTab(HOME_TAB.id);
    expect(getTabs()).not.toContainEqual(HOME_TAB);
  });

  it("prefers a page-supplied detail over the ?file= label", () => {
    // web 版 EPUB 编辑器用 <input type=file> 选书，地址里没有书名，
    // 只能由页面自己上报（可以是多行）
    pushTab("/toolbox/epub-editor");
    setTabDetail(
      "/toolbox/epub-editor",
      "测试书\n当前文件：OEBPS/Text/c1.xhtml",
    );
    const tab = getTabs().find((item) => item.id === "/toolbox/epub-editor");
    expect(tab?.detail).toBe("测试书\n当前文件：OEBPS/Text/c1.xhtml");
    expect(
      tabTooltipFor("/toolbox/epub-editor", "", "EPUB 编辑器", tab?.detail),
    ).toBe("测试书\n当前文件：OEBPS/Text/c1.xhtml");
    // 清空后回到只显示页面名
    setTabDetail("/toolbox/epub-editor", "");
    expect(
      getTabs().find((item) => item.id === "/toolbox/epub-editor")?.detail,
    ).toBe(undefined);
  });

  it("keeps tab detail out of the visible title", () => {
    pushTab("/reader", "?file=a.epub");
    expect(
      getTabs().find((tab) => tab.id === "/reader?file=a.epub")?.title,
    ).toBe("EPUB 阅读器");
  });
});

/**
 * 手机外壳不渲染标签条，标签只增不减；KeepAliveViews 的存活集合正是
 * 这份标签表，所以必须裁剪。裁剪是自动的（用户没点关闭），因此规则要严：
 * 当前页不能裁，带未保存修改的页面不能裁。
 */
describe("trimTabs", () => {
  function seed(...paths: string[]) {
    clearTabs();
    for (const path of paths) pushTab(path);
  }

  it("keeps the newest tabs and drops the oldest", () => {
    seed("/a", "/b", "/c", "/d");
    trimTabs(2);
    expect(getTabs().map((tab) => tab.id)).toEqual(["/c", "/d"]);
  });

  it("never trims a tab listed in keep", () => {
    seed("/a", "/b", "/c", "/d");
    // /a 是最旧的，但它是"当前页"，必须留下
    trimTabs(2, { keep: ["/a"] });
    const ids = getTabs().map((tab) => tab.id);
    expect(ids).toContain("/a");
    expect(ids).toContain("/d");
    expect(ids).toHaveLength(3);
  });

  it("keeps tabs whose page refuses to be evicted", () => {
    seed("/a", "/b", "/c", "/d");
    // /b 里有未保存的编辑：宁可多挂一个页面，也不能丢用户改动
    trimTabs(2, { canEvict: (id) => id !== "/b" });
    const ids = getTabs().map((tab) => tab.id);
    expect(ids).toEqual(["/b", "/c", "/d"]);
  });

  it("does nothing when the list is already within the limit", () => {
    seed("/a", "/b");
    const seen: number[] = [];
    const unsubscribe = subscribeTabs((tabs) => seen.push(tabs.length));
    trimTabs(5);
    unsubscribe();
    // 只有订阅时的立刻回调，没有第二次提交
    expect(seen).toEqual([2]);
    expect(getTabs()).toHaveLength(2);
  });

  it("keeps everything when every tab refuses eviction", () => {
    seed("/a", "/b", "/c");
    trimTabs(1, { canEvict: () => false });
    expect(getTabs()).toHaveLength(3);
  });

  it("can clear the phone's clean tool pages when returning home", () => {
    seed("/a", "/b", "/c");
    trimTabs(0, { canEvict: (id) => id !== "/b" });
    // 回首页代表一次工具会话结束；只有带未保存改动的页面继续保活。
    expect(getTabs().map((tab) => tab.id)).toEqual(["/b"]);
  });

  /**
   * 裁剪按数组顺序理解为"最近使用"，所以重新访问必须挪到末尾。
   * 桌面标签栏的顺序是用户自己开的顺序，不会自己跳（promote 只有手机传）。
   */
  it("moves a revisited tab to the end only when promote is set", () => {
    seed("/a", "/b", "/c");
    pushTab("/a", "", undefined, undefined, { promote: true });
    expect(getTabs().map((tab) => tab.id)).toEqual(["/b", "/c", "/a"]);
  });

  it("keeps the desktop tab order stable without promote", () => {
    seed("/a", "/b", "/c");
    pushTab("/a");
    expect(getTabs().map((tab) => tab.id)).toEqual(["/a", "/b", "/c"]);
  });

  it("evicts the least recently used page on the phone after a revisit", () => {
    // A→B→C→A→D：最久没用的是 B，不是刚看过的 A
    seed("/a", "/b", "/c");
    pushTab("/a", "", undefined, undefined, { promote: true });
    pushTab("/d");
    trimTabs(3, { keep: ["/d"] });
    expect(getTabs().map((tab) => tab.id)).toEqual(["/c", "/a", "/d"]);
  });

  it("keeps a ? inside a stored query value when hydrating", async () => {
    // 只按第一个 ? 拆分 id。旧实现用 split("?")[1] 会把查询值本身带的 ?
    // 后面整段丢掉，刷新后同一本书的标签 id 变了，保活视图也会重建。
    sessionStorage.setItem(
      "tepub-secondary-tabs",
      JSON.stringify([
        {
          id: "/reader?file=a?b.epub&refresh=171",
          href: "/reader?file=a?b.epub&refresh=171",
          title: "旧标题",
        },
      ]),
    );
    vi.resetModules();
    const fresh = await import("./secondaryTabs");

    expect(fresh.getTabs()[0]?.id).toBe("/reader?file=a?b.epub");
    expect(fresh.getTabs()[0]?.href).toBe("/reader?file=a?b.epub");
    sessionStorage.removeItem("tepub-secondary-tabs");
  });
});
