// @vitest-environment happy-dom

import JSZip from "jszip";
import { describe, expect, it, vi } from "vitest";
import { EPUB_ILLUSTRATION_STYLES } from "$lib/epubStyleLibrary";
import { loadWebEpub } from "$lib/webEpub";
import {
  embedWebEpubRemoteImages,
  scanWebEpubRemoteImages,
  webEpubRemoteImagesTesting,
} from "$lib/webEpubRemoteImages";

const IMAGE_A = "https://e1.kuangxiangit.com/uploads/chapterimgsnew/502/130550/250428/1745814777-100432167-113288634.jpg";
const IMAGE_B = "https://aigcc.yuewen.com/imgChapter/19827814108189801/19919761301481004/10102981/3dc6bea40ba1d0b40add3f5d8d6f1948e08813Mwu4W97eq_hd.webp";

async function makeRemoteImageFixture() {
  const zip = new JSZip();
  zip.file("mimetype", "application/epub+zip", { compression: "STORE" });
  zip.file("META-INF/container.xml", `<?xml version="1.0"?><container xmlns="urn:oasis:names:tc:opendocument:xmlns:container" version="1.0"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>`);
  zip.file("OEBPS/Text/chapter1.xhtml", `<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>第一章</title></head><body><p class="te-paragraph"><img src="${IMAGE_A}" alt="山间旧照"/></p><p>正文中的重复图片 <img src="${IMAGE_A}" /></p></body></html>`);
  zip.file("OEBPS/Text/chapter2.xhtml", `<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><head><title>第二章</title></head><body><p class="te-paragraph">&lt;img src=&quot;${IMAGE_A}&quot;&gt;</p><p><img src="${IMAGE_B}"/></p></body></html>`);
  zip.file("OEBPS/content.opf", `<?xml version="1.0"?><package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="BookId"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="BookId">remote-image-test</dc:identifier><dc:title>在线插图测试</dc:title><dc:language>zh-CN</dc:language></metadata><manifest><item id="c1" href="Text/chapter1.xhtml" media-type="application/xhtml+xml"/><item id="c2" href="Text/chapter2.xhtml" media-type="application/xhtml+xml"/></manifest><spine><itemref idref="c1"/><itemref idref="c2"/></spine></package>`);
  const blob = await zip.generateAsync({ type: "blob", mimeType: "application/epub+zip" });
  return new File([blob], "remote.epub", { type: "application/epub+zip" });
}

