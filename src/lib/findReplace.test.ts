import { describe, expect, it } from "vitest";
import {
  buildFindRegex,
  countMatchesInContent,
  escapeRegExpLiteral,
  expandExtendedEscapes,
  filterFilesInScope,
  findAllMatches,
  findNextMatch,
  findNextMatchWrapped,
  replaceAllInContent,
  type FindQuery,
} from "./findReplace";

function textQuery(pattern: string, extra: Partial<FindQuery> = {}): FindQuery {
  return { pattern, mode: "text", ...extra };
}

function regexQuery(pattern: string, extra: Partial<FindQuery> = {}): FindQuery {
  return { pattern, mode: "regex", ...extra };
}

describe("escapeRegExpLiteral", () => {
  it("escapes regex metacharacters so text patterns match literally", () => {
    const source = "a.b*c(d)[e]{f}^$|?+\\";
    const regex = new RegExp(escapeRegExpLiteral(source));
    expect(regex.test(source)).toBe(true);
    expect(new RegExp(escapeRegExpLiteral("a.b")).test("axb")).toBe(false);
  });

  it("stays valid under the u-flag dialect used by CodeMirror SearchQuery", () => {
    // Editor.svelte 的 wholeWord 包装把产物喂给 SearchQuery（内部 "gmu" 标志），
    // 因此转义结果不允许出现 `\-` 这类 u 模式非法转义
    for (const sample of ["a-b", "a/b", "price$", "x{1}", "back\\slash", "(par)"]) {
      expect(() => new RegExp(`\\b${escapeRegExpLiteral(sample)}\\b`, "gmu")).not.toThrow();
    }
    // 词边界包装后仍按字面量整词命中（\b 依赖两侧词字符，取词字符收尾的样例）
    for (const sample of ["a-b", "a/b", "x1"]) {
      const regex = new RegExp(`\\b${escapeRegExpLiteral(sample)}\\b`, "gmu");
      expect(`x ${sample} y`.match(regex)).toEqual([sample]);
    }
  });
});

describe("expandExtendedEscapes", () => {
  it("expands literal \\n, \\t and \\r into control characters", () => {
    expect(expandExtendedEscapes("a\\nb\\tc\\rd")).toBe("a\nb\tc\rd");
    expect(expandExtendedEscapes("\\r\\n")).toBe("\r\n");
  });

  it("leaves plain text and other escape sequences untouched", () => {
    expect(expandExtendedEscapes("")).toBe("");
    expect(expandExtendedEscapes("plain text")).toBe("plain text");
    expect(expandExtendedEscapes("\\s\\d\\w\\b")).toBe("\\s\\d\\w\\b");
  });

  it("keeps the legacy sequential semantics without backslash un-escaping", () => {
    // 与既有内联实现一致：\\n（双反斜杠 + n）也会命中第一遍替换，
    // 展开成 反斜杠 + 换行，而不是保留字面量 \n
    expect(expandExtendedEscapes("\\\\n")).toBe("\\\n");
    // 顺序替换（\n → \t → \r）逐遍作用于上一遍的结果
    expect(expandExtendedEscapes("\\n\\t\\r")).toBe("\n\t\r");
  });

  it("applies the same expansion to replacement strings", () => {
    const search = expandExtendedEscapes("one\\ntwo");
    const replaced = replaceAllInContent("one\ntwo three", textQuery(search), expandExtendedEscapes("1\\t2"));
    expect(replaced).toEqual({ content: "1\t2 three", count: 1 });
  });
});

