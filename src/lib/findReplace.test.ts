import { describe, expect, it } from "vitest";
import {
  buildFindRegex,
  countMatchesInContent,
  escapeRegExpLiteral,
  filterFilesInScope,
  findAllMatches,
  findNextMatch,
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
