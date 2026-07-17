// @vitest-environment happy-dom

import { existsSync, readFileSync } from "node:fs";
import JSZip from "jszip";
import { describe, expect, it } from "vitest";
import { loadWebEpub } from "$lib/webEpub";
import { processWebEpubFont, webEpubFontProcessTesting } from "$lib/webEpubFontProcess";
import { processWebEpubAdvanced, webEpubAdvancedTesting } from "$lib/webEpubAdvanced";

type FixtureOptions = { title?: string; chapters?: string[]; chapterBodies?: string[]; fontBytes?: Uint8Array };

async function makeFixture(options: FixtureOptions = {}) {
  const title = options.title || "测试图书";
  const chapters = options.chapters || ["汉语软件后台发型。", "第二章正文。"];
  const zip = new JSZip();
  zip.file("mimetype", "application/epub+zip", { compression: "STORE" });
  zip.file("META-INF/container.xml", `<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>`);
  zip.file("OEBPS/style.css", options.fontBytes
    ? '@font-face { font-family: "BookFont"; src: url("Fonts/book.ttf"); } body { line-height: 1.7; font-family: "BookFont"; }'
    : "body { line-height: 1.7; }");
  const manifestChapters: string[] = [];
  const spine: string[] = [];
  const nav: string[] = [];
  const ncx: string[] = [];
  chapters.forEach((text, index) => {
    const number = index + 1;
    const body = options.chapterBodies?.[index] || `<h1>第${number}章</h1><p>${text}</p>`;
    zip.file(`OEBPS/Text/chapter${number}.xhtml`, `<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><head><title>第${number}章</title></head><body>${body}</body></html>`);
    manifestChapters.push(`<item id="chapter${number}" href="Text/chapter${number}.xhtml" media-type="application/xhtml+xml"/>`);
    spine.push(`<itemref idref="chapter${number}"/>`);
    nav.push(`<li><a href="Text/chapter${number}.xhtml">第${number}章</a></li>`);
    ncx.push(`<navPoint id="n${number}" playOrder="${number}"><navLabel><text>第${number}章</text></navLabel><content src="Text/chapter${number}.xhtml"/></navPoint>`);
  });
  const fontManifest = options.fontBytes ? `<item id="font" href="Fonts/book.ttf" media-type="font/ttf"/>` : "";
  if (options.fontBytes) zip.file("OEBPS/Fonts/book.ttf", options.fontBytes);
  zip.file("OEBPS/nav.xhtml", `<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops"><body><nav epub:type="toc"><ol>${nav.join("")}</ol></nav></body></html>`);
  zip.file("OEBPS/toc.ncx", `<?xml version="1.0"?><ncx xmlns="http://www.daisy.org/z3986/2005/ncx/"><docTitle><text>${title}</text></docTitle><navMap>${ncx.join("")}</navMap></ncx>`);
  zip.file("OEBPS/content.opf", `<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="BookId"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="BookId">urn:uuid:11111111-1111-4111-8111-111111111111</dc:identifier><dc:title>${title}</dc:title><dc:language>zh-CN</dc:language></metadata><manifest><item id="css" href="style.css" media-type="text/css"/>${manifestChapters.join("")}<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/><item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>${fontManifest}</manifest><spine toc="ncx">${spine.join("")}</spine></package>`);
  const blob = await zip.generateAsync({ type: "blob", mimeType: "application/epub+zip" });
  return new File([blob], `${title}.epub`, { type: "application/epub+zip" });
}

async function firstZipEntryName(blob: Blob) {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  expect([...bytes.slice(0, 4)]).toEqual([0x50, 0x4b, 0x03, 0x04]);
  const nameLength = bytes[26] | (bytes[27] << 8);
  return new TextDecoder().decode(bytes.slice(30, 30 + nameLength));
}

