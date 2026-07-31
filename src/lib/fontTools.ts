/**
 * 字体管理纯逻辑公共核心（docs/REFACTORING.md 第 2 期第 4 条）。
 *
 * 从 epub-editor 页面抽出的字体预览 / 字形统计相关纯函数：
 * 字体文件识别、CSS font-family 与 @font-face 解析、字符使用计数、
 * analyze_epub_font_glyphs 结果加工、预览用 font-family 名生成。
 * 本模块保持纯函数：无 DOM、无 Tauri 依赖、不持有组件状态。
 */

/** Rust 命令 analyze_epub_font_glyphs 的返回结构。 */
export interface FontGlyphAnalyzeResult {
  internal_names: string[];
  missing_chars: string[];
  unsupported_reason: string | null;
}

/** 字形统计汇总结果（按文中出现次数计数）。 */
export interface FontUsageSummary {
  /** 使用该字体渲染的非空白字符总数（含重复出现）。 */
  usedCount: number;
  /** 其中字体缺失字形的字符出现次数。 */
  missingCount: number;
  /** 缺失字形的去重字符列表（保持后端返回顺序）。 */
  missingChars: string[];
}

/** 字体预览默认示例文本。 */
export const DEFAULT_FONT_PREVIEW_TEXT =
  "ABCDEFGHIJKLMNOPQRSTUVWXYZ\nabcdefghijklmnopqrstuvwxyz\n春江潮水连海平，海上明月共潮生。\n永和九年，岁在癸丑，暮春之初。\n天地玄黄 宇宙洪荒 日月盈昃 辰宿列张\n1234567890 ~!@#$%^&*()";

/** 按扩展名判断是否为字体文件。 */
export function isFontFile(name: string): boolean {
  return /\.(ttf|otf|woff|woff2|eot)$/i.test(name);
}

/** 去掉文件名的最后一个扩展名（用作 font-family 别名）。 */
export function fontFileStem(fileName: string): string {
  return fileName.replace(/\.[^.]+$/, "");
}

/**
 * 二进制资源预览用的 MIME 识别（字体 + 图片），未知扩展名回退 octet-stream。
 * 与字体预览共用：Blob URL 的 type 会影响部分 WebView 的字体解码。
 */
export function getBinaryMimeType(fileName: string): string {
  const lower = fileName.toLowerCase();
  if (lower.endsWith(".png")) return "image/png";
  if (lower.endsWith(".gif")) return "image/gif";
  if (lower.endsWith(".svg")) return "image/svg+xml";
  if (lower.endsWith(".webp")) return "image/webp";
  if (lower.endsWith(".woff2")) return "font/woff2";
  if (lower.endsWith(".woff")) return "font/woff";
  if (lower.endsWith(".ttf")) return "font/ttf";
  if (lower.endsWith(".otf")) return "font/otf";
  if (lower.endsWith(".eot")) return "application/vnd.ms-fontobject";
  return "application/octet-stream";
}

/** 统一 zip 内路径：反斜杠转正斜杠并去掉开头的 "/"。 */
export function normalizeZipPath(path: string): string {
  return path.replace(/\\/g, "/").replace(/^\/+/, "");
}

/** 去掉 URL 的 ?query 与 #hash 部分。 */
export function stripQueryAndHash(path: string): string {
  return path.split("#")[0].split("?")[0];
}

/** 以 basePath 所在目录为基准解析相对路径（"." / ".." 归一化）。 */
export function resolveRelativePath(
  basePath: string,
  relativePath: string,
): string {
  const stack = basePath.split("/");
  stack.pop();

  const parts = relativePath.split("/");
  for (const part of parts) {
    if (part === ".") continue;
    if (part === "..") {
      if (stack.length > 0) stack.pop();
    } else {
      stack.push(part);
    }
  }
  return stack.join("/");
}

/** 把 HTML/CSS 中的相对引用解析成 zip 内规范路径（去 query/hash、统一分隔符）。 */
export function resolveCssHrefToZipPath(
  basePath: string,
  href: string,
): string {
  return normalizeZipPath(
    stripQueryAndHash(resolveRelativePath(basePath, href)),
  );
}

