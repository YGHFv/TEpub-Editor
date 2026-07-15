import JSZip from "jszip";
import {
  addWebEpubResource,
  exportWebEpubBlob,
  guessWebEpubMediaType,
  loadWebEpub,
  normalizeZipPath,
  readWebEpubBlob,
  readWebEpubText,
  renameWebEpubResource,
  updateWebEpubBinary,
  updateWebEpubText,
  type WebEpubDocument,
} from "$lib/webEpub";
import { rewriteWebEpubTextLinks } from "$lib/webEpubProcess";

export type WebEpubAdvancedAction =
  | "epub-to-txt"
  | "epub-version"
  | "epub-chinese"
  | "epub-ad-clean"
  | "epub-phonetic"
  | "epub-footnote"
  | "image-compress"
  | "image-watermark"
  | "epub-merge"
  | "epub-split";

export type WebEpubAdvancedOptions = {
  targetVersion?: "2" | "3";
  chineseDirection?: "s2t" | "t2s";
  imageQuality?: number;
  maxImageDimension?: number;
  watermarkMode?: "embed" | "inspect";
  watermarkText?: string;
  outputTitle?: string;
  splitEvery?: number;
  adPatterns?: string;
  footnoteMode?: "standard-to-popup" | "popup-to-standard";
};

export type WebEpubAdvancedOutput = {
  name: string;
  blob: Blob;
  message: string;
};

export type WebEpubAdvancedResult = {
  sourceNames: string[];
  action: WebEpubAdvancedAction;
  changedEntries: number;
  message: string;
  outputs: WebEpubAdvancedOutput[];
  report?: string;
};

export type WebEpubChapterTarget = {
  index: number;
  title: string;
  path: string;
};

const TEXT_REFERENCE_EXTENSIONS = new Set(["xhtml", "html", "htm", "css", "xml", "ncx", "svg"]);
const RASTER_EXTENSIONS = new Set(["jpg", "jpeg", "png", "webp"]);
const WATERMARK_MAGIC = new TextEncoder().encode("TEPUBWM2");

function extension(path: string) {
  return path.split(".").pop()?.toLowerCase() || "";
}

function dirname(path: string) {
  const index = path.lastIndexOf("/");
  return index >= 0 ? path.slice(0, index + 1) : "";
}

function stem(fileName: string) {
  return fileName.replace(/\.epub$/i, "") || "book";
}

