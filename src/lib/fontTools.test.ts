import { describe, expect, it } from "vitest";
import {
  accumulateCharUsage,
  buildFontFamilyAliases,
  collectFontFaceFamiliesByFontPath,
  collectSelectorsUsingFont,
  createFontPreviewFamily,
  fontFileStem,
  getBinaryMimeType,
  isFontFile,
  normalizeFontFamilyName,
  normalizeZipPath,
  parseFontFamilyList,
  resolveCssHrefToZipPath,
  resolveRelativePath,
  stripQueryAndHash,
  styleAttributeUsesFont,
  summarizeGlyphUsage,
} from "./fontTools";

describe("isFontFile", () => {
  it("matches font extensions case-insensitively", () => {
    expect(isFontFile("a.ttf")).toBe(true);
    expect(isFontFile("A.OTF")).toBe(true);
    expect(isFontFile("b.woff")).toBe(true);
    expect(isFontFile("b.WOFF2")).toBe(true);
    expect(isFontFile("legacy.eot")).toBe(true);
  });

  it("rejects non-font files and extension-in-the-middle names", () => {
    expect(isFontFile("cover.jpg")).toBe(false);
    expect(isFontFile("style.css")).toBe(false);
    expect(isFontFile("font.ttf.bak")).toBe(false);
  });
});

describe("fontFileStem", () => {
  it("strips only the last extension", () => {
    expect(fontFileStem("SourceHan.ttf")).toBe("SourceHan");
    expect(fontFileStem("My.Font.woff2")).toBe("My.Font");
    expect(fontFileStem("noext")).toBe("noext");
  });
});

describe("getBinaryMimeType", () => {
  it("maps font and image extensions", () => {
    expect(getBinaryMimeType("a.woff2")).toBe("font/woff2");
    expect(getBinaryMimeType("a.woff")).toBe("font/woff");
    expect(getBinaryMimeType("A.TTF")).toBe("font/ttf");
    expect(getBinaryMimeType("a.otf")).toBe("font/otf");
    expect(getBinaryMimeType("a.eot")).toBe("application/vnd.ms-fontobject");
    expect(getBinaryMimeType("a.png")).toBe("image/png");
    expect(getBinaryMimeType("a.svg")).toBe("image/svg+xml");
  });

  it("falls back to octet-stream for unknown extensions", () => {
    expect(getBinaryMimeType("a.jpg")).toBe("application/octet-stream");
    expect(getBinaryMimeType("a.bin")).toBe("application/octet-stream");
  });
});

describe("path helpers", () => {
  it("normalizeZipPath unifies separators and strips leading slashes", () => {
    expect(normalizeZipPath("\\OEBPS\\Fonts\\a.ttf")).toBe("OEBPS/Fonts/a.ttf");
    expect(normalizeZipPath("//a/b")).toBe("a/b");
    expect(normalizeZipPath("a/b")).toBe("a/b");
  });

  it("stripQueryAndHash removes query and hash", () => {
    expect(stripQueryAndHash("a.ttf?v=1#frag")).toBe("a.ttf");
    expect(stripQueryAndHash("a.ttf#frag?x")).toBe("a.ttf");
    expect(stripQueryAndHash("a.ttf")).toBe("a.ttf");
  });

  it("resolveRelativePath resolves . and .. against the base file's directory", () => {
    expect(resolveRelativePath("OEBPS/Styles/main.css", "../Fonts/a.ttf")).toBe(
      "OEBPS/Fonts/a.ttf",
    );
    expect(resolveRelativePath("OEBPS/Text/ch1.xhtml", "./../Styles/a.css")).toBe(
      "OEBPS/Styles/a.css",
    );
    expect(resolveRelativePath("root.opf", "a.css")).toBe("a.css");
  });

  it("resolveCssHrefToZipPath combines resolution, query stripping and normalization", () => {
    expect(resolveCssHrefToZipPath("OEBPS/Text/ch1.xhtml", "../Styles/a.css?v=2#top")).toBe(
      "OEBPS/Styles/a.css",
    );
  });
});

describe("normalizeFontFamilyName / parseFontFamilyList", () => {
  it("strips quotes, trims and lowercases", () => {
    expect(normalizeFontFamilyName('"Source Han Serif"')).toBe("source han serif");
    expect(normalizeFontFamilyName("'MyFont'")).toBe("myfont");
    expect(normalizeFontFamilyName("  Arial  ")).toBe("arial");
    // 与原实现一致：先去首尾引号再 trim，引号外有空白时引号保留
    expect(normalizeFontFamilyName(' "Quoted" ')).toBe('"quoted"');
  });

  it("splits comma lists while keeping commas inside quotes", () => {
    expect(parseFontFamilyList('"Foo, Bar",\'Baz\',serif')).toEqual([
      "foo, bar",
      "baz",
      "serif",
    ]);
  });

  it("filters out empty entries", () => {
    expect(parseFontFamilyList("a,,b")).toEqual(["a", "b"]);
    expect(parseFontFamilyList("")).toEqual([]);
  });
});

