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
  zip.file("OEBPS/style.css", "body { line-height: 1.7; }");
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

  const windowsFont = "C:\\Windows\\Fonts\\Deng.ttf";
  const fontTest = existsSync(windowsFont) ? it : it.skip;
  fontTest("subsets a real embedded Chinese TTF", async () => {
    const bytes = new Uint8Array(readFileSync(windowsFont));
    const file = await makeFixture({ fontBytes: bytes, chapters: ["汉语测试 ABC 123"] });
    const result = await processWebEpubFont(file, "font-subset");
    expect(result.changedFonts).toBe(1);
    expect(result.blob.size).toBeLessThan(file.size);
  }, 60_000);
});
