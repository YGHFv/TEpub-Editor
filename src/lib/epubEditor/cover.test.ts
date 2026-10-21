// @vitest-environment happy-dom

import { describe, expect, it } from "vitest";
import {
  extractImageFromWrapper,
  resolveEpubCover,
} from "$lib/epubEditor/cover";
import type { WebEpubCoverHints, WebEpubFileEntry } from "$lib/webEpub";

function image(path: string, size = 1000): WebEpubFileEntry {
  return {
    path,
    name: path.split("/").pop() || path,
    size,
    kind: "image",
    mediaType: "image/jpeg",
    editable: false,
  };
}

function hints(partial: Partial<WebEpubCoverHints> = {}): WebEpubCoverHints {
  return { propertiesPath: "", metaPath: "", guidePath: "", ...partial };
}

const noText = async () => null;

describe("resolveEpubCover", () => {
  it("prefers the EPUB3 cover-image property", async () => {
    const cover = await resolveEpubCover({
      files: [
        image("OEBPS/Images/cover.jpg"),
        image("OEBPS/Images/author.jpg"),
      ],
      hints: hints({ propertiesPath: "OEBPS/Images/cover.jpg" }),
      readText: noText,
    });
    expect(cover).toEqual({
      path: "OEBPS/Images/cover.jpg",
      reason: "properties",
    });
  });

  it("falls back to EPUB2 <meta name=cover>", async () => {
    const cover = await resolveEpubCover({
      files: [image("OEBPS/Images/00001.jpg"), image("OEBPS/Images/ad.jpg")],
      hints: hints({ metaPath: "OEBPS/Images/00001.jpg" }),
      readText: noText,
    });
    expect(cover?.reason).toBe("meta");
    expect(cover?.path).toBe("OEBPS/Images/00001.jpg");
  });

  it("digs the image out of a guide cover.xhtml wrapper", async () => {
    const wrapper = `<?xml version="1.0"?><html xmlns="http://www.w3.org/1999/xhtml"><body><div><svg xmlns="http://www.w3.org/2000/svg" xmlns:xlink="http://www.w3.org/1999/xlink"><image width="1000" height="1333" xlink:href="../Images/00001.jpg"/></svg></div></body></html>`;
    const cover = await resolveEpubCover({
      files: [image("OEBPS/Images/00001.jpg"), image("OEBPS/Images/logo.png")],
      hints: hints({ guidePath: "OEBPS/Text/cover.xhtml" }),
      readText: async (path) =>
        path === "OEBPS/Text/cover.xhtml" ? wrapper : null,
    });
    expect(cover).toEqual({
      path: "OEBPS/Images/00001.jpg",
      reason: "guide",
    });
  });

  it("never picks a thumbnail over the real cover", async () => {
    const cover = await resolveEpubCover({
      files: [
        image("OEBPS/Images/cover-thumb.jpg", 20_000),
        image("OEBPS/Images/cover.jpg", 300_000),
      ],
      hints: hints(),
      readText: noText,
    });
    expect(cover?.path).toBe("OEBPS/Images/cover.jpg");
    expect(cover?.reason).toBe("name");
  });

  it("ignores logos and author portraits when nothing declares a cover", async () => {
    const cover = await resolveEpubCover({
      files: [
        image("OEBPS/Images/00001-author.jpg", 5_000),
        image("OEBPS/Images/00002-logo.png", 3_000),
        image("OEBPS/Images/00003-big.jpg", 900_000),
      ],
      hints: hints(),
      readText: noText,
    });
    expect(cover).toEqual({
      path: "OEBPS/Images/00003-big.jpg",
      reason: "largest",
    });
  });

  it("returns null when a book has no usable image", async () => {
    expect(
      await resolveEpubCover({
        files: [],
        hints: hints(),
        readText: noText,
      }),
    ).toBe(null);
  });

  it("ignores hints that point at missing files", async () => {
    const cover = await resolveEpubCover({
      files: [image("OEBPS/Images/b.jpg")],
      hints: hints({
        propertiesPath: "OEBPS/Images/missing.jpg",
        metaPath: "OEBPS/Images/also-missing.jpg",
      }),
      readText: noText,
    });
    expect(cover?.path).toBe("OEBPS/Images/b.jpg");
  });
});

describe("extractImageFromWrapper", () => {
  it("reads namespaced xlink:href on svg image", () => {
    const source = `<html xmlns="http://www.w3.org/1999/xhtml"><body><svg xmlns:xlink="http://www.w3.org/1999/xlink"><image xlink:href="../Images/c.jpg"/></svg></body></html>`;
    expect(extractImageFromWrapper(source, "OEBPS/Text/cover.xhtml")).toBe(
      "OEBPS/Images/c.jpg",
    );
  });

  it("reads a plain <img src>", () => {
    const source = `<html><body><img src="images/c.png"/></body></html>`;
    expect(extractImageFromWrapper(source, "OEBPS/Text/cover.xhtml")).toBe(
      "OEBPS/Text/images/c.png",
    );
  });

  it("falls back to <object data>", () => {
    const source = `<html><body><object data="../Images/c.jpg"></object></body></html>`;
    expect(extractImageFromWrapper(source, "OEBPS/Text/cover.xhtml")).toBe(
      "OEBPS/Images/c.jpg",
    );
  });

  it("returns empty for a broken document", () => {
    expect(extractImageFromWrapper("<html><body>", "a.xhtml")).toBe("");
  });
});
