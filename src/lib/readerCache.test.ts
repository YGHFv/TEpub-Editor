import { describe, expect, it } from "vitest";

import {
  bookTextCache,
  clearReaderCache,
  readerCacheKey,
  readReaderCache,
  writeReaderCache,
  type ReaderBookSnapshot,
} from "./readerCache";

function snapshot(title: string): ReaderBookSnapshot {
  return {
    spine: [{ path: "OEBPS/c1.xhtml", title }],
    toc: [{ title, spineIdx: 0, children: [] }],
    title,
    author: "",
    chapterFullHtml: [`<section data-idx="0">${title}</section>`],
    chapterBgStyle: [""],
  };
}

describe("reader text cache", () => {
  it("keys on source identity and extract dir", () => {
    const a = readerCacheKey("E:/books/a.epub", 1000, 42, "E:/tmp/x");
    const b = readerCacheKey("E:/books/a.epub", 1001, 42, "E:/tmp/x");
    const c = readerCacheKey("E:/books/a.epub", 1000, 42, "E:/tmp/y");
    expect(a).not.toBe(b);
    expect(a).not.toBe(c);
  });

  it("returns the same map for the same book and isolates different books", () => {
    clearReaderCache();
    const first = bookTextCache("book-a");
    first.set("OEBPS/c1.xhtml", "<p>1</p>");
    // 切标签回来走过的路径：同一个键必须拿到同一份数据
    expect(bookTextCache("book-a").get("OEBPS/c1.xhtml")).toBe("<p>1</p>");
    expect(bookTextCache("book-b").get("OEBPS/c1.xhtml")).toBeUndefined();
  });

  it("keeps at most two books and drops the least recently used", () => {
    clearReaderCache();
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

describe("reader snapshot cache", () => {
  it("round-trips a parsed book and misses on a different key", () => {
    clearReaderCache();
    writeReaderCache("k1", snapshot("甲"));
    expect(readReaderCache("k1")?.chapterFullHtml[0]).toContain("甲");
    expect(readReaderCache("k2")).toBeNull();
    expect(readReaderCache("")).toBeNull();
  });

  it("evicts the least recently used book", () => {
    clearReaderCache();
    writeReaderCache("k1", snapshot("1"));
    writeReaderCache("k2", snapshot("2"));
    // 读 k1 让它变成最近使用，再插第三本，k2 应当被淘汰
    readReaderCache("k1");
    writeReaderCache("k3", snapshot("3"));
    expect(readReaderCache("k1")?.title).toBe("1");
    expect(readReaderCache("k3")?.title).toBe("3");
    expect(readReaderCache("k2")).toBeNull();
  });

  it("clears both layers at once", () => {
    clearReaderCache();
    writeReaderCache("k", snapshot("x"));
    bookTextCache("k").set("f", "1");
    clearReaderCache();
    expect(readReaderCache("k")).toBeNull();
    expect(bookTextCache("k").size).toBe(0);
  });
});
