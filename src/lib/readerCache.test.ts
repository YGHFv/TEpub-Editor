import { describe, expect, it } from "vitest";

import {
  bookTextCache,
  clearReaderTextCache,
  readerCacheKey,
} from "./readerCache";

describe("reader text cache", () => {
  it("keys on source identity and extract dir", () => {
    const a = readerCacheKey("E:/books/a.epub", 1000, 42, "E:/tmp/x");
    const b = readerCacheKey("E:/books/a.epub", 1001, 42, "E:/tmp/x");
    const c = readerCacheKey("E:/books/a.epub", 1000, 42, "E:/tmp/y");
    expect(a).not.toBe(b);
    expect(a).not.toBe(c);
  });

  it("returns the same map for the same book and isolates different books", () => {
    clearReaderTextCache();
    const first = bookTextCache("book-a");
    first.set("OEBPS/c1.xhtml", "<p>1</p>");
    // 切标签回来走过的路径：同一个键必须拿到同一份数据
    expect(bookTextCache("book-a").get("OEBPS/c1.xhtml")).toBe("<p>1</p>");
    expect(bookTextCache("book-b").get("OEBPS/c1.xhtml")).toBeUndefined();
  });

  it("keeps at most two books and drops the least recently used", () => {
    clearReaderTextCache();
    bookTextCache("one").set("f", "1");
    bookTextCache("two").set("f", "2");
    // 访问 one，让它变成最近使用
    bookTextCache("one");
    bookTextCache("three").set("f", "3");
    expect(bookTextCache("one").get("f")).toBe("1");
    expect(bookTextCache("three").get("f")).toBe("3");
    // two 是最久未用的，应当已被淘汰
    expect(bookTextCache("two").get("f")).toBeUndefined();
  });

  it("ignores an empty key so pages without a path never share one bucket", () => {
    const map = bookTextCache("");
    map.set("a", "1");
    expect(bookTextCache("").get("a")).toBeUndefined();
  });
});
