import { describe, expect, it, vi } from "vitest";

vi.mock("$app/paths", () => ({ base: "" }));

import {
  HOME_TAB,
  appRoutePath,
  closeTab,
  getTabs,
  isHomeRoute,
  pushTab,
  subscribeTabs,
  tabIdFor,
  tabTitleFor,
  tabTooltipFor,
  titleForRoute,
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
});
