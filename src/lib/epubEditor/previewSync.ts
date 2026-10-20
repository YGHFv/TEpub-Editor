/**
 * 编辑器 <-> 预览 的双向定位。
 *
 * 预览 iframe 现在按正文高度撑开、由外层容器滚动，因此：
 * - 定位时不能调 iframe 内部的 scrollIntoView（它自己已经不可滚动），
 *   要把命中元素相对 iframe 的偏移换算成外层容器的 scrollTop；
 * - 也不能用 postMessage 往返，两边同源、直接调函数即可。
 */

export type RevealMode = "center" | "nearest";

/** 空白字符（含不换行空格） */
const SPACE = /[\s ]/;

/** HTML 里的空白统一折叠成单个空格 */
export function collapseWhitespace(value: string): string {
  return value.replace(/[\s ]+/g, " ").trim();
}

/**
 * 去掉所有空白，只留文字。
 * 编辑器里的 `<br/>`、换行、缩进会让同一段内容和预览里的文本
 * 在空白上对不上（中文尤其明显），所以匹配前两边都去空白。
 */
export function stripWhitespace(value: string): string {
  return value.replace(/[\s ]+/g, "");
}

/**
 * 在去空白后的文本里定位 needle，返回 { at, length }（纯函数，便于单测）。
 * needle 同样先去掉空白；找不到返回 null。
 */
export function findStrippedMatch(
  haystack: string,
  needle: string,
): { at: number; length: number } | null {
  const target = stripWhitespace(needle);
  if (!target) return null;
  const at = stripWhitespace(haystack).indexOf(target);
  return at < 0 ? null : { at, length: target.length };
}

interface TextEntry {
  node: Text;
  /** 该文本节点在原始文本流中的起始偏移 */
  start: number;
  length: number;
}

interface TextIndex {
  /** 去掉空白后的文本 */
  text: string;
  /** text 中每个字符 -> 原始文本偏移 */
  positions: number[];
  entries: TextEntry[];
}

function buildTextIndex(root: Node): TextIndex {
  const doc = root.ownerDocument ?? (root as Document);
  const entries: TextEntry[] = [];
  const positions: number[] = [];
  const chars: string[] = [];
  let raw = 0;

  const walker = doc.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let current = walker.nextNode() as Text | null;
  while (current) {
    const value = current.nodeValue ?? "";
    if (value) {
      entries.push({ node: current, start: raw, length: value.length });
      for (let i = 0; i < value.length; i++) {
        if (SPACE.test(value[i])) continue;
        positions.push(raw + i);
        chars.push(value[i]);
      }
      raw += value.length;
    }
    current = walker.nextNode() as Text | null;
  }

  return { text: chars.join(""), positions, entries };
}

function entryAt(index: TextIndex, rawOffset: number): TextEntry | null {
  for (const entry of index.entries) {
    if (rawOffset >= entry.start && rawOffset < entry.start + entry.length) {
      return entry;
    }
    if (rawOffset < entry.start) return null;
  }
  const last = index.entries[index.entries.length - 1];
  if (last && rawOffset === last.start + last.length) return last;
  return null;
}

export interface TextHit {
  range: Range;
  /** 命中文本所在的块级元素 */
  element: Element;
}

/** 向上找到最近的块级祖先，避免把整篇内容当成"点击段落" */
export function closestBlock(node: Element | null): Element | null {
  const view = node?.ownerDocument?.defaultView;
  let current: Element | null = node;
  while (current && current !== current.ownerDocument?.body) {
    const display = view?.getComputedStyle(current).display;
    if (display && !display.startsWith("inline") && display !== "contents") {
      return current;
    }
    current = current.parentElement;
  }
  return null;
}

/**
 * 在文档中查找一段文本（忽略标签与空白差异）。
 * 找不到返回 null；命中时返回可用于度量的 Range。
 */
export function findTextHit(doc: Document, needle: string): TextHit | null {
  if (!doc.body) return null;

  const index = buildTextIndex(doc.body);
  const match = findStrippedMatch(index.text, needle);
  if (!match) return null;
  const { at, length } = match;

  const rawStart = index.positions[at];
  const lastChar = index.positions[at + length - 1];
  // Range 的 end 是排他偏移：要用「最后一个字符所在节点 + 该字符之后」，
  // 直接把 rawStart + 1 当成节点内偏移会串到下一个文本节点里
  const startEntry = entryAt(index, rawStart);
  const endEntry = entryAt(index, lastChar);
  if (!startEntry || !endEntry) return null;

  const range = doc.createRange();
  range.setStart(startEntry.node, rawStart - startEntry.start);
  range.setEnd(
    endEntry.node,
    Math.min(endEntry.length, lastChar - endEntry.start + 1),
  );

  const probe =
    range.startContainer.nodeType === Node.TEXT_NODE
      ? (range.startContainer.parentElement ?? doc.body)
      : ((range.startContainer as Element) ?? doc.body);
  const element = closestBlock(probe) ?? probe;

  return { range, element };
}