function sanitizeFileName(value: string) {
  return value.replace(/[\\/:*?"<>|]/g, "_").replace(/\s+/g, " ").trim().slice(0, 100) || "book";
}

function outputName(fileName: string, suffix: string, ext = "epub") {
  return `${sanitizeFileName(stem(fileName))}${suffix}.${ext}`;
}

function escapeXml(value: string) {
  return value.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&apos;");
}

function parseXml(source: string, label: string) {
  const document = new DOMParser().parseFromString(source, "application/xml");
  if (document.querySelector("parsererror")) throw new Error(`${label} XML 格式无效`);
  return document;
}

function elementsByLocalName(root: ParentNode, name: string) {
  return Array.from(root.querySelectorAll("*")).filter((node) => node.localName === name) as Element[];
}

function serializeXml(document: XMLDocument, original = "") {
  const serialized = new XMLSerializer().serializeToString(document);
  const declaration = original.match(/^\s*<\?xml[^>]*\?>/i)?.[0];
  return declaration && !serialized.trimStart().startsWith("<?xml") ? `${declaration}\n${serialized}` : serialized;
}

function pathLabelMap(doc: WebEpubDocument) {
  const labels = new Map<string, string>();
  const visit = (items: typeof doc.navItems) => {
    for (const item of items) {
      if (item.fullPath && !labels.has(item.fullPath)) labels.set(item.fullPath, item.label);
      if (item.children) visit(item.children);
    }
  };
  visit(doc.navItems);
  return labels;
}

async function xhtmlTitle(doc: WebEpubDocument, path: string) {
  try {
    const source = await readWebEpubText(doc, path);
    const xml = new DOMParser().parseFromString(source, "application/xml");
    const document = xml.querySelector("parsererror") ? new DOMParser().parseFromString(source, "text/html") : xml;
    return document.querySelector("h1,h2,h3,title")?.textContent?.replace(/\s+/g, " ").trim() || path.split("/").pop() || path;
  } catch {
    return path.split("/").pop() || path;
  }
}

export async function listWebEpubChapterTargets(file: File) {
  const doc = await loadWebEpub(file);
  const labels = pathLabelMap(doc);
  const targets: WebEpubChapterTarget[] = [];
  for (let index = 0; index < doc.spine.length; index += 1) {
    const path = doc.spine[index].manifest?.fullPath || "";
    if (!path) continue;
    targets.push({ index, path, title: labels.get(path) || await xhtmlTitle(doc, path) });
  }
  return targets;
}

function htmlToPlainText(source: string) {
  const xml = new DOMParser().parseFromString(source, "application/xml");
  const document = xml.querySelector("parsererror") ? new DOMParser().parseFromString(source, "text/html") : xml;
  document.querySelectorAll("script,style,noscript").forEach((node) => node.remove());
  document.querySelectorAll("br").forEach((node) => node.replaceWith("\n"));
  document.querySelectorAll("p,div,section,article,aside,li,h1,h2,h3,h4,h5,h6,blockquote,pre,tr").forEach((node) => node.append("\n"));
  return ((document as Document & { body?: HTMLElement }).body?.textContent || document.documentElement.textContent || "")
    .replace(/\u00a0/g, " ")
    .replace(/[ \t]+\n/g, "\n")
    .replace(/\n[ \t]+/g, "\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

async function epubToTxt(file: File): Promise<WebEpubAdvancedResult> {
  const doc = await loadWebEpub(file);
  const labels = pathLabelMap(doc);
  const sections: string[] = [];
  for (const spine of doc.spine) {
    const path = spine.manifest?.fullPath;
    if (!path || !doc.zip.file(path)) continue;
    const text = htmlToPlainText(await readWebEpubText(doc, path));
    if (!text) continue;
    const title = labels.get(path) || await xhtmlTitle(doc, path);
    sections.push(text.startsWith(title) ? text : `${title}\n\n${text}`);
  }
  if (!sections.length) throw new Error("EPUB 阅读顺序中没有可提取正文");
  const header = [`书名：${doc.metadata.title}`, doc.metadata.creator ? `作者：${doc.metadata.creator}` : "", ""].filter((line, index) => line || index === 2).join("\n");
  const blob = new Blob([`${header}${sections.join("\n\n")}\n`], { type: "text/plain;charset=utf-8" });
  return {
    sourceNames: [file.name], action: "epub-to-txt", changedEntries: sections.length,
    message: `已按阅读顺序提取 ${sections.length} 个正文文件`,
    outputs: [{ name: outputName(file.name, "", "txt"), blob, message: `${sections.length} 个章节` }],
  };
}

async function chapterDescriptors(doc: WebEpubDocument) {
  const labels = pathLabelMap(doc);
  const chapters: Array<{ id: string; href: string; title: string }> = [];
  for (let index = 0; index < doc.spine.length; index += 1) {
    const item = doc.spine[index].manifest;
    if (!item) continue;
    chapters.push({ id: item.id, href: item.href, title: labels.get(item.fullPath) || await xhtmlTitle(doc, item.fullPath) });
  }
  return chapters;
}

function navDocument(title: string, chapters: Array<{ href: string; title: string }>) {
  const items = chapters.map((chapter) => `      <li><a href="${escapeXml(chapter.href)}">${escapeXml(chapter.title)}</a></li>`).join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE html>\n<html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops" lang="zh-CN">\n<head><title>${escapeXml(title)}</title></head>\n<body><nav epub:type="toc" id="toc"><h1>${escapeXml(title)}</h1><ol>\n${items}\n</ol></nav></body>\n</html>`;
}

function ncxDocument(title: string, identifier: string, chapters: Array<{ href: string; title: string }>) {
  const points = chapters.map((chapter, index) => `    <navPoint id="navPoint-${index + 1}" playOrder="${index + 1}"><navLabel><text>${escapeXml(chapter.title)}</text></navLabel><content src="${escapeXml(chapter.href)}"/></navPoint>`).join("\n");
  return `<?xml version="1.0" encoding="UTF-8"?>\n<ncx xmlns="http://www.daisy.org/z3986/2005/ncx/" version="2005-1"><head><meta name="dtb:uid" content="${escapeXml(identifier)}"/></head><docTitle><text>${escapeXml(title)}</text></docTitle><navMap>\n${points}\n</navMap></ncx>`;
}

async function convertVersion(file: File, targetVersion: "2" | "3"): Promise<WebEpubAdvancedResult> {
  const doc = await loadWebEpub(file);
  const chapters = await chapterDescriptors(doc);
  if (!chapters.length) throw new Error("EPUB 没有可转换的 spine 章节");
  let changed = 0;
  if (targetVersion === "3" && !doc.manifest.some((item) => item.properties.split(/\s+/).includes("nav"))) {
    await addWebEpubResource(doc, { path: `${doc.opfDir}nav.xhtml`, content: navDocument(doc.metadata.title, chapters), mediaType: "application/xhtml+xml", properties: "nav" });
    changed += 1;
  }
  if (targetVersion === "2" && !doc.manifest.some((item) => item.mediaType === "application/x-dtbncx+xml")) {
    await addWebEpubResource(doc, { path: `${doc.opfDir}toc.ncx`, content: ncxDocument(doc.metadata.title, doc.metadata.identifier, chapters), mediaType: "application/x-dtbncx+xml" });
    changed += 1;
  }
  const source = await readWebEpubText(doc, doc.opfPath);
  const xml = parseXml(source, "OPF");
  const packageNode = elementsByLocalName(xml, "package")[0];
  if (!packageNode) throw new Error("OPF 缺少 package 节点");
  if (packageNode.getAttribute("version") !== `${targetVersion}.0`) changed += 1;
  packageNode.setAttribute("version", `${targetVersion}.0`);
  const spine = elementsByLocalName(xml, "spine")[0];
  if (targetVersion === "2") {
    const ncx = elementsByLocalName(xml, "item").find((item) => item.getAttribute("media-type") === "application/x-dtbncx+xml");
    if (spine && ncx?.getAttribute("id")) spine.setAttribute("toc", ncx.getAttribute("id")!);
    for (const item of elementsByLocalName(xml, "item")) {
      const properties = (item.getAttribute("properties") || "").split(/\s+/).filter((value) => value && value !== "nav");
      if (properties.length) item.setAttribute("properties", properties.join(" ")); else item.removeAttribute("properties");
    }
  }
  updateWebEpubText(doc, doc.opfPath, serializeXml(xml, source));
  const blob = await exportWebEpubBlob(doc);
  return {
    sourceNames: [file.name], action: "epub-version", changedEntries: changed,
    message: `已转换为 EPUB ${targetVersion}.0，并补齐 ${targetVersion === "3" ? "NAV" : "NCX"} 导航`,
    outputs: [{ name: outputName(file.name, `_epub${targetVersion}`), blob, message: `EPUB ${targetVersion}.0` }],
  };
}

async function chineseConverter(direction: "s2t" | "t2s") {
  const { default: OpenCC } = await import("opencc-js");
  return direction === "s2t" ? OpenCC.Converter({ from: "cn", to: "tw" }) : OpenCC.Converter({ from: "hk", to: "cn" });
}

function transformDocumentText(source: string, label: string, convert: (value: string) => string) {
  const xml = parseXml(source, label);
  const walker = xml.createTreeWalker(xml, NodeFilter.SHOW_TEXT);
  let changed = false;
  let node = walker.nextNode();
  while (node) {
    const parent = node.parentElement?.localName.toLowerCase() || "";
    if (parent !== "script" && parent !== "style") {
      const next = convert(node.nodeValue || "");
      if (next !== node.nodeValue) { node.nodeValue = next; changed = true; }
    }
    node = walker.nextNode();
  }
  return changed ? serializeXml(xml, source) : source;
}

async function convertChinese(file: File, direction: "s2t" | "t2s"): Promise<WebEpubAdvancedResult> {
  const doc = await loadWebEpub(file);
  const convert = await chineseConverter(direction);
  let changed = 0;
  const paths = new Set([doc.opfPath, ...doc.manifest.filter((item) => /(?:xhtml|html|xml|ncx)\b/i.test(item.mediaType) || ["xhtml", "html", "htm", "xml", "ncx"].includes(extension(item.fullPath))).map((item) => item.fullPath)]);
  for (const path of paths) {
    if (!doc.zip.file(path)) continue;
    const source = await readWebEpubText(doc, path);
    try {
      const transformed = transformDocumentText(source, path, convert);
      if (transformed !== source) { updateWebEpubText(doc, path, transformed); changed += 1; }
    } catch {
      const transformed = source.replace(/>([^<]+)</g, (full, text: string) => `>${convert(text)}<`);
      if (transformed !== source) { updateWebEpubText(doc, path, transformed); changed += 1; }
    }
  }
  const blob = await exportWebEpubBlob(doc);
  const label = direction === "s2t" ? "简体转繁体" : "繁体转简体";
  return {
    sourceNames: [file.name], action: "epub-chinese", changedEntries: changed, message: `${label}完成：更新 ${changed} 个内容文件`,
    outputs: [{ name: outputName(file.name, direction === "s2t" ? "_traditional" : "_simplified"), blob, message: label }],
  };
}

const DEFAULT_AD_PATTERNS = [
  "^\\s*(?:PS|P\\.S\\.)[：:].*$",
  "^\\s*(?:求|跪求|感谢).{0,12}(?:月票|推荐票|订阅|收藏).*$",
  "^\\s*(?:本章未完|未完待续|最新网址|请记住本站).*$",
  "^\\s*(?:手机用户请|请使用搜索引擎搜索).*$",
];

function compileAdPatterns(source = "") {
  const lines = source.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  return (lines.length ? lines : DEFAULT_AD_PATTERNS).map((pattern) => new RegExp(pattern, "i"));
}

async function cleanEpubAds(file: File, sourcePatterns = ""): Promise<WebEpubAdvancedResult> {
  const doc = await loadWebEpub(file);
  const patterns = compileAdPatterns(sourcePatterns);
  let removed = 0;
  let changedFiles = 0;
  for (const item of doc.manifest.filter((entry) => entry.mediaType === "application/xhtml+xml" || ["xhtml", "html", "htm"].includes(extension(entry.fullPath)))) {
    if (!doc.zip.file(item.fullPath)) continue;
    const source = await readWebEpubText(doc, item.fullPath);
    let xml: XMLDocument;
    try { xml = parseXml(source, item.fullPath); } catch { continue; }
    const removedBefore = removed;
    for (const element of Array.from(xml.querySelectorAll("p,div,section,aside"))) {
      if (element.children.length > 2) continue;
      const text = element.textContent?.replace(/\s+/g, " ").trim() || "";
      if (text && text.length <= 240 && patterns.some((pattern) => pattern.test(text))) {
        element.remove();
        removed += 1;
      }
    }
    if (removed > removedBefore) { updateWebEpubText(doc, item.fullPath, serializeXml(xml, source)); changedFiles += 1; }
  }
  const blob = await exportWebEpubBlob(doc);
  return {
    sourceNames: [file.name], action: "epub-ad-clean", changedEntries: removed, message: `广告清理完成：从 ${changedFiles} 个正文文件移除 ${removed} 个段落`,
    outputs: [{ name: outputName(file.name, "_clean"), blob, message: `${removed} 个广告段落` }],
  };
}

function isCjkCharacter(character: string) {
  const code = character.codePointAt(0) || 0;
  return (code >= 0x3400 && code <= 0x9fff) || (code >= 0xf900 && code <= 0xfaff) || (code >= 0x20000 && code <= 0x3ffff);
}

async function addEpubPhonetics(file: File): Promise<WebEpubAdvancedResult> {
  const { pinyin } = await import("pinyin-pro");
  const doc = await loadWebEpub(file);
  let annotated = 0;
  let changedFiles = 0;
  const xhtmlNamespace = "http://www.w3.org/1999/xhtml";
  for (const item of doc.manifest.filter((entry) => entry.mediaType === "application/xhtml+xml" || ["xhtml", "html", "htm"].includes(extension(entry.fullPath)))) {
    if (!doc.zip.file(item.fullPath)) continue;
    const source = await readWebEpubText(doc, item.fullPath);
    let xml: XMLDocument;
    try { xml = parseXml(source, item.fullPath); } catch { continue; }
    const annotatedBefore = annotated;
    const walker = xml.createTreeWalker(xml, NodeFilter.SHOW_TEXT);
    const nodes: Text[] = [];
    let current = walker.nextNode();
    while (current) { nodes.push(current as Text); current = walker.nextNode(); }
    for (const node of nodes) {
      const parentName = node.parentElement?.localName.toLowerCase() || "";
      if (["script", "style", "ruby", "rt", "rp", "code", "pre"].includes(parentName) || ![...node.data].some(isCjkCharacter)) continue;
      const fragment = xml.createDocumentFragment();
      for (const character of node.data) {
        if (!isCjkCharacter(character)) { fragment.appendChild(xml.createTextNode(character)); continue; }
        const reading = String(pinyin(character, { toneType: "symbol", type: "array" })[0] || "").trim();
        if (!reading || reading === character) { fragment.appendChild(xml.createTextNode(character)); continue; }
        const ruby = xml.createElementNS(xhtmlNamespace, "ruby");
        ruby.appendChild(xml.createTextNode(character));
        const rt = xml.createElementNS(xhtmlNamespace, "rt");
        rt.textContent = reading;
        ruby.appendChild(rt);
        fragment.appendChild(ruby);
        annotated += 1;
      }
      node.parentNode?.replaceChild(fragment, node);
    }
    if (annotated > annotatedBefore) { updateWebEpubText(doc, item.fullPath, serializeXml(xml, source)); changedFiles += 1; }
  }
  if (!annotated) throw new Error("正文中没有可标注拼音的汉字");
  const blob = await exportWebEpubBlob(doc);
  return {
    sourceNames: [file.name], action: "epub-phonetic", changedEntries: annotated, message: `拼音标注完成：在 ${changedFiles} 个正文文件标注 ${annotated} 个汉字`,
    outputs: [{ name: outputName(file.name, "_pinyin"), blob, message: `${annotated} 个 ruby 标注` }],
  };
}

function cssEscapeId(value: string) {
  return typeof CSS !== "undefined" && CSS.escape ? CSS.escape(value) : value.replace(/[^A-Za-z0-9_-]/g, "\\$&");
}

async function convertEpubFootnotes(file: File, mode: "standard-to-popup" | "popup-to-standard"): Promise<WebEpubAdvancedResult> {
  const doc = await loadWebEpub(file);
  let changedEntries = 0;
  let changedFiles = 0;
  const xhtmlNamespace = "http://www.w3.org/1999/xhtml";
  const epubNamespace = "http://www.idpf.org/2007/ops";
  for (const item of doc.manifest.filter((entry) => entry.mediaType === "application/xhtml+xml" || ["xhtml", "html", "htm"].includes(extension(entry.fullPath)))) {
    if (!doc.zip.file(item.fullPath)) continue;
    const source = await readWebEpubText(doc, item.fullPath);
    let xml: XMLDocument;
    try { xml = parseXml(source, item.fullPath); } catch { continue; }
    const changedBefore = changedEntries;
    if (mode === "standard-to-popup") {
      for (const anchor of Array.from(xml.querySelectorAll("a[href^='#']"))) {
        const id = decodeURIComponent((anchor.getAttribute("href") || "").slice(1));
        const target = id ? xml.querySelector(`#${cssEscapeId(id)}`) : null;
        if (!target || (target.localName !== "aside" && !/footnote|note/i.test(target.getAttribute("class") || ""))) continue;
        const note = target.textContent?.replace(/\s+/g, " ").trim() || "";
        if (!note) continue;
        anchor.setAttributeNS(epubNamespace, "epub:type", "noteref");
        anchor.setAttribute("data-tepub-note", note);
        anchor.setAttribute("class", [...new Set(`${anchor.getAttribute("class") || ""} tepub-popup-note`.split(/\s+/).filter(Boolean))].join(" "));
        target.setAttributeNS(epubNamespace, "epub:type", "footnote");
        changedEntries += 1;
      }
    } else {
      const body = elementsByLocalName(xml, "body")[0] || xml.documentElement;
      let index = 0;
      for (const span of Array.from(xml.querySelectorAll("span[data-note],span[data-comment],span[zy-footnote],span.tepub-popup-note"))) {
        const note = span.getAttribute("data-note") || span.getAttribute("data-comment") || span.getAttribute("zy-footnote") || span.getAttribute("data-tepub-note") || "";
        if (!note.trim()) continue;
        const id = `tepub-footnote-${++index}`;
        const anchor = xml.createElementNS(xhtmlNamespace, "a");
        anchor.setAttribute("href", `#${id}`);
        anchor.setAttributeNS(epubNamespace, "epub:type", "noteref");
        anchor.textContent = span.textContent || "注";
        span.parentNode?.replaceChild(anchor, span);
        const aside = xml.createElementNS(xhtmlNamespace, "aside");
        aside.setAttribute("id", id);
        aside.setAttributeNS(epubNamespace, "epub:type", "footnote");
        aside.textContent = note;
        body.appendChild(aside);
        changedEntries += 1;
      }
    }
    if (changedEntries > changedBefore) { updateWebEpubText(doc, item.fullPath, serializeXml(xml, source)); changedFiles += 1; }
  }
  if (!changedEntries) throw new Error(mode === "standard-to-popup" ? "未找到可增强的标准脚注" : "未找到可转换的弹窗批注");
  const blob = await exportWebEpubBlob(doc);
  const label = mode === "standard-to-popup" ? "标准脚注弹窗增强" : "弹窗批注转 EPUB3 脚注";
  return {
    sourceNames: [file.name], action: "epub-footnote", changedEntries, message: `${label}完成：更新 ${changedEntries} 个注释`,
    outputs: [{ name: outputName(file.name, "_footnotes"), blob, message: `${changedEntries} 个注释` }],
  };
}

async function canvasEncode(blob: Blob, mediaType: string, quality: number, maxDimension: number) {
  const bitmap = await createImageBitmap(blob);
  const scale = maxDimension > 0 ? Math.min(1, maxDimension / Math.max(bitmap.width, bitmap.height)) : 1;
  const width = Math.max(1, Math.round(bitmap.width * scale));
  const height = Math.max(1, Math.round(bitmap.height * scale));
  const canvas = document.createElement("canvas");
  canvas.width = width;
  canvas.height = height;
  const context = canvas.getContext("2d", { willReadFrequently: true });
  if (!context) throw new Error("浏览器无法创建图片画布");
  context.drawImage(bitmap, 0, 0, width, height);
  bitmap.close();
  const outputType = mediaType === "image/jpeg" ? "image/jpeg" : mediaType === "image/webp" ? "image/webp" : "image/png";
  const output = await new Promise<Blob>((resolve, reject) => canvas.toBlob((value) => value ? resolve(value) : reject(new Error("图片编码失败")), outputType, quality));
  return { output, width, height, canvas, context };
}

async function compressImages(file: File, options: WebEpubAdvancedOptions): Promise<WebEpubAdvancedResult> {
  const doc = await loadWebEpub(file);
  const quality = Math.max(0.35, Math.min(0.95, options.imageQuality ?? 0.78));
  const maxDimension = Math.max(0, Math.floor(options.maxImageDimension ?? 2400));
  let changed = 0;
  let savedBytes = 0;
  for (const entry of doc.files.filter((item) => item.kind === "image" && RASTER_EXTENSIONS.has(extension(item.path)))) {
    try {
      const original = await readWebEpubBlob(doc, entry.path, entry.mediaType || guessWebEpubMediaType(entry.path));
      const { output } = await canvasEncode(original, entry.mediaType || guessWebEpubMediaType(entry.path), quality, maxDimension);
      if (output.size >= original.size) continue;
      await updateWebEpubBinary(doc, entry.path, output, entry.mediaType || output.type);
      changed += 1;
      savedBytes += original.size - output.size;
    } catch {
      // Unsupported or malformed raster images remain untouched.
    }
  }
  const blob = await exportWebEpubBlob(doc);
  return {
    sourceNames: [file.name], action: "image-compress", changedEntries: changed,
    message: `压缩 ${changed} 张图片，图片数据减少 ${(savedBytes / 1024 / 1024).toFixed(2)} MB`,
    outputs: [{ name: outputName(file.name, "_compressed"), blob, message: `${changed} 张图片` }],
  };
}

function checksum(bytes: Uint8Array) {
  let value = 0x811c9dc5;
  for (const byte of bytes) { value ^= byte; value = Math.imul(value, 0x01000193) >>> 0; }
  return value >>> 0;
}

function uint32Bytes(value: number) {
  return new Uint8Array([(value >>> 24) & 255, (value >>> 16) & 255, (value >>> 8) & 255, value & 255]);
}

function readUint32(bytes: Uint8Array, offset: number) {
  return (((bytes[offset] << 24) >>> 0) | (bytes[offset + 1] << 16) | (bytes[offset + 2] << 8) | bytes[offset + 3]) >>> 0;
}

function watermarkPayload(text: string) {
  const data = new TextEncoder().encode(text);
  const payload = new Uint8Array(WATERMARK_MAGIC.length + 8 + data.length);
  payload.set(WATERMARK_MAGIC, 0);
  payload.set(uint32Bytes(data.length), WATERMARK_MAGIC.length);
  payload.set(uint32Bytes(checksum(data)), WATERMARK_MAGIC.length + 4);
  payload.set(data, WATERMARK_MAGIC.length + 8);
  return payload;
}

function imageDataCapacity(image: ImageData) {
  return Math.floor((image.width * image.height * 3) / 8);
}

function embedPayload(image: ImageData, payload: Uint8Array) {
  if (imageDataCapacity(image) < payload.length) return false;
  let bitIndex = 0;
  for (let pixel = 0; pixel < image.width * image.height && bitIndex < payload.length * 8; pixel += 1) {
    for (let channel = 0; channel < 3 && bitIndex < payload.length * 8; channel += 1) {
      const byte = payload[Math.floor(bitIndex / 8)];
      const bit = (byte >>> (7 - (bitIndex % 8))) & 1;
      const offset = pixel * 4 + channel;
      image.data[offset] = (image.data[offset] & 0xfe) | bit;
      bitIndex += 1;
    }
  }
  return true;
}

function extractPayload(image: ImageData) {
  let bitIndex = 0;
  const read = (count: number) => {
    const output = new Uint8Array(count);
    for (let index = 0; index < count; index += 1) {
      let value = 0;
      for (let bit = 0; bit < 8; bit += 1) {
        const pixel = Math.floor(bitIndex / 3);
        const channel = bitIndex % 3;
        if (pixel >= image.width * image.height) return null;
        value = (value << 1) | (image.data[pixel * 4 + channel] & 1);
        bitIndex += 1;
      }
      output[index] = value;
    }
    return output;
  };
  const header = read(WATERMARK_MAGIC.length + 8);
  if (!header || !WATERMARK_MAGIC.every((byte, index) => header[index] === byte)) return null;
  const length = readUint32(header, WATERMARK_MAGIC.length);
  const expected = readUint32(header, WATERMARK_MAGIC.length + 4);
  if (length > imageDataCapacity(image) - header.length) return null;
  const data = read(length);
  if (!data || checksum(data) !== expected) return null;
  return new TextDecoder().decode(data);
}

async function rewriteResourceReferences(doc: WebEpubDocument, oldPath: string, newPath: string) {
  const pathMap = new Map([[oldPath, newPath]]);
  for (const entry of doc.files.filter((item) => item.path !== doc.opfPath && TEXT_REFERENCE_EXTENSIONS.has(extension(item.path)))) {
    const source = await readWebEpubText(doc, entry.path);
    const transformed = rewriteWebEpubTextLinks(source, entry.path, entry.path, pathMap);
    if (transformed !== source) updateWebEpubText(doc, entry.path, transformed);
  }
}

function uniquePngPath(doc: WebEpubDocument, oldPath: string) {
  const base = oldPath.replace(/\.[^.]+$/, "");
  let target = `${base}.png`;
  let index = 2;
  while (target !== oldPath && doc.zip.file(target)) target = `${base}-${index++}.png`;
  return target;
}

async function watermarkImages(file: File, options: WebEpubAdvancedOptions): Promise<WebEpubAdvancedResult> {
  const doc = await loadWebEpub(file);
  const mode = options.watermarkMode || "embed";
  const reports: string[] = [];
  let changed = 0;
  if (mode === "inspect") {
    for (const entry of doc.files.filter((item) => item.kind === "image" && RASTER_EXTENSIONS.has(extension(item.path)))) {
      try {
        const blob = await readWebEpubBlob(doc, entry.path, entry.mediaType);
        const { canvas, context } = await canvasEncode(blob, "image/png", 1, 0);
        const text = extractPayload(context.getImageData(0, 0, canvas.width, canvas.height));
        if (text !== null) reports.push(`${entry.path}: ${text}`);
      } catch {
        // Ignore images the browser cannot decode.
      }
    }
    const report = reports.length ? reports.join("\n") : "未发现由 TEpub Editor 写入的图片水印。";
    return { sourceNames: [file.name], action: "image-watermark", changedEntries: reports.length, message: `找到 ${reports.length} 张带水印图片`, outputs: [], report };
  }
  const text = options.watermarkText?.trim() || "";
  if (!text) throw new Error("水印文本不能为空");
  const payload = watermarkPayload(text);
  for (const entry of [...doc.files].filter((item) => item.kind === "image" && RASTER_EXTENSIONS.has(extension(item.path)))) {
    try {
      const original = await readWebEpubBlob(doc, entry.path, entry.mediaType);
      const { canvas, context } = await canvasEncode(original, "image/png", 1, 0);
      const pixels = context.getImageData(0, 0, canvas.width, canvas.height);
      if (!embedPayload(pixels, payload)) continue;
      context.putImageData(pixels, 0, 0);
      const watermarked = await new Promise<Blob>((resolve, reject) => canvas.toBlob((value) => value ? resolve(value) : reject(new Error("PNG 编码失败")), "image/png"));
      const target = uniquePngPath(doc, entry.path);
      if (target !== entry.path) {
        await rewriteResourceReferences(doc, entry.path, target);
        await renameWebEpubResource(doc, entry.path, target);
      }
      await updateWebEpubBinary(doc, target, watermarked, "image/png");
      changed += 1;
    } catch {
      // Keep unsupported images unchanged and continue processing the EPUB.
    }
  }
  if (!changed) throw new Error("没有图片能够容纳水印文本");
  const blob = await exportWebEpubBlob(doc);
  return {
    sourceNames: [file.name], action: "image-watermark", changedEntries: changed, message: `已向 ${changed} 张图片写入隐形水印`,
    outputs: [{ name: outputName(file.name, "_watermarked"), blob, message: `${changed} 张图片` }],
  };
}

type CollectionSelection = { doc: WebEpubDocument; indexes: number[]; sourceIndex: number };

async function buildCollection(selections: CollectionSelection[], title: string) {
  const output = new JSZip();
  const manifestRows: Array<{ id: string; href: string; mediaType: string; properties: string }> = [];
  const spineRows: Array<{ id: string; href: string; title: string }> = [];
  let resourceCounter = 0;
  let coverId = "";

  for (const selection of selections) {
    const { doc, sourceIndex } = selection;
    const prefix = `OEBPS/Books/book${sourceIndex + 1}/`;
    const pathMap = new Map<string, string>();
    for (const [path, entry] of Object.entries(doc.zip.files)) {
      if (entry.dir || path === "mimetype" || path === doc.opfPath || path.startsWith("META-INF/")) continue;
      pathMap.set(path, `${prefix}${path}`);
    }
    const manifestByPath = new Map(doc.manifest.map((item) => [item.fullPath, item]));
    const ids = new Map<string, string>();
    for (const [oldPath, newPath] of pathMap) {
      const entry = doc.zip.file(oldPath);
      if (!entry) continue;
      const ext = extension(oldPath);
      if (TEXT_REFERENCE_EXTENSIONS.has(ext)) {
        const source = await entry.async("text");
        output.file(newPath, rewriteWebEpubTextLinks(source, oldPath, newPath, pathMap));
      } else output.file(newPath, await entry.async("uint8array"));
      const sourceItem = manifestByPath.get(oldPath);
      const id = `resource-${++resourceCounter}`;
      ids.set(oldPath, id);
      const properties = sourceIndex === 0 && sourceItem?.properties.split(/\s+/).includes("cover-image") ? "cover-image" : "";
      if (properties && !coverId) coverId = id;
      manifestRows.push({ id, href: newPath.replace(/^OEBPS\//, ""), mediaType: sourceItem?.mediaType || guessWebEpubMediaType(oldPath), properties });
    }
    const labels = pathLabelMap(doc);
    for (const index of selection.indexes) {
      const item = doc.spine[index]?.manifest;
      const id = item ? ids.get(item.fullPath) : "";
      const target = item ? pathMap.get(item.fullPath) : "";
      if (!item || !id || !target) continue;
      spineRows.push({ id, href: target.replace(/^OEBPS\//, ""), title: labels.get(item.fullPath) || await xhtmlTitle(doc, item.fullPath) });
    }
  }
  if (!spineRows.length) throw new Error("没有可写入新 EPUB 的章节");

  const identifier = `urn:uuid:${crypto.randomUUID()}`;
  const navChapters = spineRows.map((row) => ({ href: row.href, title: row.title }));
  output.file("mimetype", "application/epub+zip", { compression: "STORE", createFolders: false });
  output.file("META-INF/container.xml", `<?xml version="1.0" encoding="UTF-8"?><container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container"><rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles></container>`);
  output.file("OEBPS/nav.xhtml", navDocument(title, navChapters));
  output.file("OEBPS/toc.ncx", ncxDocument(title, identifier, navChapters));
  const manifest = manifestRows.map((row) => `    <item id="${row.id}" href="${escapeXml(row.href)}" media-type="${escapeXml(row.mediaType)}"${row.properties ? ` properties="${row.properties}"` : ""}/>`).join("\n");
  const spine = spineRows.map((row) => `    <itemref idref="${row.id}"/>`).join("\n");
  const opf = `<?xml version="1.0" encoding="UTF-8"?>\n<package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="BookId"><metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:identifier id="BookId">${identifier}</dc:identifier><dc:title>${escapeXml(title)}</dc:title><dc:language>zh-CN</dc:language>${coverId ? `<meta name="cover" content="${coverId}"/>` : ""}</metadata><manifest>\n${manifest}\n    <item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>\n    <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>\n  </manifest><spine toc="ncx">\n${spine}\n  </spine></package>`;
  output.file("OEBPS/content.opf", opf);
  return output.generateAsync({ type: "blob", mimeType: "application/epub+zip", compression: "DEFLATE", compressionOptions: { level: 6 } });
}

async function mergeEpubs(files: File[], options: WebEpubAdvancedOptions): Promise<WebEpubAdvancedResult> {
  if (files.length < 2) throw new Error("合并 EPUB 至少需要两个文件");
  const docs = await Promise.all(files.map(loadWebEpub));
  const title = options.outputTitle?.trim() || `${docs[0].metadata.title || stem(files[0].name)} 合集`;
  const selections = docs.map((doc, sourceIndex) => ({ doc, sourceIndex, indexes: doc.spine.map((_, index) => index) }));
  const blob = await buildCollection(selections, title);
  const count = selections.reduce((sum, item) => sum + item.indexes.length, 0);
  return {
    sourceNames: files.map((file) => file.name), action: "epub-merge", changedEntries: count, message: `已合并 ${files.length} 本 EPUB，共 ${count} 个阅读顺序条目`,
    outputs: [{ name: `${sanitizeFileName(title)}.epub`, blob, message: `${files.length} 本 / ${count} 章` }],
  };
}

async function splitEpub(file: File, options: WebEpubAdvancedOptions): Promise<WebEpubAdvancedResult> {
  const doc = await loadWebEpub(file);
  const splitEvery = Math.max(1, Math.floor(options.splitEvery || 20));
  if (!doc.spine.length) throw new Error("EPUB 没有可拆分的阅读顺序条目");
  const outputs: WebEpubAdvancedOutput[] = [];
  for (let start = 0, part = 1; start < doc.spine.length; start += splitEvery, part += 1) {
    const indexes = Array.from({ length: Math.min(splitEvery, doc.spine.length - start) }, (_, index) => start + index);
    const title = `${doc.metadata.title || stem(file.name)} ${String(part).padStart(2, "0")}`;
    const blob = await buildCollection([{ doc, sourceIndex: 0, indexes }], title);
    outputs.push({ name: `${sanitizeFileName(title)}.epub`, blob, message: `第 ${start + 1}-${start + indexes.length} 项` });
  }
  return {
    sourceNames: [file.name], action: "epub-split", changedEntries: doc.spine.length, message: `按每 ${splitEvery} 个阅读顺序条目拆分为 ${outputs.length} 本 EPUB`, outputs,
  };
}

export async function processWebEpubAdvanced(files: File[], action: WebEpubAdvancedAction, options: WebEpubAdvancedOptions = {}) {
  const epubFiles = files.filter((file) => file.name.toLowerCase().endsWith(".epub"));
  if (!epubFiles.length) throw new Error("请选择 EPUB 文件");
  if (action === "epub-merge") return mergeEpubs(epubFiles, options);
  const file = epubFiles[0];
  if (action === "epub-to-txt") return epubToTxt(file);
  if (action === "epub-version") return convertVersion(file, options.targetVersion || "3");
  if (action === "epub-chinese") return convertChinese(file, options.chineseDirection || "s2t");
  if (action === "epub-ad-clean") return cleanEpubAds(file, options.adPatterns);
  if (action === "epub-phonetic") return addEpubPhonetics(file);
  if (action === "epub-footnote") return convertEpubFootnotes(file, options.footnoteMode || "standard-to-popup");
  if (action === "image-compress") return compressImages(file, options);
  if (action === "image-watermark") return watermarkImages(file, options);
  return splitEpub(file, options);
}

export const webEpubAdvancedTesting = {
  watermarkPayload,
  embedPayload,
  extractPayload,
};