describe("shared EPUB advanced processing", () => {
  it("extracts spine text to UTF-8 TXT", async () => {
    const file = await makeFixture();
    const result = await processWebEpubAdvanced([file], "epub-to-txt");
    expect(result.changedEntries).toBe(2);
    const text = await result.outputs[0].blob.text();
    expect(text).toContain("汉语软件后台发型");
    expect(text).toContain("第二章正文");
  });

  it("converts EPUB 3 package metadata to EPUB 2", async () => {
    const file = await makeFixture();
    const result = await processWebEpubAdvanced([file], "epub-version", { targetVersion: "2" });
    const zip = await JSZip.loadAsync(await result.outputs[0].blob.arrayBuffer());
    const opf = await zip.file("OEBPS/content.opf")!.async("text");
    expect(opf).toContain('version="2.0"');
    expect(opf).toMatch(/<spine[^>]+toc="ncx"/);
  });

  it("converts visible EPUB text from simplified to traditional Chinese", async () => {
    const file = await makeFixture({ chapters: ["汉语软件后台发型。"] });
    const result = await processWebEpubAdvanced([file], "epub-chinese", { chineseDirection: "s2t" });
    const zip = await JSZip.loadAsync(await result.outputs[0].blob.arrayBuffer());
    const chapter = await zip.file("OEBPS/Text/chapter1.xhtml")!.async("text");
    expect(chapter).toContain("漢語軟件後臺髮型");
  });

  it("removes matching short advertisement paragraphs without dropping body text", async () => {
    const file = await makeFixture({ chapterBodies: ["<h1>第一章</h1><p>正文保留。</p><p>求月票和推荐票！</p>"] });
    const result = await processWebEpubAdvanced([file], "epub-ad-clean");
    const zip = await JSZip.loadAsync(await result.outputs[0].blob.arrayBuffer());
    const chapter = await zip.file("OEBPS/Text/chapter1.xhtml")!.async("text");
    expect(chapter).toContain("正文保留");
    expect(chapter).not.toContain("求月票");
  });

  it("adds standard ruby phonetics without changing non-Chinese text", async () => {
    const file = await makeFixture({ chapters: ["中文 ABC"] });
    const result = await processWebEpubAdvanced([file], "epub-phonetic");
    const zip = await JSZip.loadAsync(await result.outputs[0].blob.arrayBuffer());
    const chapter = await zip.file("OEBPS/Text/chapter1.xhtml")!.async("text");
    expect(chapter).toMatch(/<ruby[^>]*>中<rt[^>]*>zhōng<\/rt><\/ruby>/);
    expect(chapter).toContain("ABC");
  });

  it("enhances standard EPUB footnotes with popup data while retaining the aside", async () => {
    const file = await makeFixture({ chapterBodies: ["<p>正文<a href=\"#fn1\">[1]</a></p><aside id=\"fn1\" epub:type=\"footnote\">脚注内容</aside>"] });
    const result = await processWebEpubAdvanced([file], "epub-footnote", { footnoteMode: "standard-to-popup" });
    const zip = await JSZip.loadAsync(await result.outputs[0].blob.arrayBuffer());
    const chapter = await zip.file("OEBPS/Text/chapter1.xhtml")!.async("text");
    expect(chapter).toContain('data-tepub-note="脚注内容"');
    expect(chapter).toContain('id="fn1"');
    expect(chapter).toContain("脚注内容");
  });

  it("merges EPUB resources and reading orders into a loadable package", async () => {
    const first = await makeFixture({ title: "第一本", chapters: ["甲一", "甲二"] });
    const second = await makeFixture({ title: "第二本", chapters: ["乙一"] });
    const result = await processWebEpubAdvanced([first, second], "epub-merge", { outputTitle: "测试合集" });
    const merged = await loadWebEpub(new File([result.outputs[0].blob], result.outputs[0].name));
    expect(merged.metadata.title).toBe("测试合集");
    expect(merged.spine).toHaveLength(3);
    expect(merged.zip.file("OEBPS/Books/book1/OEBPS/style.css")).toBeTruthy();
    expect(merged.zip.file("OEBPS/Books/book2/OEBPS/Text/chapter1.xhtml")).toBeTruthy();
    expect(await firstZipEntryName(result.outputs[0].blob)).toBe("mimetype");
  });

  it("splits a book into independently loadable EPUB outputs", async () => {
    const file = await makeFixture({ chapters: ["一", "二", "三"] });
    const result = await processWebEpubAdvanced([file], "epub-split", { splitEvery: 1 });
    expect(result.outputs).toHaveLength(3);
    for (const output of result.outputs) {
      const split = await loadWebEpub(new File([output.blob], output.name));
      expect(split.spine).toHaveLength(1);
    }
  });

  it("round-trips the TEpub image watermark payload with checksum", () => {
    const payload = webEpubAdvancedTesting.watermarkPayload("本地隐形水印 2026");
    const data = new Uint8ClampedArray(64 * 64 * 4).fill(254);
    for (let index = 3; index < data.length; index += 4) data[index] = 255;
    const image = { width: 64, height: 64, data } as ImageData;
    expect(webEpubAdvancedTesting.embedPayload(image, payload)).toBe(true);
    expect(webEpubAdvancedTesting.extractPayload(image)).toBe("本地隐形水印 2026");
    image.data[0] ^= 1;
    expect(webEpubAdvancedTesting.extractPayload(image)).toBeNull();
  });

  it("keeps code points referenced through numeric XHTML entities when subsetting fonts", () => {
    const codePoints = new Set<number>();
    webEpubFontProcessTesting.collectSubsetCodePoints("<p>&#x6C49;&#35821; ABC</p>", codePoints);
    expect(codePoints).toContain(0x6c49);
    expect(codePoints).toContain(35821);
  });

  it("does not reuse private code points already occupied by an embedded font", () => {
    const fonts = [{ glyphs: [{ unicode: [0x63a9] }, { unicode: [0xee01] }] }];
    const occupied = webEpubFontProcessTesting.collectFontPrivateCodePoints(fonts as never);
    const generated = webEpubFontProcessTesting.privateCharacters(4236, occupied)
      .map((character) => character.codePointAt(0));
    expect(occupied).toContain(0xee01);
    expect(generated).toHaveLength(4236);
    expect(generated).not.toContain(0xee01);
  });

  it("adds embedded font fallbacks before generic families without changing font-face names", () => {
    const css = '@font-face { font-family: "AS"; src: url("book.ttf"); } .title { font-family: "楷体", sans-serif; }';
    const rewritten = webEpubFontProcessTesting.rewriteCssFontFallbacks(css, ["AS", "Title"]);
    expect(rewritten).toContain('@font-face { font-family: "AS";');
    expect(rewritten).toContain('font-family: "楷体", "AS", "Title", sans-serif;');
  });

  it("injects line-breaking compatibility for private-use text", () => {
    const html = '<html><head><title>Book</title></head><body><p>text</p></body></html>';
    const rewritten = webEpubFontProcessTesting.injectObfuscatedTextCompatibility(html, ["AS"]);
    expect(rewritten).toContain('data-tepub-font-obfuscation="1"');
    expect(rewritten).toContain('word-break: break-all');
    expect(rewritten).toContain('font-family: "AS", serif');
  });

  it("randomly permutes normal CJK code points without fixed mappings", () => {
    const source = ["中", "文", "测", "试"];
    const shuffled = webEpubFontProcessTesting.randomDerangement(source);
    expect([...shuffled].sort()).toEqual([...source].sort());
    expect(shuffled.every((character, index) => character !== source[index])).toBe(true);
  });

  it("encrypts paragraph text only and leaves headings and punctuation unchanged", () => {
    const html = '<html><head></head><body><h1>标题</h1><p class="body">正文，测试。</p></body></html>';
    const mapping = new Map([["正", "测"], ["文", "试"], ["测", "正"], ["试", "文"]]);
    const rewritten = webEpubFontProcessTesting.transformBodyText(html, mapping, "encrypt");
    expect(rewritten).toContain("<h1>标题</h1>");
    expect(rewritten).toContain('class="body tepub-font-encrypted-body"');
    expect(rewritten).toContain("测试，正文。");
  });

  it("decrypts numbered body-font markers without touching following headings", () => {
    const html = '<body><div class="calibre tepub-font-encrypted-body-2"><span>密文</span></div><h2>密文标题</h2></body>';
    const mapping = new Map([["密", "正"], ["文", "文"]]);
    const rewritten = webEpubFontProcessTesting.transformBodyText(html, mapping, "decrypt");
    expect(rewritten).toContain('<div class="calibre"><span>正文</span></div>');
    expect(rewritten).toContain("<h2>密文标题</h2>");
    expect(rewritten).not.toContain("tepub-font-encrypted-body-2");
  });

  it("forces encrypted paragraphs to use only the selected body font", () => {
    const html = '<html><head></head><body><p>正文</p></body></html>';
    const rewritten = webEpubFontProcessTesting.injectBodyFontStyle(html, "OEBPS/Text/chapter.xhtml", "OEBPS/Fonts/body.ttf");
    expect(rewritten).toContain('data-tepub-body-font-encryption="1"');
    expect(rewritten).toContain('url("../Fonts/body.ttf")');
    expect(rewritten).not.toContain("word-break");
  });

  const windowsFont = "C:\\Windows\\Fonts\\Deng.ttf";
  const fontTest = existsSync(windowsFont) ? it : it.skip;
  fontTest("subsets a real embedded Chinese TTF", async () => {
    const bytes = new Uint8Array(readFileSync(windowsFont));
    const file = await makeFixture({ fontBytes: bytes, chapters: ["汉语测试 ABC 123"] });
    const result = await processWebEpubFont(file, "font-subset");
    expect(result.changedFonts).toBe(1);
    expect(result.blob.size).toBeLessThan(file.size);
  }, 60_000);

  fontTest("repairs missing font names and impossible global glyph bounds", async () => {
    const bytes = readFileSync(windowsFont);
    const { createFont } = await import("fonteditor-core");
    const font = createFont(bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength), { type: "ttf" });
    const data = font.get();
    data.name = {} as never;
    data.head.xMax = data.head.unitsPerEm * 40;
    data.head.yMax = data.head.unitsPerEm * 30;
    const parsed = { path: "OEBPS/Fonts/broken.ttf", type: "ttf", font, glyphs: data.glyf || [] };
    expect(webEpubFontProcessTesting.normalizeFontCompatibility(parsed as never)).toBe(true);
    const repaired = createFont(font.write({ type: "ttf", hinting: true, kerning: true }), { type: "ttf" }).get();
    expect(repaired.name.fontFamily).toBe("TEpub broken");
    expect(repaired.name.postScriptName).toBe("TEpubSubset-broken");
    expect(repaired.head.xMax).toBeLessThan(data.head.unitsPerEm * 4);
    expect(repaired.head.yMax).toBeLessThan(data.head.unitsPerEm * 4);
  }, 60_000);

  fontTest("rejects misleading partial encryption when the body font is not embedded", async () => {
    const bytes = new Uint8Array(readFileSync(windowsFont));
    const bodyText = "正文没有嵌入字体，因此装饰标题字体不能代替完整正文字体执行整书加密。".repeat(20);
    const base = await makeFixture({ fontBytes: bytes, chapterBodies: [`<h1>装饰标题</h1><p>${bodyText}</p>`] });
    const zip = await JSZip.loadAsync(await base.arrayBuffer());
    zip.file("OEBPS/style.css", '@font-face { font-family: "TitleFont"; src: url("Fonts/book.ttf"); } body, p { font-family: "MissingBodyFont", serif; } h1 { font-family: "TitleFont"; }');
    const input = new File([await zip.generateAsync({ type: "blob" })], "missing-body-font.epub", { type: "application/epub+zip" });
    await expect(processWebEpubFont(input, "font-encrypt")).rejects.toThrow("未嵌入的系统字体");
  }, 120_000);

  fontTest("subsets all fonts and independently encrypts multiple body fonts", async () => {
    const bytes = new Uint8Array(readFileSync(windowsFont));
    const base = await makeFixture({
      fontBytes: bytes,
      chapterBodies: ['<p class="font-a">第一套正文字体测试。</p><p class="font-b">第二套正文字体测试。</p>'],
    });
    const zip = await JSZip.loadAsync(await base.arrayBuffer());
    zip.file("OEBPS/Fonts/第二 正文.ttf", bytes);
    zip.file("OEBPS/style.css", '@font-face { font-family: "BodyA"; src: url("Fonts/book.ttf"); } @font-face { font-family: "BodyB"; src: url("Fonts/第二 正文.ttf"); } p.font-a { font-family: "BodyA"; } p.font-b { font-family: "BodyB"; }');
    const opf = await zip.file("OEBPS/content.opf")!.async("text");
    zip.file("OEBPS/content.opf", opf.replace("</manifest>", '<item id="font2" href="Fonts/第二 正文.ttf" media-type="font/ttf"/></manifest>'));
    const input = new File([await zip.generateAsync({ type: "blob" })], "two-body-fonts.epub", { type: "application/epub+zip" });
    const result = await processWebEpubFont(input, "font-encrypt");
    const output = await JSZip.loadAsync(await result.blob.arrayBuffer());
    const chapter = await output.file("OEBPS/Text/chapter1.xhtml")!.async("text");
    expect(result.changedFonts).toBe(2);
    expect(result.mode).toBe("independent-body-font-permutation");
    expect(chapter).toContain("tepub-font-encrypted-body-1");
    expect(chapter).toContain("tepub-font-encrypted-body-2");
    expect(chapter).toContain('data-tepub-body-font-encryption="1"');
    expect(chapter).toContain('href="../Styles/tepub-font-encryption.css"');
    const compatibilityCss = await output.file("OEBPS/Styles/tepub-font-encryption.css")!.async("text");
    expect(compatibilityCss).toContain('format("truetype")');
    expect(compatibilityCss).toContain("%E7%AC%AC%E4%BA%8C%20%E6%AD%A3%E6%96%87.ttf");
    const updatedOpf = await output.file("OEBPS/content.opf")!.async("text");
    expect(updatedOpf).toContain("tepub-font-encryption.css");
    const { createFont } = await import("fonteditor-core");
    const encryptedFont = createFont(await output.file("OEBPS/Fonts/第二 正文.ttf")!.async("arraybuffer"), { type: "ttf" }).get();
    expect(encryptedFont.name.fontFamily).toBe("TEpub Encrypted Font 2");
    expect(encryptedFont.name.postScriptName).toBe("TEpubEncryptedFont2");
  }, 120_000);

  fontTest("subsets each font from only the text rendered with that font", async () => {
    const bytes = new Uint8Array(readFileSync(windowsFont));
    const base = await makeFixture({ fontBytes: bytes, chapterBodies: ["<h1>标题</h1><p>正文独有汉字</p>"] });
    const zip = await JSZip.loadAsync(await base.arrayBuffer());
    zip.file("OEBPS/Fonts/title.ttf", bytes);
    zip.file("OEBPS/style.css", '@font-face { font-family: "BodyFont"; src: url("Fonts/book.ttf"); } @font-face { font-family: "TitleFont"; src: url("Fonts/title.ttf"); } body { font-family: "BodyFont"; } /* Chapter Title */ h1 { font-family: "TitleFont"; }');
    const opf = await zip.file("OEBPS/content.opf")!.async("text");
    zip.file("OEBPS/content.opf", opf.replace("</manifest>", '<item id="title-font" href="Fonts/title.ttf" media-type="font/ttf"/></manifest>'));
    const input = new File([await zip.generateAsync({ type: "blob" })], "font-aware-subset.epub", { type: "application/epub+zip" });
    const result = await processWebEpubFont(input, "font-subset");
    const output = await JSZip.loadAsync(await result.blob.arrayBuffer());
    const { createFont } = await import("fonteditor-core");
    const body = createFont(await output.file("OEBPS/Fonts/book.ttf")!.async("arraybuffer"), { type: "ttf" }).get().glyf || [];
    const title = createFont(await output.file("OEBPS/Fonts/title.ttf")!.async("arraybuffer"), { type: "ttf" }).get().glyf || [];
    const bodyCodes = new Set(body.flatMap((glyph) => glyph.unicode || []));
    const titleCodes = new Set(title.flatMap((glyph) => glyph.unicode || []));
    expect(bodyCodes).toContain("正".codePointAt(0));
    expect(titleCodes).toContain("标".codePointAt(0));
    expect(titleCodes).not.toContain("正".codePointAt(0));
  }, 120_000);

  fontTest("does not permute body-font characters that are also used outside paragraphs", async () => {
    const bytes = new Uint8Array(readFileSync(windowsFont));
    const base = await makeFixture({ fontBytes: bytes, chapterBodies: ['<h3><span class="num">第1章</span></h3><p>第一章正文测试。</p>'] });
    const zip = await JSZip.loadAsync(await base.arrayBuffer());
    zip.file("OEBPS/style.css", '@font-face { font-family: "BodyFont"; src: url("Fonts/book.ttf"); } body { font-family: "BodyFont"; } span.num { font-family: "BodyFont"; }');
    const input = new File([await zip.generateAsync({ type: "blob" })], "protected-heading-text.epub", { type: "application/epub+zip" });
    const result = await processWebEpubFont(input, "font-encrypt");
    const output = await JSZip.loadAsync(await result.blob.arrayBuffer());
    const chapter = await output.file("OEBPS/Text/chapter1.xhtml")!.async("text");
    expect(chapter).toContain('<span class="num">第1章</span>');
    expect(chapter).toContain('tepub-font-encrypted-body-1');
    expect(chapter).toMatch(/<p[^>]*>第/);
  }, 120_000);

  fontTest("treats title-like paragraphs as protected text even when they use the body font", async () => {
    const bytes = new Uint8Array(readFileSync(windowsFont));
    const base = await makeFixture({
      fontBytes: bytes,
      chapterBodies: ['<p class="te-chapter-title"><span class="te-chapter-number">第3章</span> 标题</p><p><span style="font-family: serif">标题</span>正文测试。</p>'],
    });
    const result = await processWebEpubFont(base, "font-encrypt");
    const output = await JSZip.loadAsync(await result.blob.arrayBuffer());
    const chapter = await output.file("OEBPS/Text/chapter1.xhtml")!.async("text");
    expect(chapter).toContain('<p class="te-chapter-title"><span class="te-chapter-number">第3章</span> 标题</p>');
    expect(chapter).not.toMatch(/te-chapter-title[^>]*tepub-font-encrypted-body/);
    expect(chapter).toContain("tepub-font-encrypted-body-1");
    expect(chapter).toContain("tepub-font-encrypted-body-1 *");
  }, 120_000);

  fontTest("includes heading characters selected by an embedded style block when subsetting", async () => {
    const bytes = new Uint8Array(readFileSync(windowsFont));
    const base = await makeFixture({ fontBytes: bytes, chapterBodies: ['<style>h1 { font-family: "TitleFont" !important; }</style><h1>嵌入标题</h1><p>正文</p>'] });
    const zip = await JSZip.loadAsync(await base.arrayBuffer());
    zip.file("OEBPS/Fonts/title.ttf", bytes);
    zip.file("OEBPS/style.css", '@font-face { font-family: "BodyFont"; src: url("Fonts/book.ttf"); } @font-face { font-family: "TitleFont"; src: url("Fonts/title.ttf"); } body { font-family: "BodyFont"; }');
    const opf = await zip.file("OEBPS/content.opf")!.async("text");
    zip.file("OEBPS/content.opf", opf.replace("</manifest>", '<item id="title-font-inline" href="Fonts/title.ttf" media-type="font/ttf"/></manifest>'));
    const input = new File([await zip.generateAsync({ type: "blob" })], "embedded-title-style.epub", { type: "application/epub+zip" });
    const result = await processWebEpubFont(input, "font-subset");
    const output = await JSZip.loadAsync(await result.blob.arrayBuffer());
    const { createFont } = await import("fonteditor-core");
    const title = createFont(await output.file("OEBPS/Fonts/title.ttf")!.async("arraybuffer"), { type: "ttf" }).get().glyf || [];
    const titleCodes = new Set(title.flatMap((glyph) => glyph.unicode || []));
    expect(titleCodes).toContain("嵌".codePointAt(0));
    expect(titleCodes).toContain("题".codePointAt(0));
  }, 120_000);

  fontTest("encrypts div-based body text in an EPUB that was already subsetted", async () => {
    const bytes = new Uint8Array(readFileSync(windowsFont));
    const paragraphText = "赵钱孙李周吴郑王冯陈褚卫蒋沈韩杨朱秦尤许何吕施张孔曹严华金魏陶姜戚谢邹喻柏水窦章云苏潘葛奚范彭郎鲁韦昌马苗凤花方俞任袁柳酆鲍史唐费廉岑薛雷贺倪汤滕殷罗毕郝邬安常乐于时傅皮卞齐康伍余元卜顾孟平黄".repeat(3);
    const originalBase = await makeFixture({
      fontBytes: bytes,
      chapterBodies: [`<h1>第一章</h1><div class="calibre1"><span>${paragraphText}</span></div>`],
    });
    const originalZip = await JSZip.loadAsync(await originalBase.arrayBuffer());
    const originalChapter = await originalZip.file("OEBPS/Text/chapter1.xhtml")!.async("text");
    originalZip.file("OEBPS/Text/chapter1.xhtml", originalChapter.replace("<body>", '<body class="te-chapter-page">'));
    const original = new File([await originalZip.generateAsync({ type: "blob" })], originalBase.name, { type: "application/epub+zip" });
    const subset = await processWebEpubFont(original, "font-subset");
    const subsetFile = new File([subset.blob], subset.outputName, { type: "application/epub+zip" });
    const encrypted = await processWebEpubFont(subsetFile, "font-encrypt");
    expect(encrypted.mappedCharacters).toBeGreaterThan(80);
    expect(encrypted.message).toContain("无需重复子集化");
    const encryptedZip = await JSZip.loadAsync(await encrypted.blob.arrayBuffer());
    const encryptedChapter = await encryptedZip.file("OEBPS/Text/chapter1.xhtml")!.async("text");
    expect(encryptedChapter).toContain("tepub-font-encrypted-body-1");
    expect(encryptedChapter).not.toContain(paragraphText);
  }, 120_000);
});