describe("EPUB online image embedding", () => {
  it("scans real and escaped image tags and aggregates duplicate URLs", async () => {
    const doc = await loadWebEpub(await makeRemoteImageFixture());
    const references = await scanWebEpubRemoteImages(doc);

    expect(references).toHaveLength(2);
    expect(references.find((item) => item.url === IMAGE_A)).toMatchObject({
      occurrences: 3,
      encodedOccurrences: 1,
    });
    expect(references.find((item) => item.url === IMAGE_A)?.filePaths).toHaveLength(2);
  });

  it("downloads each selected URL once, embeds resources, and applies centered caption markup", async () => {
    const doc = await loadWebEpub(await makeRemoteImageFixture());
    const downloader = vi.fn(async (url: string) => ({
      bytes: url === IMAGE_A
        ? new Uint8Array([0xff, 0xd8, 0xff, 0xdb, 0x00, 0x01])
        : new Uint8Array([0x52, 0x49, 0x46, 0x46, 0x04, 0x00, 0x00, 0x00, 0x57, 0x45, 0x42, 0x50]),
      mediaType: url === IMAGE_A ? "image/jpeg" : "image/webp",
    }));
    const style = EPUB_ILLUSTRATION_STYLES.find((item) => item.id === "illustration-centered-caption")!;

    const result = await embedWebEpubRemoteImages(doc, [IMAGE_A, IMAGE_A, IMAGE_B], style, downloader);
    const output = await JSZip.loadAsync(await result.blob.arrayBuffer());
    const chapter1 = await output.file("OEBPS/Text/chapter1.xhtml")!.async("text");
    const chapter2 = await output.file("OEBPS/Text/chapter2.xhtml")!.async("text");
    const opf = await output.file("OEBPS/content.opf")!.async("text");
    const css = await output.file("OEBPS/Styles/tepub-online-illustrations.css")!.async("text");

    expect(downloader).toHaveBeenCalledTimes(2);
    expect(result).toMatchObject({ downloadedImages: 2, replacedOccurrences: 4, changedFiles: 2 });
    expect(output.file("OEBPS/Images/online/1745814777-100432167-113288634.jpg")).not.toBeNull();
    expect(Object.keys(output.files).some((path) => path.startsWith("OEBPS/Images/online/3dc6bea40ba1d0b40add3f5d8d6f1948") && path.endsWith(".webp"))).toBe(true);
    expect(opf).toContain("Images/online/1745814777-100432167-113288634.jpg");
    expect(opf).toContain("Styles/tepub-online-illustrations.css");
    expect(chapter1).not.toContain(IMAGE_A);
    expect(chapter2).not.toContain("&lt;img");
    expect(chapter1).toContain('class="te-illustration"');
    expect(chapter1).toContain('class="te-illustration-caption">山间旧照</figcaption>');
    expect(chapter2).toContain('href="../Styles/tepub-online-illustrations.css"');
    expect(chapter2).toContain('data-tepub-online-illustrations="1"');
    expect(css).toContain(".te-illustration-image");
    expect(chapter1).not.toContain("//>");
    expect(chapter2).not.toContain("//>");
  });

  it("keeps failed remote references while embedding successful images", async () => {
    const doc = await loadWebEpub(await makeRemoteImageFixture());
    const style = EPUB_ILLUSTRATION_STYLES[0];
    const result = await embedWebEpubRemoteImages(doc, [IMAGE_A, IMAGE_B], style, async (url) => {
      if (url === IMAGE_B) throw new Error("服务器拒绝访问");
      return { bytes: new Uint8Array([0xff, 0xd8, 0xff, 0xdb]), mediaType: "image/jpeg" };
    }, undefined, 0);
    const output = await JSZip.loadAsync(await result.blob.arrayBuffer());
    const chapter2 = await output.file("OEBPS/Text/chapter2.xhtml")!.async("text");

    expect(result.failedImages).toEqual([{ url: IMAGE_B, message: "服务器拒绝访问" }]);
    expect(chapter2).toContain(IMAGE_B);
    expect(chapter2).not.toContain(IMAGE_A);
  });

  it("waits and automatically retries one failed download", async () => {
    const doc = await loadWebEpub(await makeRemoteImageFixture());
    const downloader = vi.fn()
      .mockRejectedValueOnce(new Error("临时限流"))
      .mockResolvedValueOnce({
        bytes: new Uint8Array([0xff, 0xd8, 0xff, 0xdb]),
        mediaType: "image/jpeg",
      });
    const progress: string[] = [];

    const result = await embedWebEpubRemoteImages(
      doc,
      [IMAGE_A],
      EPUB_ILLUSTRATION_STYLES[0],
      downloader,
      (item) => progress.push(item.stage),
      0,
    );

    expect(downloader).toHaveBeenCalledTimes(2);
    expect(progress).toContain("retrying");
    expect(result.downloadedImages).toBe(1);
    expect(result.failedImages).toEqual([]);
  });

  it("returns failed items after both attempts so they can be retried manually", async () => {
    const doc = await loadWebEpub(await makeRemoteImageFixture());
    const downloader = vi.fn().mockRejectedValue(new Error("持续失败"));

    const result = await embedWebEpubRemoteImages(
      doc,
      [IMAGE_B],
      EPUB_ILLUSTRATION_STYLES[0],
      downloader,
      undefined,
      0,
    );

    expect(downloader).toHaveBeenCalledTimes(2);
    expect(result.downloadedImages).toBe(0);
    expect(result.failedImages).toEqual([{ url: IMAGE_B, message: "持续失败" }]);
  });

  it("normalizes image tags to one XHTML self-closing slash", () => {
    const normal = webEpubRemoteImagesTesting.localImageTag(`<img src="${IMAGE_A}">`, "../Images/a.jpg", "");
    const selfClosing = webEpubRemoteImagesTesting.localImageTag(`<img src="${IMAGE_A}"/>`, "../Images/a.jpg", "");
    expect(normal).toMatch(/<img[^>]+ \/>$/);
    expect(selfClosing).toMatch(/<img[^>]+ \/>$/);
    expect(normal).not.toContain("//>");
    expect(selfClosing).not.toContain("//>");
  });
});
