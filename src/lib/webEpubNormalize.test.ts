// @vitest-environment happy-dom

import JSZip from "jszip";
import { describe, expect, it } from "vitest";
import { loadWebEpub } from "$lib/webEpub";
import {
  clearZipEncryptionFlags,
  isObfuscatedEntryName,
  nameFromManifestId,
  normalizeEpubArchive,
  planEntryRenames,
  repairEpubContainerBytes,
  rewriteWebEpubTextLinks,
} from "$lib/webEpubNormalize";

const CONTAINER = `<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>`;

function opf(items: string, extra = "") {
  return `<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="BookId"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="BookId">test</dc:identifier><dc:title>测试书</dc:title><dc:language>zh-CN</dc:language>${extra}</metadata><manifest>${items}</manifest><spine><itemref idref="c1"/></spine></package>`;
}

async function makeObfuscatedBook() {
  // 书名混淆：文件名是 128 个 :* 字符，manifest id 仍然可读
  const obfChapter = `${"_" + "*:*::*".repeat(20)}.xhtml`;
  const obfCover = `${"_" + "::**::".repeat(15)}.jpg`;
  const zip = new JSZip();
  zip.file("mimetype", "application/epub+zip", { compression: "STORE" });
  zip.file("META-INF/container.xml", CONTAINER);
  zip.file(
    "OEBPS/content.opf",
    opf(
      `<item id="Chapter0001.xhtml" href="Text/${obfChapter}" media-type="application/xhtml+xml"/>` +
        `<item id="cover.jpg" href="Images/${obfCover}" media-type="image/jpeg" properties="cover-image"/>`,
    ),
  );
  zip.file(
    `OEBPS/Text/${obfChapter}`,
    `<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>第一章</title></head><body><p>正文</p></body></html>`,
  );
  zip.file(
    `OEBPS/Images/${obfCover}`,
    new Uint8Array([0xff, 0xd8, 0xff, 0xd9]),
  );
  return zip;
}

async function toFile(zip: JSZip, name = "book.epub") {
  const blob = await zip.generateAsync({
    type: "blob",
    mimeType: "application/epub+zip",
  });
  return new File([blob], name, { type: "application/epub+zip" });
}

/** 在中央目录与本地头里把加密位（bit0）置上，模拟伪加密书 */
function setEncryptionBits(bytes: Uint8Array) {
  const patched = bytes.slice();
  const view = new DataView(
    patched.buffer,
    patched.byteOffset,
    patched.byteLength,
  );
  for (let offset = 0; offset + 4 <= patched.length; offset += 1) {
    const signature = view.getUint32(offset, true);
    if (signature === 0x04034b50) {
      view.setUint16(offset + 6, view.getUint16(offset + 6, true) | 1, true);
    } else if (signature === 0x02014b50) {
      view.setUint16(offset + 8, view.getUint16(offset + 8, true) | 1, true);
    }
  }
  return patched;
}

describe("EPUB container repair", () => {
  it("clears pseudo-encryption flags without touching payload", async () => {
    const file = await toFile(await makeObfuscatedBook());
    const original = new Uint8Array(await file.arrayBuffer());
    const encrypted = setEncryptionBits(original);

    // 未修复：JSZip 直接拒收，这正是 web 版导入失败的原因
    await expect(JSZip.loadAsync(encrypted)).rejects.toThrow(/Encrypted zip/i);

    const repaired = repairEpubContainerBytes(encrypted);
    expect(repaired.repairedEncryption).toBe(true);
    const zip = await JSZip.loadAsync(repaired.bytes);
    expect(await zip.file("META-INF/container.xml")!.async("string")).toBe(
      CONTAINER,
    );
  });

  it("detects an XOR-scrambled container and recovers it", async () => {
    const file = await toFile(await makeObfuscatedBook());
    const original = new Uint8Array(await file.arrayBuffer());
    const scrambled = original.map((byte) => byte ^ 0xa5);

    expect(clearZipEncryptionFlags(scrambled)).toBe(false);
    const repaired = repairEpubContainerBytes(scrambled);
    expect(repaired.xorKey).toBe(0xa5);
    const zip = await JSZip.loadAsync(repaired.bytes);
    expect(zip.file("OEBPS/content.opf")).toBeTruthy();
  });

  it("leaves a normal archive untouched", async () => {
    const file = await toFile(await makeObfuscatedBook());
    const bytes = new Uint8Array(await file.arrayBuffer());
    const repaired = repairEpubContainerBytes(bytes);
    expect(repaired.xorKey).toBe(null);
    expect(repaired.repairedEncryption).toBe(false);
    expect(repaired.bytes).toEqual(bytes);
  });
});