describe("buildFindRegex", () => {
  it("compiles text mode patterns as escaped literals with global flag", () => {
    const regex = buildFindRegex(textQuery("1+1=2?"));
    expect(regex.flags).toBe("g");
    expect("1+1=2? yes 1+1=2?".match(regex)).toHaveLength(2);
    expect(regex.source).toBe("1\\+1=2\\?");
  });

  it("keeps regex mode patterns as-is", () => {
    const regex = buildFindRegex(regexQuery("a+b?"));
    expect(regex.source).toBe("a+b?");
  });

  it("toggles case sensitivity via the i flag (default case-sensitive)", () => {
    expect(buildFindRegex(textQuery("abc")).flags).toBe("g");
    expect(buildFindRegex(textQuery("abc", { caseSensitive: false })).flags).toBe("gi");
    expect(countMatchesInContent("ABC abc Abc", textQuery("abc"))).toBe(1);
    expect(countMatchesInContent("ABC abc Abc", textQuery("abc", { caseSensitive: false }))).toBe(3);
  });

  it("wraps whole-word queries in word boundaries", () => {
    const query = textQuery("cat", { wholeWord: true });
    expect(countMatchesInContent("cat concatenate cat. scatter", query)).toBe(2);
    // regex 模式用非捕获组包裹，避免影响 $1 编号
    const regex = buildFindRegex(regexQuery("cat|dog", { wholeWord: true }));
    expect(regex.source).toBe("\\b(?:cat|dog)\\b");
    expect("hotdog dog".match(regex)).toEqual(["dog"]);
  });

  it("throws the native SyntaxError for invalid user regex", () => {
    expect(() => buildFindRegex(regexQuery("([unclosed"))).toThrow(SyntaxError);
    expect(() => findAllMatches("content", regexQuery("(?<"))).toThrow(SyntaxError);
    expect(() => replaceAllInContent("content", regexQuery("*bad"), "x")).toThrow(SyntaxError);
    // text 模式下同样的输入是合法字面量，不应抛错
    expect(countMatchesInContent("*bad ([unclosed", textQuery("*bad"))).toBe(1);
  });
});

describe("countMatchesInContent / findAllMatches", () => {
  it("counts literal occurrences without overlap", () => {
    expect(countMatchesInContent("aaa", textQuery("aa"))).toBe(1);
    expect(countMatchesInContent("ab ab ab", textQuery("ab"))).toBe(3);
    expect(countMatchesInContent("abc", textQuery(""))).toBe(0);
  });

  it("returns match offsets", () => {
    expect(findAllMatches("foo bar foo", textQuery("foo"))).toEqual([
      { from: 0, to: 3 },
      { from: 8, to: 11 },
    ]);
  });

  it("counts regex matches including zero-length ones without hanging", () => {
    expect(countMatchesInContent("a1b22c", regexQuery("\\d+"))).toBe(2);
    // 与 String.prototype.match(/a*/g) 的计数保持一致
    expect(countMatchesInContent("bb", regexQuery("a*"))).toBe("bb".match(/a*/g)?.length ?? 0);
  });

  it("handles multiline content", () => {
    const content = "line one\nline two\nline three";
    expect(countMatchesInContent(content, textQuery("line"))).toBe(3);
    expect(findAllMatches(content, regexQuery("one\\nline"))).toEqual([{ from: 5, to: 13 }]);
  });
});

describe("findNextMatch", () => {
  const content = "ab ab ab";

  it("finds the first match at or after start when searching down", () => {
    expect(findNextMatch(content, textQuery("ab"), 0, "down")).toEqual({ from: 0, to: 2 });
    expect(findNextMatch(content, textQuery("ab"), 1, "down")).toEqual({ from: 3, to: 5 });
    expect(findNextMatch(content, textQuery("ab"), 7, "down")).toBeNull();
  });

  it("finds the previous match when searching up", () => {
    expect(findNextMatch(content, textQuery("ab"), 5, "up")).toEqual({ from: 3, to: 5 });
    expect(findNextMatch(content, regexQuery("ab"), 5, "up")).toEqual({ from: 3, to: 5 });
    expect(findNextMatch(content, regexQuery("ab"), 0, "up")).toBeNull();
  });

  it("returns null for empty patterns", () => {
    expect(findNextMatch(content, textQuery(""), 0, "down")).toBeNull();
  });
});

