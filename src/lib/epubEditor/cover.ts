/**
 * 封面解析：回答「这本书的封面到底是哪个文件」。
 *
 * 原来的做法是拿 manifest 的 id / href / properties 加文件名做子串匹配，
 * 匹配不上就退到 `files[0]`（按路径排序的第一张图）。这有三个坑：
 *
 * 1. 没有读 OPF 里真正的声明：EPUB3 的 `properties="cover-image"`、
 *    EPUB2 的 `<meta name="cover" content="封面图 id"/>`；
 * 2. 没有处理 `<guide><reference type="cover">` 指向的包装页 ——
 *    截图里那种 `Text/cover.xhtml` 里再放 `<svg><image xlink:href>` 的书，
 *    封面图本身可能叫 `00001.jpg`，光看名字永远找不到；
 * 3. 兜底取 files[0] 会拿到插图、作者头像、logo（按字母序排在最前的图）。
 *
 * 所以这里按「声明可信度」排成一条链，逐级降级，最后一级才看大小。
 * 判据全部基于已解析文档，桌面版与 web 版可以共用同一套规则。
 */
import type { WebEpubCoverHints, WebEpubFileEntry } from "$lib/webEpub";

export type CoverCandidate = {
  path: string;
  /** 命中的依据，写进状态栏便于排查 */
  reason: "properties" | "meta" | "guide" | "name" | "largest";
};

export type CoverResolveInput = {
  files: WebEpubFileEntry[];
  hints: WebEpubCoverHints;
  /** 读出指定路径的文本内容；文件不存在返回 null */
  readText: (path: string) => Promise<string | null>;
};

const IMAGE_EXTENSIONS = ["jpg", "jpeg", "png", "gif", "webp", "svg", "bmp"];

function isImagePath(path: string) {
  const ext = path.split(".").pop()?.toLowerCase() || "";
  return IMAGE_EXTENSIONS.includes(ext);
}

function dirOf(path: string) {
  const index = path.lastIndexOf("/");
  return index >= 0 ? path.slice(0, index + 1) : "";
}

function normalizePath(path: string) {
  const parts: string[] = [];
  for (const part of path.replace(/\\/g, "/").split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") parts.pop();
    else parts.push(part);
  }
  return parts.join("/");
}

function resolveRelative(basePath: string, href: string) {
  const clean = (href.split("#")[0] || "").trim();
  if (!clean || /^(?:data|blob|https?|mailto|tel):/i.test(clean)) return "";
  try {
    return normalizePath(
      `${dirOf(basePath)}${decodeURIComponent(clean).replace(/^\//, "")}`,
    );
  } catch {
    return normalizePath(`${dirOf(basePath)}${clean.replace(/^\//, "")}`);
  }
}

/**
 * 从 XHTML 包装页里挑出封面图。
 *
 * 用 DOMParser 而不是正则：`xlink:href` 在 XML 里是带命名空间的属性，
 * `getAttribute("href")` 读不到（这正是预览里封面空白的原因之一），
 * 必须走 getAttributeNS。
 */
export function extractImageFromWrapper(
  source: string,
  wrapperPath: string,
): string {
  if (typeof DOMParser === "undefined") return "";
  const document = new DOMParser().parseFromString(source, "application/xml");
  if (document.querySelector("parsererror")) return "";
  const XLINK = "http://www.w3.org/1999/xlink";

  for (const element of Array.from(document.querySelectorAll("*"))) {
    const name = element.localName;
    if (name === "image") {
      const href =
        element.getAttributeNS(XLINK, "href") ||
        element.getAttribute("xlink:href") ||
        element.getAttribute("href") ||
        "";
      if (href) return resolveRelative(wrapperPath, href);
    }
    if (name === "img") {
      const src = element.getAttribute("src") || "";
      if (src) return resolveRelative(wrapperPath, src);
    }
    if (name === "object") {
      const data = element.getAttribute("data") || "";
      if (data) return resolveRelative(wrapperPath, data);
    }
    if (name === "embed") {
      const src = element.getAttribute("src") || "";
      if (src) return resolveRelative(wrapperPath, src);
    }
  }
  return "";
}

/** 文件名像封面的程度：0 表示不像 */
function nameScore(path: string) {
  const name = (path.split("/").pop() || "").toLowerCase();
  const stem = name.replace(/\.[^.]+$/, "");
  let score = 0;
  if (/(^|[-_. ])cover([-_. ]|$)/.test(stem)) score += 6;
  if (/^cover/.test(stem)) score += 4;
  if (/cover/.test(stem)) score += 2;
  if (/封面/.test(stem)) score += 6;
  // 缩略图 / 变形版本排在正式封面后面
  if (/thumb|small|slim|缩略/.test(stem)) score -= 3;
  if (/titlepage|title-page|logo|author|avatar/.test(stem)) score -= 4;
  return score;
}

/**
 * 按可信度解析封面。找不到时返回 null —— 调用方显示「无封面」占位，
 * 不要再退回 files[0] 乱猜。
 */
export async function resolveEpubCover(
  input: CoverResolveInput,
): Promise<CoverCandidate | null> {
  const { files, hints, readText } = input;
  const imageFiles = files.filter(
    (file) => file.kind === "image" || isImagePath(file.path),
  );
  const hasImage = (path: string) => imageFiles.some((f) => f.path === path);

  // 1) EPUB3：manifest properties="cover-image"
  if (hints.propertiesPath && hasImage(hints.propertiesPath)) {
    return { path: hints.propertiesPath, reason: "properties" };
  }

  // 2) EPUB2：<meta name="cover" content="id">
  if (hints.metaPath && hasImage(hints.metaPath)) {
    return { path: hints.metaPath, reason: "meta" };
  }

  // 3) <guide><reference type="cover"> 指向的包装页
  if (hints.guidePath) {
    if (hasImage(hints.guidePath)) {
      return { path: hints.guidePath, reason: "guide" };
    }
    try {
      const source = await readText(hints.guidePath);
      if (source) {
        const imagePath = extractImageFromWrapper(source, hints.guidePath);
        if (imagePath && hasImage(imagePath)) {
          return { path: imagePath, reason: "guide" };
        }
      }
    } catch {
      // 包装页读不出来就继续往下走
    }
  }

  // 4) 名字像封面的图（按分数挑，同分取大的 —— 缩略图通常更小）
  const named = imageFiles
    .map((file) => ({ file, score: nameScore(file.path) }))
    .filter((item) => item.score > 0)
    .sort(
      (left, right) =>
        right.score - left.score ||
        (right.file.size || 0) - (left.file.size || 0),
    );
  if (named.length) return { path: named[0].file.path, reason: "name" };

  // 5) 实在没有线索：取最大的图（封面通常比插图和 logo 大），
  //    但仍然不取 files[0] —— 那是按路径排序的第一张，几乎总是错的
  const largest = [...imageFiles].sort(
    (left, right) => (right.size || 0) - (left.size || 0),
  );
  if (largest.length && (largest[0].size || 0) > 0) {
    return { path: largest[0].path, reason: "largest" };
  }

  return null;
}
