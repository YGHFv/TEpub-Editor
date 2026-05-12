export type ProofTitleScope = "all" | "volumes" | "chapters" | "regex";
export type ProofNumberStyle = "arabic" | "chinese";
export type ProofConvertDirection = "simplified-to-traditional" | "traditional-to-simplified";
export type ProofBuiltinRuleId = "title-brackets" | "ads" | "pinyin";

export interface ProofTocNode {
  id?: string;
  title: string;
  line: number;
  type?: "Volume" | "Chapter" | "Meta" | string;
  parentId?: string;
  level?: number;
}

export interface ProofTitleRewriteOptions {
  scope: ProofTitleScope;
  regex: string;
  volumeNumberStyle: ProofNumberStyle;
  chapterNumberStyle: ProofNumberStyle;
  perVolume: boolean;
}

export interface ProofTitlePreviewRow {
  id: string;
  line: number;
  kind: "volume" | "chapter";
  volumeKey: string;
  original: string;
  replacement: string;
  changed: boolean;
  sequenceBroken: boolean;
  originalIndex: number | null;
  expectedIndex: number;
}

export interface ProofTransformResult {
  text: string;
  changedCount: number;
  message: string;
}

export interface ProofBuiltinRegexRule {
  id: ProofBuiltinRuleId;
  name: string;
  description: string;
}

export interface ProofRegexPreviewRow {
  id: string;
  ruleId: ProofBuiltinRuleId;
  lineStart: number;
  lineEnd: number;
  original: string;
  replacement: string;
}

type ChineseConverter = (text: string) => string;

const converterCache: Partial<Record<ProofConvertDirection, ChineseConverter>> = {};

async function getChineseConverter(direction: ProofConvertDirection) {
  if (converterCache[direction]) return converterCache[direction]!;

  const { default: OpenCC } = await import("opencc-js");
  const converter =
    direction === "traditional-to-simplified"
      ? OpenCC.Converter({ from: "hk", to: "cn" })
      : OpenCC.Converter({ from: "cn", to: "tw" });
  converterCache[direction] = converter;
  return converter;
}


const CHINESE_DIGITS = [
  "零",
  "一",
  "二",
  "三",
  "四",
  "五",
  "六",
  "七",
  "八",
  "九",
];

const CHINESE_CHAR_RE = /[\u3400-\u9fff]/;
const PINYIN_MARK_RE =
  /[āáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňḿẁẃẅÿ]/i;
