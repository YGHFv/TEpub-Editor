import type { EpubStyleModule } from "$lib/epubStyleLibrary";
import {
  addWebEpubResource,
  exportWebEpubBlob,
  normalizeZipPath,
  readWebEpubText,
  updateWebEpubText,
  type WebEpubDocument,
} from "$lib/webEpub";

export type RemoteImageReference = {
  url: string;
  occurrences: number;
  encodedOccurrences: number;
  filePaths: string[];
};

export type DownloadedRemoteImage = {
  bytes: Uint8Array;
  mediaType: string;
  finalUrl?: string;
};

export type RemoteImageDownloadFailure = {
  url: string;
  message: string;
};

export type RemoteImageEmbedResult = {
  blob: Blob;
  outputName: string;
  downloadedImages: number;
  failedImages: RemoteImageDownloadFailure[];
  replacedOccurrences: number;
  changedFiles: number;
};

export type RemoteImageProgress = {
  index: number;
  total: number;
  url: string;
  stage: "downloading" | "retrying" | "downloaded" | "failed" | "rewriting";
  message: string;
};

type ImageTagInfo = { url: string; alt: string };

const ONLINE_STYLE_MARKER = "TEpub online illustration embedding";
const ONLINE_STYLE_LINK_ATTRIBUTE = "data-tepub-online-illustrations";
const ACTUAL_IMAGE_PATTERN = /<img\b[^>]*>/gi;
const ENCODED_IMAGE_PATTERN = /&lt;img\b[\s\S]*?&gt;/gi;
const MAX_REMOTE_IMAGE_BYTES = 32 * 1024 * 1024;