/** 规范化单个 font-family 名：去首尾引号、trim、转小写。 */
export function normalizeFontFamilyName(name: string): string {
  return name
    .replace(/^['"]|['"]$/g, "")
    .trim()
    .toLowerCase();
}

/** 解析 font-family 属性值为规范化的 family 列表（引号内的逗号不分隔）。 */
export function parseFontFamilyList(value: string): string[] {
  const chunks = value.match(/\"[^\"]*\"|'[^']*'|[^,]+/g) || [];
  return chunks.map((item) => normalizeFontFamilyName(item)).filter(Boolean);
}

/** 从 CSS 规则体中提取 font-family 声明的原始值（大小写不敏感）。 */
function matchFontFamilyValue(cssBody: string): string | null {
  const match = cssBody.match(/font-family\s*:\s*([^;]+);?/i);
  return match ? match[1] : null;
}

/**
 * 收集 CSS 中 font-family 命中目标字体族的普通规则选择器
 * （逐个拆开逗号分隔的选择器，跳过 @ 规则）。
 */
export function collectSelectorsUsingFont(
  cssContent: string,
  targetFamilies: ReadonlySet<string>,
): string[] {
  const selectors: string[] = [];
  const ruleRegex = /([^{}]+)\{([^{}]*)\}/g;
  let match: RegExpExecArray | null;

  while ((match = ruleRegex.exec(cssContent)) !== null) {
    const selectorPart = match[1].trim();
    const body = match[2];
    if (!selectorPart || selectorPart.startsWith("@")) continue;

    const familyValue = matchFontFamilyValue(body);
    if (!familyValue) continue;

    const families = parseFontFamilyList(familyValue);
    if (!families.some((f) => targetFamilies.has(f))) continue;

    selectorPart
      .split(",")
      .map((s) => s.trim())
      .filter(Boolean)
      .forEach((s) => selectors.push(s));
  }

  return selectors;
}

/**
 * 在 CSS 里找出 src 指向目标字体文件的 @font-face 声明，
 * 返回其声明的（规范化）font-family 列表。
 * data: 与 http(s) 的 src 不参与匹配；相对 url 以 cssPath 为基准解析。
 */
export function collectFontFaceFamiliesByFontPath(
  cssPath: string,
  cssContent: string,
  targetFontPath: string,
): string[] {
  const result: string[] = [];
  const target = normalizeZipPath(stripQueryAndHash(targetFontPath));
  const faceRegex = /@font-face\s*\{([\s\S]*?)\}/gi;
  let faceMatch: RegExpExecArray | null;

  while ((faceMatch = faceRegex.exec(cssContent)) !== null) {
    const block = faceMatch[1];
    const srcMatch = block.match(/src\s*:\s*([^;]+);?/i);
    const familyValue = matchFontFamilyValue(block);
    if (!srcMatch || !familyValue) continue;

    const srcValue = srcMatch[1];
    const urls = [...srcValue.matchAll(/url\(([^)]+)\)/gi)].map((m) =>
      m[1].trim().replace(/^['"]|['"]$/g, ""),
    );

    const matched = urls.some((u) => {
      if (!u || u.startsWith("data:") || u.startsWith("http")) {
        return false;
      }
      return resolveCssHrefToZipPath(cssPath, u) === target;
    });

    if (!matched) continue;

    parseFontFamilyList(familyValue).forEach((family) => {
      if (family) result.push(family);
    });
  }

  return result;
}

export interface FontFamilyAliasOptions {
  /** 预览时选定的主 family 名。 */
  fontFamily: string;
  /** 字体文件名（其去扩展名形式也视为别名）。 */
  fontFileName: string;
  /** 字体内部名等额外别名。 */
  extraAliases?: string[];
  /** css 路径 → 内容，用于扫描 @font-face 关联的 family。 */
  cssContents?: Record<string, string>;
  /** 字体文件在 zip 内的路径，配合 cssContents 使用。 */
  fontPath?: string;
}

/**
 * 汇总一款字体可能对应的全部（规范化）font-family 别名：
 * 主 family、文件名去扩展名、额外别名、以及各 CSS 中
 * src 指向该字体文件的 @font-face 声明的 family。
 */
export function buildFontFamilyAliases(
  options: FontFamilyAliasOptions,
): Set<string> {
  const aliases = new Set<string>();
  aliases.add(normalizeFontFamilyName(options.fontFamily));
  aliases.add(normalizeFontFamilyName(fontFileStem(options.fontFileName)));
  (options.extraAliases || []).forEach((name) => {
    aliases.add(normalizeFontFamilyName(name));
  });

  if (options.cssContents && options.fontPath) {
    for (const [cssPath, cssContent] of Object.entries(options.cssContents)) {
      collectFontFaceFamiliesByFontPath(
        cssPath,
        cssContent || "",
        options.fontPath,
      ).forEach((family) => aliases.add(normalizeFontFamilyName(family)));
    }
  }

  return aliases;
}

/** 判断元素内联 style 的 font-family 是否命中目标字体族。 */
export function styleAttributeUsesFont(
  styleValue: string,
  targetFamilies: ReadonlySet<string>,
): boolean {
  const familyValue = matchFontFamilyValue(styleValue);
  if (!familyValue) return false;
  return parseFontFamilyList(familyValue).some((f) => targetFamilies.has(f));
}

/** 把一段文本的非空白字符逐个累加进使用计数表（Unicode 空白按 \s 判定）。 */
export function accumulateCharUsage(
  text: string,
  usageMap: Map<string, number>,
): void {
  for (const ch of text) {
    if (/\s/u.test(ch)) continue;
    usageMap.set(ch, (usageMap.get(ch) || 0) + 1);
  }
}

/**
 * 汇总字形统计：usedCount 为全部计数之和，
 * missingCount 只累计后端报告缺字形的字符出现次数，
 * missingChars 为后端缺字形字符的去重列表（与页面展示一致）。
 */
export function summarizeGlyphUsage(
  usageMap: ReadonlyMap<string, number>,
  missingChars: readonly string[],
): FontUsageSummary {
  const missingSet = new Set(missingChars);
  let usedCount = 0;
  let missingCount = 0;
  for (const [ch, count] of usageMap.entries()) {
    usedCount += count;
    if (missingSet.has(ch)) {
      missingCount += count;
    }
  }
  return {
    usedCount,
    missingCount,
    missingChars: Array.from(missingSet.values()),
  };
}

/** 生成预览用的一次性 font-family 名（避免与文档内已有 family 冲突）。 */
export function createFontPreviewFamily(): string {
  return (
    "tepub-font-" + Date.now() + "-" + Math.random().toString(36).slice(2, 8)
  );
}