describe("obfuscated entry names", () => {
  it("derives names from manifest ids the way the desktop build does", () => {
    // 与 Rust build_name_from_id 对齐：slim 后缀要保留，避免缩略图顶替正图
    expect(nameFromManifestId("cover.jpg", "Images/x.jpg")).toBe("cover.jpg");
    expect(nameFromManifestId("cover", "Images/x.jpg")).toBe("cover.jpg");
    expect(nameFromManifestId("cover", "Images/x")).toBe("cover.bin");
    // id 扩展名与 href 冲突时以 href 为准
    expect(nameFromManifestId("cover.png", "Images/x.jpg")).toBe("cover.jpg");
    expect(nameFromManifestId("cover_slim", "Images/x.jpg")).toBe(
      "cover~slim.jpg",
    );
    // 只有 href 名字里带 slim 时不截断 id，但仍标记为 slim
    expect(nameFromManifestId("cover.jpg", "Images/x-slim.jpg")).toBe(
      "cover~slim.jpg",
    );
    // 带目录的 id 只取文件名部分
    expect(nameFromManifestId("Text/Chapter0001.xhtml", "a/b.xhtml")).toBe(
      "Chapter0001.xhtml",
    );
    // Windows 非法字符落到下划线
    expect(nameFromManifestId("_*:*", "x.xhtml")).toBe("____.xhtml");
  });

  it("flags the `_*:*` basename format used by Chinese obfuscators", () => {
    expect(isObfuscatedEntryName("OEBPS/Text/_*:*::**:*.xhtml")).toBe(true);
    expect(isObfuscatedEntryName("OEBPS/Text/Chapter0001.xhtml")).toBe(false);
    expect(isObfuscatedEntryName("OEBPS/Text/第一章.xhtml")).toBe(false);
    // 超过 40 字符也按混淆处理（有些书用长 hash）
    expect(isObfuscatedEntryName(`OEBPS/Text/${"a".repeat(41)}.xhtml`)).toBe(
      true,
    );
  });

  it("renames from manifest ids and keeps suffixes unique", () => {
    const hints = new Map([
      ["OEBPS/Text/_*:*.xhtml", "Chapter0001.xhtml"],
      ["OEBPS/Images/_::::.jpg", "cover.jpg"],
    ]);
    const plan = planEntryRenames(
      ["OEBPS/Text/_*:*.xhtml", "OEBPS/Images/_::::.jpg", "mimetype"],
      hints,
    );
    expect(plan.get("OEBPS/Text/_*:*.xhtml")).toBe(
      "OEBPS/Text/Chapter0001.xhtml",
    );
    expect(plan.get("OEBPS/Images/_::::.jpg")).toBe("OEBPS/Images/cover.jpg");
    expect(plan.get("mimetype")).toBe("mimetype");
  });

  it("falls back to friendly names when the manifest has no hint", () => {
    const plan = planEntryRenames(
      ["OEBPS/Text/_*:*::*::*:::.xhtml"],
      new Map(),
    );
    expect(plan.get("OEBPS/Text/_*:*::*::*:::.xhtml")).toBe(
      "OEBPS/Text/chapter001.xhtml",
    );
  });

  it("rewrites references to renamed entries and keeps fragments", () => {
    const plan = new Map([
      ["OEBPS/Text/_*:*.xhtml", "OEBPS/Text/Chapter0001.xhtml"],
      ["OEBPS/Images/_::::.jpg", "OEBPS/Images/cover.jpg"],
    ]);
    const html = `<a href="_*:*.xhtml#p3">下一章</a><img src="../Images/_::::.jpg"/>`;
    const rewritten = rewriteWebEpubTextLinks(
      html,
      "OEBPS/Text/_*:*.xhtml",
      "OEBPS/Text/Chapter0001.xhtml",
      plan,
    );
    expect(rewritten).toContain(`href="Chapter0001.xhtml#p3"`);
    expect(rewritten).toContain(`src="../Images/cover.jpg"`);
    expect(rewritten).not.toContain("_::::");
  });

  it("does not touch unrelated quoted text", () => {
    const plan = new Map([["a.xhtml", "b.xhtml"]]);
    const html = `<p class="note">“这里不是路径”</p>`;
    expect(rewriteWebEpubTextLinks(html, "x.xhtml", "x.xhtml", plan)).toBe(
      html,
    );
  });
});

describe("loadWebEpub normalization", () => {
  it("recovers readable names and reports what it changed", async () => {
    const doc = await loadWebEpub(await toFile(await makeObfuscatedBook()));
    expect(doc.normalization.renamedEntries).toBeGreaterThan(0);
    const chapter = doc.files.find((file) => file.path.endsWith(".xhtml"));
    expect(chapter?.path).toBe("OEBPS/Text/Chapter0001.xhtml");
    expect(doc.coverHints.propertiesPath).toBe("OEBPS/Images/cover.jpg");
    // manifest 与文件表必须对得上，否则封面/预览会全部失联
    for (const item of doc.manifest) {
      expect(doc.files.some((file) => file.path === item.fullPath)).toBe(true);
    }
  });

  it("opens a pseudo-encrypted book instead of failing", async () => {
    const file = await toFile(await makeObfuscatedBook());
    const encrypted = setEncryptionBits(
      new Uint8Array(await file.arrayBuffer()),
    );
    const doc = await loadWebEpub(
      new File([encrypted], "enc.epub", { type: "application/epub+zip" }),
    );
    expect(doc.normalization.repairedEncryption).toBe(true);
    expect(doc.metadata.title).toBe("测试书");
    expect(doc.spine.length).toBe(1);
  });

  it("leaves a clean book byte-identical and unreported", async () => {
    const zip = new JSZip();
    zip.file("mimetype", "application/epub+zip", { compression: "STORE" });
    zip.file("META-INF/container.xml", CONTAINER);
    zip.file(
      "OEBPS/content.opf",
      opf(
        `<item id="c1" href="Text/chapter.xhtml" media-type="application/xhtml+xml"/>`,
      ),
    );
    zip.file("OEBPS/Text/chapter.xhtml", "<html><body>ok</body></html>");
    const doc = await loadWebEpub(await toFile(zip, "clean.epub"));
    expect(doc.normalization).toMatchObject({
      repairedEncryption: false,
      xorKey: null,
      renamedEntries: 0,
    });
    const normalized = await normalizeEpubArchive(
      await JSZip.loadAsync(await (await toFile(zip)).arrayBuffer()),
    );
    expect(normalized.changed).toBe(false);
  });
});