describe("collectSelectorsUsingFont", () => {
  const aliases = new Set(["myfont"]);

  it("collects selectors whose rules declare the target family", () => {
    const css = 'h1, .title { color: red; font-family: "MyFont", serif; }\np { font-family: serif; }';
    expect(collectSelectorsUsingFont(css, aliases)).toEqual(["h1", ".title"]);
  });

  it("skips at-rules and rules without font-family", () => {
    const css =
      '@font-face { font-family: "MyFont"; src: url(a.ttf); }\n.plain { color: blue; }';
    expect(collectSelectorsUsingFont(css, aliases)).toEqual([]);
  });

  it("matches font-family property case-insensitively", () => {
    expect(collectSelectorsUsingFont("div { FONT-FAMILY: MyFont; }", aliases)).toEqual(["div"]);
  });
});

describe("collectFontFaceFamiliesByFontPath", () => {
  const cssPath = "OEBPS/Styles/main.css";
  const fontPath = "OEBPS/Fonts/a.ttf";

  it("returns families whose src resolves to the target font", () => {
    const css =
      '@font-face { font-family: "Alias One"; src: url("../Fonts/a.ttf"); }\n' +
      "@font-face { font-family: Other; src: url(../Fonts/b.ttf); }";
    expect(collectFontFaceFamiliesByFontPath(cssPath, css, fontPath)).toEqual(["alias one"]);
  });

  it("ignores data: and http(s) urls but matches a later local url", () => {
    const css =
      "@font-face { font-family: Remote; src: url(https://x/a.ttf); }\n" +
      "@font-face { font-family: Mixed; src: url(data:,AAA), url('../Fonts/a.ttf?v=3'); }";
    expect(collectFontFaceFamiliesByFontPath(cssPath, css, fontPath)).toEqual(["mixed"]);
  });

  it("skips blocks missing src or font-family", () => {
    const css =
      "@font-face { src: url(../Fonts/a.ttf); }\n@font-face { font-family: NoSrc; }";
    expect(collectFontFaceFamiliesByFontPath(cssPath, css, fontPath)).toEqual([]);
  });
});

describe("buildFontFamilyAliases", () => {
  it("collects the main family, file stem and extra aliases (normalized)", () => {
    const aliases = buildFontFamilyAliases({
      fontFamily: '"Preview Family"',
      fontFileName: "SourceHan.ttf",
      extraAliases: ["Internal Name"],
    });
    expect(aliases).toEqual(new Set(["preview family", "sourcehan", "internal name"]));
  });

  it("adds families linked through @font-face declarations in css contents", () => {
    const aliases = buildFontFamilyAliases({
      fontFamily: "x",
      fontFileName: "a.ttf",
      cssContents: {
        "OEBPS/Styles/main.css":
          '@font-face { font-family: "CSS Alias"; src: url(../Fonts/a.ttf); }',
      },
      fontPath: "OEBPS/Fonts/a.ttf",
    });
    expect(aliases.has("css alias")).toBe(true);
    expect(aliases.has("x")).toBe(true);
    expect(aliases.has("a")).toBe(true);
  });
});

describe("styleAttributeUsesFont", () => {
  const aliases = new Set(["myfont"]);

  it("detects the target family in inline styles", () => {
    expect(styleAttributeUsesFont('color: red; font-family: "MyFont", serif', aliases)).toBe(true);
    expect(styleAttributeUsesFont("font-family: serif", aliases)).toBe(false);
  });

  it("returns false when no font-family declaration exists", () => {
    expect(styleAttributeUsesFont("color: red", aliases)).toBe(false);
    expect(styleAttributeUsesFont("", aliases)).toBe(false);
  });
});

describe("accumulateCharUsage", () => {
  it("counts non-whitespace characters and accumulates across calls", () => {
    const map = new Map<string, number>();
    accumulateCharUsage("ab a", map);
    accumulateCharUsage("春 春b", map);
    expect(map.get("a")).toBe(2);
    expect(map.get("b")).toBe(2);
    expect(map.get("春")).toBe(2);
    // 空白（含全角空格、换行）不计入
    accumulateCharUsage(" \n\t　", map);
    expect(map.size).toBe(3);
  });
});

describe("summarizeGlyphUsage", () => {
  it("sums usage counts and counts missing occurrences", () => {
    const usage = new Map([
      ["a", 3],
      ["b", 2],
      ["c", 1],
    ]);
    expect(summarizeGlyphUsage(usage, ["b", "c"])).toEqual({
      usedCount: 6,
      missingCount: 3,
      missingChars: ["b", "c"],
    });
  });

  it("dedupes missing chars and handles empty inputs", () => {
    expect(summarizeGlyphUsage(new Map(), [])).toEqual({
      usedCount: 0,
      missingCount: 0,
      missingChars: [],
    });
    const usage = new Map([["x", 1]]);
    // 后端缺字列表可包含未统计到的字符，展示列表保持去重后的原顺序
    expect(summarizeGlyphUsage(usage, ["y", "y", "z"])).toEqual({
      usedCount: 1,
      missingCount: 0,
      missingChars: ["y", "z"],
    });
  });
});

describe("createFontPreviewFamily", () => {
  it("generates unique names with the tepub-font prefix", () => {
    const a = createFontPreviewFamily();
    const b = createFontPreviewFamily();
    expect(a).toMatch(/^tepub-font-\d+-[a-z0-9]{1,6}$/);
    expect(a).not.toBe(b);
  });
});