const PINYIN_WORD_RE = /^[A-Za-züÜvVāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňḿ'’\-\s0-9]+$/;
const PINYIN_LATIN_RE = /[A-Za-züÜvVāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňḿ]/;
const PINYIN_TONE_RE = /[A-Za-züÜvVāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňḿ][1-5]\b/;
const TRAILING_PINYIN_RE = /\s+([A-Za-züÜvVāáǎàēéěèīíǐìōóǒòūúǔùǖǘǚǜńňḿ'’\-\s0-9]+)$/;
const AD_LINE_RE = /(月票|求票|求推荐票|推荐票|订阅|本章完|求收藏|求打赏|打赏)/i;
const AD_BLOCK_START_RE =
  /^\s*(?:(?:请假条|请假|上架感言|上架通知|完本感言|作者的话|本书上架)|(?:PS\d*|P\.S\.?\d*|ps\d*)\s*(?:[:：.．、\-—]|$))/i;

export const PROOF_BUILTIN_REGEX_RULES: ProofBuiltinRegexRule[] = [
  {
    id: "title-brackets",
    name: "标题括号",
    description: "标题中被 （）()【】[] 包起来的内容，替换为空",
  },
  {
    id: "ads",
    name: "广告/感言/PS",
    description: "月票、求票、订阅、本章完、请假条、上架感言，以及 PS 段落",
  },
  {
    id: "pinyin",
    name: "拼音",
    description: "括号内拼音、独立拼音行和行尾拼音注音，替换为空",
  },
];

function normalizeSpaces(text: string) {
  return text.replace(/\u3000/g, " ").replace(/[ \t]+/g, " ").trim();
}

function isPinyinLike(text: string) {
  const trimmed = normalizeSpaces(text);
  if (!trimmed) return false;
  if (CHINESE_CHAR_RE.test(trimmed)) return false;
  if (!PINYIN_WORD_RE.test(trimmed)) return false;
  if (!PINYIN_LATIN_RE.test(trimmed)) return false;
  return (
    PINYIN_MARK_RE.test(trimmed) ||
    PINYIN_TONE_RE.test(trimmed) ||
    trimmed.split(/\s+/).length >= 2
  );
}

function stripTrailingPinyin(text: string) {
  return text.replace(TRAILING_PINYIN_RE, (match, tail: string) =>
    isPinyinLike(tail) ? "" : match,
  );
}

function cleanTitleBracketsLine(line: string) {
  const cleaned = line
    .replace(/[（(【\[][^（）()\[\]【】]*[）)】\]]/g, "")
    .replace(/[ \t\u3000]+/g, " ")
    .replace(/\s*([：:、.．\-—])\s*/g, "$1")
    .trim();
  if (cleaned === line.trim()) return line;
  const indent = line.match(/^[\s　]*/)?.[0] ?? "";
  return cleaned ? `${indent}${cleaned}` : indent.trimEnd();
}

function cleanPinyinLine(line: string) {
  const trimmed = line.trim();
  if (!trimmed) return line;

  if (!CHINESE_CHAR_RE.test(trimmed)) {
    return isPinyinLike(trimmed) ? "" : line;
  }

  const cleaned = stripTrailingPinyin(line)
    .replace(/[（(【\[]\s*([^（）()\[\]【】]+)\s*[）)】\]]/g, (match, inner) =>
      isPinyinLike(inner) ? "" : match,
    )
    .replace(/[ \t\u3000]+/g, " ")
    .replace(/\s*([：:、.．\-—])\s*/g, "$1")
    .trim();

  if (cleaned === trimmed) return line;
  const indent = line.match(/^[\s　]*/)?.[0] ?? "";
  return cleaned ? `${indent}${cleaned}` : indent.trimEnd();
}

function isTitleLikeLine(line: string) {
  const trimmed = normalizeSpaces(line);
  if (!trimmed) return false;
  if (
    /^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*(?:[章节回卷部])(?:\s|[:：、.．\-—]|$)/.test(
      trimmed,
    ) ||
    /^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*(?:[章节回卷部])$/.test(
      trimmed,
    ) ||
    /^Chapter\s*\d+(?:\s|[:：、.．\-—]|$)/i.test(trimmed) ||
    /^序列\s*[0-9零〇一二两三四五六七八九十百千万]+(?:\s|[:：、.．\-—]|$)/.test(
      trimmed,
    )
  ) {
    return true;
  }

  if (/^\d{1,5}(?:\s*[\.\．、:：\-—]\s*|\s+|(?=[\u4e00-\u9fff])).+/.test(trimmed)) return true;
  if (/^[一二三四五六七八九十百千万零〇两]{1,6}(?:\s*[\.\．、:：\-—]\s*|\s+).+/.test(trimmed)) return true;
  return false;
}

function toChineseNumber(num: number): string {
  if (!Number.isFinite(num) || num <= 0) return String(num);
  if (num < 10) return CHINESE_DIGITS[num];
  if (num < 20) {
    return num === 10 ? "十" : `十${CHINESE_DIGITS[num % 10]}`;
  }
  if (num < 100) {
    const tens = Math.floor(num / 10);
    const ones = num % 10;
    return `${CHINESE_DIGITS[tens]}十${ones ? CHINESE_DIGITS[ones] : ""}`;
  }
  if (num < 1000) {
    const hundreds = Math.floor(num / 100);
    const rest = num % 100;
    return `${CHINESE_DIGITS[hundreds]}百${rest ? (rest < 10 ? `零${toChineseNumber(rest)}` : toChineseNumber(rest)) : ""}`;
  }
  if (num < 10000) {
    const thousands = Math.floor(num / 1000);
    const rest = num % 1000;
    return `${CHINESE_DIGITS[thousands]}千${rest ? (rest < 100 ? `零${toChineseNumber(rest)}` : toChineseNumber(rest)) : ""}`;
  }
  const wan = Math.floor(num / 10000);
  const rest = num % 10000;
  return `${toChineseNumber(wan)}万${rest ? (rest < 1000 ? `零${toChineseNumber(rest)}` : toChineseNumber(rest)) : ""}`;
}

function formatNumber(num: number, style: ProofNumberStyle) {
  return style === "chinese" ? toChineseNumber(num) : String(num);
}

const CHINESE_NUMBER_MAP: Record<string, number> = {
  零: 0,
  〇: 0,
  一: 1,
  二: 2,
  两: 2,
  三: 3,
  四: 4,
  五: 5,
  六: 6,
  七: 7,
  八: 8,
  九: 9,
};

function parseChineseNumber(text: string): number | null {
  const input = text.replace(/\s+/g, "");
  if (!input) return null;
  if (/^\d+$/.test(input)) return Number.parseInt(input, 10);
  if (!/^[零〇一二两三四五六七八九十百千万]+$/.test(input)) return null;

  let total = 0;
  let section = 0;
  let number = 0;
  for (const char of input) {
    const digit = CHINESE_NUMBER_MAP[char];
    if (digit !== undefined) {
      number = digit;
      continue;
    }

    if (char === "十") {
      section += (number || 1) * 10;
    } else if (char === "百") {
      section += (number || 1) * 100;
    } else if (char === "千") {
      section += (number || 1) * 1000;
    } else if (char === "万") {
      total += (section + number) * 10000;
      section = 0;
    }
    number = 0;
  }

  return total + section + number || null;
}

function parseTitleNumber(original: string): number | null {
  const text = normalizeSpaces(original);
  const standard = text.match(
    /^第\s*([0-9零〇一二两三四五六七八九十百千万]+)\s*(?:章|卷|部|回|节)(?:\s|[:：、.．\-—]|$)/,
  );
  if (standard) return parseChineseNumber(standard[1]);

  const chapter = text.match(/^Chapter\s*(\d+)(?:\s|[:：、.．\-—]|$)/i);
  if (chapter) return Number.parseInt(chapter[1], 10);

  const sequence = text.match(
    /^序列\s*([0-9零〇一二两三四五六七八九十百千万]+)(?:\s|[:：、.．\-—]|$)/,
  );
  if (sequence) return parseChineseNumber(sequence[1]);

  const bracketed = text.match(/^[（(【\[]?\s*(\d{1,5})\s*[）)】\]]?(?:\s*[\.\．、:：\-—]\s*|\s+|(?=[\u4e00-\u9fff]))/);
  if (bracketed) return Number.parseInt(bracketed[1], 10);

  const chinese = text.match(/^([一二三四五六七八九十百千万零〇两]{1,8})(?:\s*[\.\．、:：\-—]\s*|\s+)/);
  if (chinese) return parseChineseNumber(chinese[1]);

  return null;
}

function extractTitleBody(original: string) {
  let text = normalizeSpaces(original);

  const patterns: RegExp[] = [
    /^第\s*[0-9零〇一二两三四五六七八九十百千万]+\s*(?:章|卷|部|回|节)\s*[：:、.．\-—\s]*/i,
    /^Chapter\s*\d+\s*[：:、.．\-—\s]*/i,
    /^序列\s*[0-9零〇一二两三四五六七八九十百千万]+\s*[：:、.．\-—\s]*/i,
    /^\(?\s*[（(【\[]?\s*\d+\s*[）)】\]]?\s*[：:、.．\-—\s]+/,
    /^\(?\s*[一二三四五六七八九十百千万零〇两]+\s*[：:、.．\-—\s]+/,
    /^\d{1,5}\s*[：:、.．\-—\s]+/,
    /^\d{1,5}(?=[\u4e00-\u9fff])\s*/,
  ];

  for (const re of patterns) {
    if (re.test(text)) {
      text = text.replace(re, "");
      break;
    }
  }

  text = text.replace(/^[：:、.．\-—\s]+/, "");
  return text.trim();
}

function isIncludedByScope(
  kind: "volume" | "chapter",
  original: string,
  options: ProofTitleRewriteOptions,
  regex: RegExp | null,
) {
  switch (options.scope) {
    case "volumes":
      return kind === "volume";
    case "chapters":
      return kind === "chapter";
    case "regex":
      return !!regex && regex.test(original);
    case "all":
    default:
      return true;
  }
}

function getNodeKind(node: ProofTocNode): "volume" | "chapter" | null {
  if (node.type === "Volume") return "volume";
  if (node.type === "Chapter") return "chapter";
  return null;
}

function buildReplacementTitle(
  kind: "volume" | "chapter",
  index: number,
  original: string,
  options: ProofTitleRewriteOptions,
) {
  const body = extractTitleBody(original);
  const prefix = kind === "volume" ? "卷" : "章";
  const numberStyle =
    kind === "volume" ? options.volumeNumberStyle : options.chapterNumberStyle;
  const head = `第${formatNumber(index, numberStyle)}${prefix}`;
  return body ? `${head} ${body}` : head;
}

export function buildTitleRewritePreview(
  content: string,
  tocNodes: ProofTocNode[],
  options: ProofTitleRewriteOptions,
): ProofTitlePreviewRow[] {
  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  const rows: ProofTitlePreviewRow[] = [];
  let regex: RegExp | null = null;
  if (options.scope === "regex" && options.regex.trim()) {
    try {
      regex = new RegExp(options.regex, "i");
    } catch {
      return rows;
    }
  }

  if (options.scope === "regex" && !regex) {
    return rows;
  }

  let volumeIndex = 1;
  const chapterCounters = new Map<string, number>();
  const previousOriginalIndexes = new Map<string, number>();
  let currentVolumeKey = "root";

  for (const node of tocNodes) {
    const kind = getNodeKind(node);
    if (!kind) continue;

    const lineIndex = node.line - 1;
    if (lineIndex < 0 || lineIndex >= lines.length) continue;

    const original = lines[lineIndex].trim();
    if (kind === "volume") {
      const volumeKey = node.id || `${node.line}:${node.title}`;
      const included = isIncludedByScope(kind, original, options, regex);
      if (included) {
        const expectedIndex = volumeIndex;
        const replacement = buildReplacementTitle(
          kind,
          expectedIndex,
          original,
          options,
        );
        rows.push({
          id: node.id || `volume:${node.line}`,
          line: node.line,
          kind,
          volumeKey,
          original,
          replacement,
          changed: replacement !== original,
          sequenceBroken: false,
          originalIndex: parseTitleNumber(original),
          expectedIndex,
        });
        volumeIndex++;
      }

      currentVolumeKey = volumeKey;
      if (!chapterCounters.has(currentVolumeKey)) {
        chapterCounters.set(currentVolumeKey, 1);
      }
      continue;
    }

    const included = isIncludedByScope(kind, original, options, regex);
    if (!included) continue;

    const groupKey = options.perVolume
      ? node.parentId || currentVolumeKey
      : "all";
    const nextIndex = chapterCounters.get(groupKey) ?? 1;
    chapterCounters.set(groupKey, nextIndex + 1);

    const replacement = buildReplacementTitle(
      kind,
      nextIndex,
      original,
      options,
    );
    const originalIndex = parseTitleNumber(original);
    const previousOriginalIndex = previousOriginalIndexes.get(groupKey);
    const sequenceBroken =
      originalIndex !== null &&
      (previousOriginalIndex === undefined
        ? originalIndex !== nextIndex
        : originalIndex !== previousOriginalIndex + 1);
    if (originalIndex !== null) {
      previousOriginalIndexes.set(groupKey, originalIndex);
    }
    rows.push({
      id: node.id || `chapter:${node.line}`,
      line: node.line,
      kind,
      volumeKey: currentVolumeKey,
      original,
      replacement,
      changed: replacement !== original,
      sequenceBroken,
      originalIndex,
      expectedIndex: nextIndex,
    });
  }

  return rows;
}

export function applyTitleRewrite(
  content: string,
  tocNodes: ProofTocNode[],
  options: ProofTitleRewriteOptions,
): ProofTransformResult {
  const preview = buildTitleRewritePreview(content, tocNodes, options);
  if (preview.length === 0) {
    return {
      text: content,
      changedCount: 0,
      message: "没有可重排的标题",
    };
  }

  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  let changedCount = 0;

  for (const row of preview) {
    if (!row.changed) continue;
    const lineIndex = row.line - 1;
    if (lineIndex < 0 || lineIndex >= lines.length) continue;
    const currentLine = lines[lineIndex];
    const indent = currentLine.match(/^[\s　]*/)?.[0] ?? "";
    lines[lineIndex] = `${indent}${row.replacement}`;
    changedCount++;
  }

  return {
    text: lines.join("\n"),
    changedCount,
    message: changedCount > 0 ? `已重排 ${changedCount} 个标题` : "标题已是标准格式",
  };
}

export function removeBracketContentFromTitles(
  content: string,
  tocNodes: ProofTocNode[],
): ProofTransformResult {
  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  let changedCount = 0;

  for (const node of tocNodes) {
    const lineIndex = node.line - 1;
    if (lineIndex < 0 || lineIndex >= lines.length) continue;
    const currentLine = lines[lineIndex];
    const cleaned = cleanTitleBracketsLine(currentLine);

    if (cleaned !== currentLine) {
      lines[lineIndex] = cleaned;
      changedCount++;
    }
  }

  return {
    text: lines.join("\n"),
    changedCount,
    message: changedCount > 0 ? `已清理 ${changedCount} 个标题括号` : "没有找到括号内容",
  };
}

export function removeCommonAds(content: string): ProofTransformResult {
  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  const output: string[] = [];
  let changedCount = 0;
  let skippingBlock = false;

  for (const line of lines) {
    const trimmed = line.trim();
    if (skippingBlock) {
      if (isTitleLikeLine(trimmed)) {
        skippingBlock = false;
        output.push(line);
      } else {
        changedCount++;
      }
      continue;
    }

    if (!trimmed) {
      output.push(line);
      continue;
    }

    if (AD_BLOCK_START_RE.test(trimmed)) {
      skippingBlock = true;
      changedCount++;
      continue;
    }

    if (AD_LINE_RE.test(trimmed)) {
      changedCount++;
      continue;
    }

    output.push(line);
  }

  return {
    text: output.join("\n"),
    changedCount,
    message:
      changedCount > 0 ? `已移除 ${changedCount} 行广告/说明` : "没有找到广告内容",
  };
}

export function removePinyin(content: string, tocNodes: ProofTocNode[]): ProofTransformResult {
  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  let changedCount = 0;

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const trimmed = line.trim();
    if (!trimmed) continue;

    if (!CHINESE_CHAR_RE.test(trimmed)) {
      if (isPinyinLike(trimmed)) {
        lines[i] = "";
        changedCount++;
      }
      continue;
    }

    const cleaned = cleanPinyinLine(line);

    if (cleaned !== line) {
      lines[i] = cleaned;
      changedCount++;
    }
  }

  // 额外清理标题行里的拼音尾注。
  for (const node of tocNodes) {
    const lineIndex = node.line - 1;
    if (lineIndex < 0 || lineIndex >= lines.length) continue;
    const currentLine = lines[lineIndex];
    if (!CHINESE_CHAR_RE.test(currentLine)) continue;
    const cleaned = cleanPinyinLine(currentLine);
    if (cleaned !== currentLine) {
      lines[lineIndex] = cleaned;
      changedCount++;
    }
  }

  return {
    text: lines.join("\n"),
    changedCount,
    message: changedCount > 0 ? `已清理 ${changedCount} 处拼音` : "没有找到拼音内容",
  };
}

export function buildBuiltinRegexPreview(
  content: string,
  ruleId: ProofBuiltinRuleId,
  tocNodes: ProofTocNode[] = [],
): ProofRegexPreviewRow[] {
  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  const rows: ProofRegexPreviewRow[] = [];
  const pushRow = (
    lineStart: number,
    lineEnd: number,
    original: string,
    replacement: string,
  ) => {
    rows.push({
      id: `${ruleId}:${lineStart}:${lineEnd}:${rows.length}`,
      ruleId,
      lineStart,
      lineEnd,
      original,
      replacement,
    });
  };

  if (ruleId === "title-brackets") {
    const seenLines = new Set<number>();
    for (const node of tocNodes) {
      const lineNumber = node.line;
      const lineIndex = lineNumber - 1;
      if (seenLines.has(lineNumber) || lineIndex < 0 || lineIndex >= lines.length) continue;
      seenLines.add(lineNumber);
      const original = lines[lineIndex];
      const replacement = cleanTitleBracketsLine(original);
      if (replacement !== original) {
        pushRow(lineNumber, lineNumber, original, replacement);
      }
    }
    return rows;
  }

  if (ruleId === "ads") {
    let index = 0;
    while (index < lines.length) {
      const line = lines[index];
      const trimmed = line.trim();
      if (!trimmed) {
        index++;
        continue;
      }

      if (AD_BLOCK_START_RE.test(trimmed)) {
        const start = index;
        let end = index;
        index++;
        while (index < lines.length && !isTitleLikeLine(lines[index].trim())) {
          end = index;
          index++;
        }
        pushRow(start + 1, end + 1, lines.slice(start, end + 1).join("\n"), "");
        continue;
      }

      if (AD_LINE_RE.test(trimmed)) {
        pushRow(index + 1, index + 1, line, "");
      }
      index++;
    }
    return rows;
  }

  for (let index = 0; index < lines.length; index++) {
    const original = lines[index];
    const replacement = cleanPinyinLine(original);
    if (replacement !== original) {
      pushRow(index + 1, index + 1, original, replacement);
    }
  }

  return rows;
}

export function applyBuiltinRegexPreview(
  content: string,
  rows: ProofRegexPreviewRow[],
  selectedIds: Set<string> | string[],
): ProofTransformResult {
  const selected = selectedIds instanceof Set ? selectedIds : new Set(selectedIds);
  const targets = rows
    .filter((row) => selected.has(row.id))
    .sort((a, b) => b.lineStart - a.lineStart);

  if (targets.length === 0) {
    return {
      text: content,
      changedCount: 0,
      message: "请选择要替换的匹配项",
    };
  }

  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");
  let changedCount = 0;

  for (const row of targets) {
    const startIndex = row.lineStart - 1;
    const deleteCount = row.lineEnd - row.lineStart + 1;
    if (startIndex < 0 || startIndex >= lines.length || deleteCount <= 0) continue;
    if (row.replacement) {
      lines.splice(startIndex, deleteCount, row.replacement);
    } else {
      lines.splice(startIndex, deleteCount);
    }
    changedCount++;
  }

  return {
    text: lines.join("\n"),
    changedCount,
    message: changedCount > 0 ? `已替换 ${changedCount} 个匹配项` : "没有可替换的匹配项",
  };
}

export async function convertChineseText(
  content: string,
  direction: ProofConvertDirection,
): Promise<ProofTransformResult> {
  const converter = await getChineseConverter(direction);
  const next = converter(content);
  let changedCount = 0;
  const len = Math.min(content.length, next.length);
  for (let i = 0; i < len; i++) {
    if (content[i] !== next[i]) changedCount++;
  }
  changedCount += Math.abs(content.length - next.length);

  return {
    text: next,
    changedCount,
    message:
      changedCount > 0
        ? direction === "traditional-to-simplified"
          ? "已转换为简体"
          : "已转换为繁体"
        : "没有需要转换的内容",
  };
}

function replaceWithCount(input: string, regex: RegExp, replacement: string) {
  regex.lastIndex = 0;
  const changedCount = Array.from(input.matchAll(regex)).length;
  regex.lastIndex = 0;
  if (changedCount === 0) {
    return { text: input, changedCount };
  }

  const text = input.replace(regex, replacement);
  regex.lastIndex = 0;
  return { text, changedCount };
}

export function replaceByRegex(
  content: string,
  pattern: string,
  replacement: string,
  options: {
    caseSensitive?: boolean;
    targetTitles?: boolean;
    tocNodes?: ProofTocNode[];
  } = {},
): ProofTransformResult {
  if (!pattern.trim()) {
    return {
      text: content,
      changedCount: 0,
      message: "请输入正则表达式",
    };
  }

  const flags = `g${options.caseSensitive ? "" : "i"}`;
  let regex: RegExp;
  try {
    regex = new RegExp(pattern, flags);
  } catch (error: any) {
    return {
      text: content,
      changedCount: 0,
      message: `正则错误: ${error?.message || error}`,
    };
  }

  const normalized = content.replace(/\r\n|\r|\u2028|\u2029/g, "\n");
  const lines = normalized.split("\n");

  if (options.targetTitles) {
    const tocNodes = options.tocNodes ?? [];
    let changedCount = 0;

    for (const node of tocNodes) {
      const lineIndex = node.line - 1;
      if (lineIndex < 0 || lineIndex >= lines.length) continue;
      const currentLine = lines[lineIndex];
      const { text: next, changedCount: localCount } = replaceWithCount(
        currentLine,
        regex,
        replacement,
      );
      if (localCount > 0) {
        lines[lineIndex] = next;
        changedCount += localCount;
      }
    }

    return {
      text: lines.join("\n"),
      changedCount,
      message: changedCount > 0 ? `已替换 ${changedCount} 处标题` : "没有匹配到标题",
    };
  }

  const { text: next, changedCount } = replaceWithCount(
    normalized,
    regex,
    replacement,
  );

  return {
    text: next,
    changedCount,
    message: changedCount > 0 ? `已替换 ${changedCount} 处` : "没有匹配到内容",
  };
}

export function buildProofPreviewSummary(rows: ProofTitlePreviewRow[]) {
  const changed = rows.filter((row) => row.changed).length;
  return {
    total: rows.length,
    changed,
  };
}