describe("findNextMatchWrapped", () => {
  const content = "ab ab ab";

  it("finds down matches from start and wraps back to the first match", () => {
    expect(findNextMatchWrapped(content, textQuery("ab"), 0, "down", false)).toEqual({
      from: 0,
      to: 2,
    });
    expect(findNextMatchWrapped(content, textQuery("ab"), 7, "down", false)).toBeNull();
    expect(findNextMatchWrapped(content, textQuery("ab"), 7, "down", true)).toEqual({
      from: 0,
      to: 2,
    });
    expect(findNextMatchWrapped(content, regexQuery("ab"), 7, "down", true)).toEqual({
      from: 0,
      to: 2,
    });
  });

  it("searches up only among matches fully before start (to <= start)", () => {
    // 与 findNextMatch 的 up 边界（from < start / from <= start）不同：
    // 桌面 epub-editor 语义要求匹配整体位于 start 之前
    expect(findNextMatchWrapped(content, textQuery("ab"), 5, "up", false)).toEqual({
      from: 3,
      to: 5,
    });
    expect(findNextMatchWrapped(content, textQuery("ab"), 4, "up", false)).toEqual({
      from: 0,
      to: 2,
    });
    expect(findNextMatchWrapped(content, regexQuery("ab"), 4, "up", false)).toEqual({
      from: 0,
      to: 2,
    });
    expect(findNextMatchWrapped(content, textQuery("ab"), 1, "up", false)).toBeNull();
  });

  it("wraps up to the last match, except straddling text matches", () => {
    expect(findNextMatchWrapped(content, textQuery("ab"), 1, "up", true)).toEqual({
      from: 6,
      to: 8,
    });
    expect(findNextMatchWrapped(content, regexQuery("ab"), 1, "up", true)).toEqual({
      from: 6,
      to: 8,
    });
    // text 模式 up 回绕要求 from >= start：唯一匹配跨越 start 时不命中（沿用桌面实现）
    expect(findNextMatchWrapped("abc", textQuery("abc"), 1, "up", true)).toBeNull();
    // regex 模式无此限制
    expect(findNextMatchWrapped("abc", regexQuery("abc"), 1, "up", true)).toEqual({
      from: 0,
      to: 3,
    });
  });

  it("allows overlapping text matches when searching down", () => {
    // text 模式 down 用 indexOf，可命中与上一次匹配重叠的位置
    expect(findNextMatchWrapped("aaa", textQuery("aa"), 1, "down", false)).toEqual({
      from: 1,
      to: 3,
    });
  });

  it("returns null for empty patterns and throws on invalid regex", () => {
    expect(findNextMatchWrapped(content, textQuery(""), 0, "down", true)).toBeNull();
    expect(() => findNextMatchWrapped(content, regexQuery("([bad"), 0, "down", true)).toThrow(
      SyntaxError,
    );
  });
});

describe("replaceAllInContent", () => {
  it("replaces literal text and reports the count", () => {
    const result = replaceAllInContent("foo bar foo", textQuery("foo"), "baz");
    expect(result).toEqual({ content: "baz bar baz", count: 2 });
  });

  it("keeps $ literal in text mode replacements", () => {
    expect(replaceAllInContent("price", textQuery("price"), "$&100")).toEqual({
      content: "$&100",
      count: 1,
    });
    // 即使大小写开关迫使内部走正则路径，text 模式替换仍是字面量
    expect(
      replaceAllInContent("Price", textQuery("price", { caseSensitive: false }), "$1"),
    ).toEqual({ content: "$1", count: 1 });
  });

  it("supports capture group templates in regex mode", () => {
    const result = replaceAllInContent(
      "<h1>Title</h1>",
      regexQuery("<h1>(.*?)</h1>"),
      "<h2>$1</h2>",
    );
    expect(result).toEqual({ content: "<h2>Title</h2>", count: 1 });
  });

  it("replaces across multiple lines", () => {
    const result = replaceAllInContent("a\nb\na\nb", regexQuery("a\\nb"), "c");
    expect(result).toEqual({ content: "c\nc", count: 2 });
  });

  it("returns the content unchanged for empty patterns", () => {
    expect(replaceAllInContent("abc", textQuery(""), "x")).toEqual({ content: "abc", count: 0 });
  });
});

describe("filterFilesInScope", () => {
  type F = { path: string; html: boolean; editable: boolean };
  const make = (path: string, html: boolean, editable: boolean): F => ({ path, html, editable });
  const chapter = make("ch1.xhtml", true, true);
  const style = make("style.css", false, true);
  const image = make("cover.jpg", false, false);
  const source = {
    allFiles: [chapter, style, image],
    currentFile: style,
    isHtml: (file: F) => file.html,
    isEditable: (file: F) => file.editable,
  };

  it("returns only the current file for current/open scopes", () => {
    expect(filterFilesInScope("current", source)).toEqual([style]);
    expect(filterFilesInScope("open", source)).toEqual([style]);
    expect(filterFilesInScope("current", { ...source, currentFile: null })).toEqual([]);
  });

  it("filters html and all scopes down to editable files", () => {
    expect(filterFilesInScope("html", source)).toEqual([chapter]);
    expect(filterFilesInScope("all", source)).toEqual([chapter, style]);
  });

  it("uses editable multi-selected files, falling back to the current file", () => {
    expect(filterFilesInScope("selected", { ...source, selectedFiles: [chapter, image] })).toEqual([
      chapter,
    ]);
    expect(filterFilesInScope("selected", { ...source, selectedFiles: [image] })).toEqual([style]);
    expect(filterFilesInScope("selected", source)).toEqual([style]);
  });
});