/**
 * 把"iframe 内部坐标"换算成外层滚动容器的 scrollTop。
 * 返回 null 表示无需滚动（nearest 模式下目标已在视野内）。
 */
function toHostScrollTop(
  host: HTMLElement,
  frame: HTMLIFrameElement,
  innerTop: number,
  innerHeight: number,
  mode: RevealMode,
): number | null {
  // iframe 相对外层滚动内容的偏移。
  // scrollTop 以 padding box 为原点，rect 差值加上当前 scrollTop 即为内容坐标。
  const hostRect = host.getBoundingClientRect();
  const frameRect = frame.getBoundingClientRect();
  const frameOffset = frameRect.top - hostRect.top + host.scrollTop;
  const itemTop = frameOffset + innerTop;
  const itemBottom = itemTop + innerHeight;

  if (mode === "nearest") {
    const viewTop = host.scrollTop;
    const viewBottom = viewTop + host.clientHeight;
    if (itemTop >= viewTop && itemBottom <= viewBottom) return null;
    if (itemTop < viewTop) return Math.max(0, itemTop - 12);
    return Math.max(0, itemBottom - host.clientHeight + 12);
  }

  // center：把命中的行放到可视区中间
  return Math.max(0, itemTop + innerHeight / 2 - host.clientHeight / 2);
}

export interface RevealOptions {
  mode?: RevealMode;
  smooth?: boolean;
}

/**
 * 把预览滚动到指定文本所在的位置。
 * 返回是否命中（未命中时调用方保留当前滚动位置）。
 */
export function revealTextInPreview(
  host: HTMLElement | null,
  frame: HTMLIFrameElement | null,
  text: string,
  options: RevealOptions = {},
): boolean {
  const doc = frame?.contentDocument;
  if (!host || !frame || !doc?.body) return false;

  const hit = findTextHit(doc, text);
  if (!hit) return false;

  // 内部坐标都以 iframe 视口为原点：range 的量测优先，
  // 空行 / 纯图片这类量不到矩形时退回块级元素
  const rangeRect = hit.range.getBoundingClientRect();
  const hasMetrics = rangeRect.height > 0;
  const elementRect = hit.element.getBoundingClientRect();
  const innerTop = hasMetrics ? rangeRect.top : elementRect.top;
  const innerHeight = hasMetrics ? rangeRect.height : elementRect.height;

  const target = toHostScrollTop(
    host,
    frame,
    innerTop,
    innerHeight,
    options.mode ?? "center",
  );
  if (target === null) return true;

  if (options.smooth === false) host.scrollTop = target;
  else host.scrollTo({ top: target, behavior: "smooth" });
  return true;
}

export interface PreviewClickPayload {
  text: string;
  context: string;
}

/** 预览里点到链接 / 注脚时不触发反向定位 */
const IGNORED_CLICK = "a, .duokan-footnote-back, .footnote-popup-backdrop";

/**
 * 预览区被点击时，整理出"点到了哪段文字"以及它所在的块级上下文。
 * 用于反向同步：预览点击 -> 编辑器定位。
 */
export function describePreviewClick(
  target: Element | null,
): PreviewClickPayload | null {
  if (!target) return null;
  if (target.closest(IGNORED_CLICK)) return null;
  const doc = target.ownerDocument;

  const selection = doc.getSelection();
  let text = "";
  if (selection && selection.toString().trim().length > 1) {
    text = selection.toString();
  } else {
    text = (target as HTMLElement).innerText || target.textContent || "";
    if (text.length > 150) text = text.slice(0, 150);
  }
  text = collapseWhitespace(text);
  if (!text) return null;

  let context = "";
  const block = closestBlock(target);
  if (block) {
    context = collapseWhitespace(
      ((block as HTMLElement).innerText || block.textContent || "").slice(
        0,
        500,
      ),
    );
  }

  return { text, context };
}
