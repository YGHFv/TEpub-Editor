/**
 * 查找替换纯逻辑公共核心（docs/REFACTORING.md 第 2 期第 1 条）。
 *
 * 目标：让 editor / epub-editor / toolbox/epub-editor / search-replace
 * 各自内联的模式构建、计数、替换逻辑逐个切换到同一套实现。
 * 本模块保持纯函数：无 DOM、无 CodeMirror、无 Tauri 依赖。
 */

export type FindMode = "text" | "regex";

export type SearchDirection = "down" | "up";

export interface FindQuery {
  /** 用户输入的查找内容。text 模式按字面量匹配，regex 模式按 JS 正则解析。 */
  pattern: string;
  mode: FindMode;
  /** 是否区分大小写，默认 true（与现有各实现一致）。 */
  caseSensitive?: boolean;
  /** 全词匹配（\b 词边界）。text 模式包裹转义后的字面量，regex 模式包裹 (?:...)。 */
  wholeWord?: boolean;
}

export interface MatchRange {
  /** 匹配起始偏移（含）。 */
  from: number;
  /** 匹配结束偏移（不含）。 */
  to: number;
}

export interface ReplaceAllResult {
  content: string;
  count: number;
}

/**
 * 把字面量文本转义成可安全嵌入正则的形式。
 *
 * 只转义正则语法字符（SyntaxCharacter），产物在带 u 标志的方言
 * （如 CodeMirror SearchQuery 内部使用的 "gmu"）下同样合法——
 * 不要额外转义 `-`、`/` 等普通字符，`\-` 在 u 模式下是 SyntaxError。
 */
export function escapeRegExpLiteral(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * search-replace 窗口“扩展(\n,\r,\t)”查找模式的转义展开：
 * 把输入里的字面量 `\n` / `\t` / `\r` 展开成换行、制表、回车，
 * 查找串与替换串都经过同一展开。
 *
 * 语义严格沿用既有内联实现：按 `\n` → `\t` → `\r` 顺序整串替换，
 * 不支持 `\\` 反转义（例如输入 `\\n` 会展开成 `\` + 换行），
 * 其余转义序列（`\s`、`\d` 等）原样保留。
 */
export function expandExtendedEscapes(text: string): string {
  return text.replace(/\\n/g, "\n").replace(/\\t/g, "\t").replace(/\\r/g, "\r");
}

/**
 * 构建查找用正则（始终带 g 标志）。
 * regex 模式下 pattern 非法时抛出原生 SyntaxError，
 * 调用方沿用既有的 try/catch → “正则表达式错误” 提示路径。
 */
export function buildFindRegex(query: FindQuery): RegExp {
  let source = query.mode === "regex" ? query.pattern : escapeRegExpLiteral(query.pattern);
  if (query.wholeWord) {
    source = query.mode === "regex" ? `\\b(?:${source})\\b` : `\\b${source}\\b`;
  }
  const flags = query.caseSensitive === false ? "gi" : "g";
  return new RegExp(source, flags);
}

/**
 * text 模式且区分大小写且非全词时走 indexOf 快路径，
 * 保持既有实现的字面量搜索语义（尤其 up 方向的边界行为，见 findNextMatch）。
 */
function isPlainTextQuery(query: FindQuery): boolean {
  return query.mode === "text" && query.caseSensitive !== false && !query.wholeWord;
}

/** 列出全部（非重叠）匹配；零宽匹配自动前进一位避免死循环。 */
export function findAllMatches(content: string, query: FindQuery): MatchRange[] {
  if (!query.pattern) return [];
  const ranges: MatchRange[] = [];
  if (isPlainTextQuery(query)) {
    let index = 0;
    while ((index = content.indexOf(query.pattern, index)) !== -1) {
      ranges.push({ from: index, to: index + query.pattern.length });
      index += query.pattern.length;
    }
    return ranges;
  }
  const regex = buildFindRegex(query);
  let match: RegExpExecArray | null;
  while ((match = regex.exec(content))) {
    const from = match.index;
    const to = from + match[0].length;
    if (to === from) regex.lastIndex += 1;
    ranges.push({ from, to });
  }
  return ranges;
}

/** 单文件匹配计数。 */
export function countMatchesInContent(content: string, query: FindQuery): number {
  return findAllMatches(content, query).length;
}

/**
 * 从 start 偏移查找下一个匹配。
 *
 * 语义沿用既有实现（toolbox/epub-editor）：
 * - down：第一个 from >= start 的匹配；
 * - up（regex 路径）：最后一个 from < start 的匹配（严格小于）；
 * - up（字面量快路径）：lastIndexOf(pattern, max(0, start))，即 from <= max(0, start)。
 *
 * regex 与字面量在 up 方向存在历史上的边界不一致（严格小于 vs 小于等于），
 * 为保持行为兼容暂不统一，待其余实现切换完成后一并处理。
 */
export function findNextMatch(
  content: string,
  query: FindQuery,
  start: number,
  direction: SearchDirection,
): MatchRange | null {
  if (!query.pattern) return null;
  if (isPlainTextQuery(query)) {
    const index =
      direction === "up"
        ? content.lastIndexOf(query.pattern, Math.max(0, start))
        : content.indexOf(query.pattern, Math.max(0, start));
    return index >= 0 ? { from: index, to: index + query.pattern.length } : null;
  }
  const matches = findAllMatches(content, query);
  if (direction === "up") {
    return [...matches].reverse().find((item) => item.from < start) ?? null;
  }
  return matches.find((item) => item.from >= start) ?? null;
}

/**
 * 单文件全部替换。
 * - regex 模式：replacement 支持 $1/$& 等替换模板；
 * - text 模式：replacement 始终按字面量插入（$ 无特殊含义），
 *   即使因大小写/全词开关内部走了正则路径也保持字面量语义。
 */
export function replaceAllInContent(
  content: string,
  query: FindQuery,
  replacement: string,
): ReplaceAllResult {
  if (!query.pattern) return { content, count: 0 };
  if (isPlainTextQuery(query)) {
    let count = 0;
    let index = 0;
    while ((index = content.indexOf(query.pattern, index)) !== -1) {
      count += 1;
      index += query.pattern.length;
    }
    return { content: content.split(query.pattern).join(replacement), count };
  }
  const count = countMatchesInContent(content, query);
  const regex = buildFindRegex(query);
  const next =
    query.mode === "regex"
      ? content.replace(regex, replacement)
      : content.replace(regex, () => replacement);
  return { content: next, count };
}

export type SearchScope = "current" | "open" | "html" | "selected" | "all";

export interface ScopeFileSource<T> {
  /** 文档内全部文件。 */
  allFiles: readonly T[];
  /** 当前打开的文件（current/open 范围的目标，也是 selected 范围为空时的回退）。 */
  currentFile: T | null;
  /** 多选的文件（selected 范围），由调用方从路径解析为文件对象。 */
  selectedFiles?: readonly T[];
  isHtml: (file: T) => boolean;
  isEditable: (file: T) => boolean;
}

/** 计算给定搜索范围内应参与查找/替换的文件列表。 */
export function filterFilesInScope<T>(scope: SearchScope, source: ScopeFileSource<T>): T[] {
  const current = source.currentFile === null ? [] : [source.currentFile];
  switch (scope) {
    case "current":
    case "open":
      return current;
    case "html":
      return source.allFiles.filter((file) => source.isHtml(file) && source.isEditable(file));
    case "selected": {
      const selected = (source.selectedFiles ?? []).filter((file) => source.isEditable(file));
      return selected.length > 0 ? selected : current;
    }
    case "all":
      return source.allFiles.filter((file) => source.isEditable(file));
  }
}