function decodeHtmlEntities(value: string) {
  return value
    .replace(/&quot;|&#34;|&#x22;/gi, '"')
    .replace(/&apos;|&#39;|&#x27;/gi, "'")
    .replace(/&lt;/gi, "<")
    .replace(/&gt;/gi, ">")
    .replace(/&amp;/gi, "&");
}

function escapeXml(value: string) {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

function attributeValue(tag: string, name: string) {
  const pattern = new RegExp(`\\b${name}\\s*=\\s*(?:"([^"]*)"|'([^']*)'|([^\\s>]+))`, "i");
  const match = pattern.exec(tag);
  return decodeHtmlEntities(match?.[1] ?? match?.[2] ?? match?.[3] ?? "").trim();
}

function imageTagInfo(tag: string, encoded = false): ImageTagInfo | null {
  const decodedTag = encoded ? decodeHtmlEntities(tag) : tag;
  const url = attributeValue(decodedTag, "src");
  if (!/^https?:\/\//i.test(url)) return null;
  return { url, alt: attributeValue(decodedTag, "alt") };
}

function collectRemoteTags(source: string) {
  const tags: Array<ImageTagInfo & { encoded: boolean }> = [];
  for (const match of source.matchAll(ACTUAL_IMAGE_PATTERN)) {
    const info = imageTagInfo(match[0]);
    if (info) tags.push({ ...info, encoded: false });
  }
  for (const match of source.matchAll(ENCODED_IMAGE_PATTERN)) {
    const info = imageTagInfo(match[0], true);
    if (info) tags.push({ ...info, encoded: true });
  }
  return tags;
}

export async function scanWebEpubRemoteImages(doc: WebEpubDocument) {
  const references = new Map<string, RemoteImageReference>();
  for (const file of doc.files.filter((entry) => entry.kind === "xhtml")) {
    const source = await readWebEpubText(doc, file.path);
    for (const tag of collectRemoteTags(source)) {
      const current = references.get(tag.url) || {
        url: tag.url,
        occurrences: 0,
        encodedOccurrences: 0,
        filePaths: [],
      };
      current.occurrences += 1;
      if (tag.encoded) current.encodedOccurrences += 1;
      if (!current.filePaths.includes(file.path)) current.filePaths.push(file.path);
      references.set(tag.url, current);
    }
  }
  return [...references.values()].sort((left, right) => left.url.localeCompare(right.url));
}

function dirname(path: string) {
  const index = path.lastIndexOf("/");
  return index >= 0 ? path.slice(0, index + 1) : "";
}

function relativeHref(fromFile: string, targetPath: string) {
  const from = normalizeZipPath(dirname(fromFile)).split("/").filter(Boolean);
  const target = normalizeZipPath(targetPath).split("/").filter(Boolean);
  while (from.length && target.length && from[0] === target[0]) {
    from.shift();
    target.shift();
  }
  return [...from.map(() => ".."), ...target].join("/");
}

function mediaTypeFromBytes(bytes: Uint8Array, declared: string, url: string) {
  const mediaType = declared.split(";", 1)[0].trim().toLowerCase();
  if (bytes[0] === 0xff && bytes[1] === 0xd8) return { mediaType: "image/jpeg", extension: "jpg" };
  if (bytes[0] === 0x89 && bytes[1] === 0x50 && bytes[2] === 0x4e && bytes[3] === 0x47) return { mediaType: "image/png", extension: "png" };
  if (String.fromCharCode(...bytes.slice(0, 4)) === "GIF8") return { mediaType: "image/gif", extension: "gif" };
  if (String.fromCharCode(...bytes.slice(0, 4)) === "RIFF" && String.fromCharCode(...bytes.slice(8, 12)) === "WEBP") {
    return { mediaType: "image/webp", extension: "webp" };
  }
  throw new Error(`响应不是支持的 JPG、PNG、WebP 或 GIF 图片（${mediaType || new URL(url).pathname.split(".").pop() || "未知类型"}）`);
}

function safeUrlStem(url: string) {
  try {
    const raw = decodeURIComponent(new URL(url).pathname.split("/").pop() || "").replace(/\.[^.]+$/, "");
    const clean = raw.replace(/[^A-Za-z0-9_-]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 48);
    return clean || "online-image";
  } catch {
    return "online-image";
  }
}

function uniqueImagePath(doc: WebEpubDocument, url: string, extension: string, reserved: Set<string>) {
  const directory = normalizeZipPath(`${doc.opfDir}Images/online`);
  const stem = safeUrlStem(url);
  let path = `${directory}/${stem}.${extension}`;
  let suffix = 2;
  while (doc.zip.file(path) || reserved.has(path)) path = `${directory}/${stem}-${suffix++}.${extension}`;
  reserved.add(path);
  return path;
}

function replaceAttribute(tag: string, name: string, value: string) {
  const encoded = escapeXml(value);
  const pattern = new RegExp(`\\b${name}\\s*=\\s*(?:"[^"]*"|'[^']*'|[^\\s>]+)`, "i");
  if (pattern.test(tag)) return tag.replace(pattern, `${name}="${encoded}"`);
  return tag.replace(/\s*\/?\s*>$/, (end) => ` ${name}="${encoded}"${end}`);
}

function ensureImageClass(tag: string) {
  const classPattern = /\bclass\s*=\s*(?:"([^"]*)"|'([^']*)')/i;
  const match = classPattern.exec(tag);
  if (!match) return tag.replace(/\s*\/?\s*>$/, (end) => ` class="te-illustration-image"${end}`);
  const classes = (match[1] ?? match[2] ?? "").split(/\s+/).filter(Boolean);
  if (!classes.includes("te-illustration-image")) classes.push("te-illustration-image");
  return tag.replace(match[0], `class="${classes.join(" ")}"`);
}

function localImageTag(sourceTag: string, localHref: string, alt: string) {
  let tag = decodeHtmlEntities(sourceTag);
  tag = replaceAttribute(tag, "src", localHref);
  tag = replaceAttribute(tag, "alt", alt);
  tag = ensureImageClass(tag);
  return tag.replace(/\s*\/?\s*>$/, " />");
}

function illustrationMarkup(style: EpubStyleModule, localHref: string, alt: string, originalTag: string) {
  const fallback = `<figure class="te-illustration">\n  ${localImageTag(originalTag, localHref, alt)}\n</figure>`;
  const template = style.markup?.trim() || fallback;
  const imageMatch = template.match(/<img\b[^>]*>/i);
  if (!imageMatch) return fallback;
  let output = template.replace(imageMatch[0], localImageTag(imageMatch[0], localHref, alt));
  const captionPattern = /<figcaption\b[^>]*>[\s\S]*?<\/figcaption>/i;
  if (alt) {
    output = output.replace(captionPattern, (caption) => caption.replace(/>[^<]*(<\/figcaption>)/i, `>${escapeXml(alt)}$1`));
  } else {
    output = output.replace(captionPattern, "");
  }
  return output;
}

function rewriteRemoteImages(
  source: string,
  xhtmlPath: string,
  localPathByUrl: Map<string, string>,
  style: EpubStyleModule,
) {
  let replacements = 0;
  const replaceStandalone = (match: string, tag: string, encoded: boolean) => {
    const info = imageTagInfo(tag, encoded);
    const localPath = info ? localPathByUrl.get(info.url) : "";
    if (!info || !localPath) return match;
    replacements += 1;
    return illustrationMarkup(style, relativeHref(xhtmlPath, localPath), info.alt, tag);
  };

  let output = source.replace(/<p\b[^>]*>\s*(<img\b[^>]*>)\s*<\/p>/gi, (match, tag) => replaceStandalone(match, tag, false));
  output = output.replace(/<p\b[^>]*>\s*(&lt;img\b[\s\S]*?&gt;)\s*<\/p>/gi, (match, tag) => replaceStandalone(match, tag, true));
  output = output.replace(ACTUAL_IMAGE_PATTERN, (tag) => {
    const info = imageTagInfo(tag);
    const localPath = info ? localPathByUrl.get(info.url) : "";
    if (!info || !localPath) return tag;
    replacements += 1;
    return localImageTag(tag, relativeHref(xhtmlPath, localPath), info.alt);
  });
  output = output.replace(ENCODED_IMAGE_PATTERN, (tag) => {
    const info = imageTagInfo(tag, true);
    const localPath = info ? localPathByUrl.get(info.url) : "";
    if (!info || !localPath) return tag;
    replacements += 1;
    return localImageTag(tag, relativeHref(xhtmlPath, localPath), info.alt);
  });
  return { output, replacements };
}

function injectStylesheetLink(source: string, xhtmlPath: string, cssPath: string) {
  const existingPattern = new RegExp(`<link\\b[^>]*${ONLINE_STYLE_LINK_ATTRIBUTE}\\s*=\\s*(["'])1\\1[^>]*\\/?\\s*>`, "gi");
  const cleaned = source.replace(existingPattern, "");
  const href = escapeXml(relativeHref(xhtmlPath, cssPath));
  const link = `<link rel="stylesheet" type="text/css" href="${href}" ${ONLINE_STYLE_LINK_ATTRIBUTE}="1" />`;
  if (/<\/head\s*>/i.test(cleaned)) return cleaned.replace(/<\/head\s*>/i, `  ${link}\n</head>`);
  return `${link}\n${cleaned}`;
}

async function installIllustrationStylesheet(doc: WebEpubDocument, style: EpubStyleModule) {
  let cssPath = normalizeZipPath(`${doc.opfDir}Styles/tepub-online-illustrations.css`);
  let suffix = 2;
  while (doc.zip.file(cssPath)) {
    const content = await doc.zip.file(cssPath)!.async("string");
    if (content.includes(ONLINE_STYLE_MARKER)) break;
    cssPath = normalizeZipPath(`${doc.opfDir}Styles/tepub-online-illustrations-${suffix++}.css`);
  }
  const css = `/* ${ONLINE_STYLE_MARKER} */\n\n${style.css.trim()}\n`;
  if (doc.zip.file(cssPath)) updateWebEpubText(doc, cssPath, css);
  else await addWebEpubResource(doc, { path: cssPath, content: css, mediaType: "text/css" });
  return cssPath;
}

function outputName(sourceName: string) {
  const stem = sourceName.replace(/\.epub$/i, "") || "book";
  return `${stem}_online_images.epub`;
}

export async function embedWebEpubRemoteImages(
  doc: WebEpubDocument,
  urls: string[],
  style: EpubStyleModule,
  downloader: (url: string) => Promise<DownloadedRemoteImage>,
  onProgress?: (progress: RemoteImageProgress) => void,
  retryDelayMs = 1500,
): Promise<RemoteImageEmbedResult> {
  const selectedUrls = [...new Set(urls.filter((url) => /^https?:\/\//i.test(url)))];
  if (!selectedUrls.length) throw new Error("没有选择需要下载的在线图片。");
  if (style.kind !== "illustration" || style.target === "annotation-illustration") {
    throw new Error("请选择普通插图样式。");
  }

  const reservedPaths = new Set<string>();
  const localPathByUrl = new Map<string, string>();
  const failures: RemoteImageDownloadFailure[] = [];
  for (const [index, url] of selectedUrls.entries()) {
    onProgress?.({ index, total: selectedUrls.length, url, stage: "downloading", message: `正在下载 ${index + 1} / ${selectedUrls.length}` });
    let downloaded: DownloadedRemoteImage | null = null;
    let type: ReturnType<typeof mediaTypeFromBytes> | null = null;
    try {
      for (let attempt = 0; attempt < 2; attempt += 1) {
        try {
          downloaded = await downloader(url);
          if (!downloaded.bytes.byteLength) throw new Error("图片响应为空");
          if (downloaded.bytes.byteLength > MAX_REMOTE_IMAGE_BYTES) throw new Error("图片超过 32 MB 限制");
          type = mediaTypeFromBytes(downloaded.bytes, downloaded.mediaType, downloaded.finalUrl || url);
          break;
        } catch (error) {
          if (attempt === 1) throw error;
          const reason = error instanceof Error ? error.message : String(error);
          onProgress?.({
            index,
            total: selectedUrls.length,
            url,
            stage: "retrying",
            message: `第 ${index + 1} 张下载失败，${(retryDelayMs / 1000).toFixed(1)} 秒后慢速重试：${reason}`,
          });
          if (retryDelayMs > 0) await new Promise((resolve) => setTimeout(resolve, retryDelayMs));
        }
      }
      if (!downloaded || !type) throw new Error("图片下载失败");
      const path = uniqueImagePath(doc, downloaded.finalUrl || url, type.extension, reservedPaths);
      await addWebEpubResource(doc, { path, content: downloaded.bytes, mediaType: type.mediaType });
      localPathByUrl.set(url, path);
      onProgress?.({ index: index + 1, total: selectedUrls.length, url, stage: "downloaded", message: `已下载 ${index + 1} / ${selectedUrls.length}` });
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      failures.push({ url, message });
      onProgress?.({ index: index + 1, total: selectedUrls.length, url, stage: "failed", message });
    }
  }
  if (!localPathByUrl.size) {
    return {
      blob: await exportWebEpubBlob(doc),
      outputName: outputName(doc.fileName),
      downloadedImages: 0,
      failedImages: failures,
      replacedOccurrences: 0,
      changedFiles: 0,
    };
  }

  const cssPath = await installIllustrationStylesheet(doc, style);
  let replacedOccurrences = 0;
  let changedFiles = 0;
  onProgress?.({ index: selectedUrls.length, total: selectedUrls.length, url: "", stage: "rewriting", message: "正在更新 XHTML 与 OPF" });
  for (const file of doc.files.filter((entry) => entry.kind === "xhtml")) {
    const source = await readWebEpubText(doc, file.path);
    const rewritten = rewriteRemoteImages(source, file.path, localPathByUrl, style);
    if (!rewritten.replacements) continue;
    updateWebEpubText(doc, file.path, injectStylesheetLink(rewritten.output, file.path, cssPath));
    replacedOccurrences += rewritten.replacements;
    changedFiles += 1;
  }

  return {
    blob: await exportWebEpubBlob(doc),
    outputName: outputName(doc.fileName),
    downloadedImages: localPathByUrl.size,
    failedImages: failures,
    replacedOccurrences,
    changedFiles,
  };
}

export const webEpubRemoteImagesTesting = {
  collectRemoteTags,
  imageTagInfo,
  localImageTag,
  rewriteRemoteImages,
};
