import JSZip from "jszip";
import type { TTF } from "fonteditor-core";
import { deflate, inflate } from "pako";
import { rewriteWebEpubTextLinks } from "$lib/webEpubProcess";

export type WebEpubFontAction = "font-encrypt" | "font-decrypt" | "font-subset";

export type WebEpubFontProcessResult = {
  sourceName: string;
  outputName: string;
  action: WebEpubFontAction;
  changedFiles: number;
  mappedCharacters: number;
  changedFonts: number;
  mode: string;
  message: string;
  blob: Blob;
};

type ParsedFont = {
  path: string;
  type: "ttf" | "otf" | "woff" | "woff2";
  font: any;
  glyphs: TTF.Glyph[];
};

const MAP_PATH = "META-INF/tepub-font-obfuscation.json";
const FONT_EXTENSIONS = new Set(["ttf", "otf", "woff", "woff2"]);
const HTML_EXTENSIONS = new Set(["xhtml", "html", "htm"]);
const TEXT_EXTENSIONS = new Set(["xhtml", "html", "htm", "xml", "opf", "ncx", "css", "svg"]);
const WOFF2_WASM_URL = new URL("../../node_modules/fonteditor-core/woff2/woff2.wasm", import.meta.url).href;
let woff2Ready: Promise<unknown> | null = null;
let fontRuntime: Promise<typeof import("fonteditor-core")> | null = null;

function loadFontRuntime() {
  fontRuntime ||= import("fonteditor-core");
  return fontRuntime;
}

function extension(path: string) {
  return path.split(".").pop()?.toLowerCase() || "";
}

function parentPath(path: string) {
  const index = path.lastIndexOf("/");
  return index >= 0 ? path.slice(0, index) : "";
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

function joinPath(base: string, relative: string) {
  return relative.startsWith("/") ? normalizePath(relative.slice(1)) : normalizePath(`${base}/${relative}`);
}

function percentDecode(value: string) {
  try { return decodeURIComponent(value); } catch { return value; }
}

function elementsByLocalName(root: ParentNode, name: string) {
  return Array.from(root.querySelectorAll("*")).filter((element) => element.localName === name) as Element[];
}

function parseXml(source: string, label: string) {
  const document = new DOMParser().parseFromString(source, "application/xml");
  if (document.querySelector("parsererror")) throw new Error(`${label} XML 格式无效`);
  return document;
}

function isPrivateCodePoint(code: number) {
  return (code >= 0xe000 && code <= 0xf8ff) || (code >= 0xf0000 && code <= 0xffffd) || (code >= 0x100000 && code <= 0x10fffd);
}

function isCjkCodePoint(code: number) {
  return (code >= 0x3400 && code <= 0x4dbf)
    || (code >= 0x4e00 && code <= 0x9fff)
    || (code >= 0xf900 && code <= 0xfaff)
    || (code >= 0x20000 && code <= 0x3ffff);
}

function shouldMapCharacter(character: string) {
  const code = character.codePointAt(0) || 0;
  return isCjkCodePoint(code) && !/[<>&]/.test(character);
}

function isMappingCandidate(character: string) {
  const code = character.codePointAt(0) || 0;
  return isCjkCodePoint(code) || isPrivateCodePoint(code);
}

function transformHtmlText(source: string, mapping: Map<string, string>) {
  let output = "";
  let rawTag: "script" | "style" | null = null;
  for (let index = 0; index < source.length;) {
    if (source[index] === "<") {
      const end = source.indexOf(">", index + 1);
      if (end < 0) { output += source.slice(index); break; }
      const tag = source.slice(index, end + 1);
      output += tag;
      const match = tag.match(/^<\s*(\/?)\s*([\w:-]+)/);
      const closing = match?.[1] === "/";
      const name = match?.[2]?.toLowerCase();
      if (name === "script" || name === "style") rawTag = closing ? null : name;
      index = end + 1;
      continue;
    }
    if (!rawTag && source[index] === "&") {
      const end = source.indexOf(";", index + 1);
      if (end > index && end - index < 16) { output += source.slice(index, end + 1); index = end + 1; continue; }
    }
    const code = source.codePointAt(index) || 0;
    const character = String.fromCodePoint(code);
    output += rawTag ? character : mapping.get(character) || character;
    index += character.length;
  }
  return output;
}

function collectHtmlCharacters(source: string) {
  const characters: string[] = [];
  let rawTag: "script" | "style" | null = null;
  for (let index = 0; index < source.length;) {
    if (source[index] === "<") {
      const end = source.indexOf(">", index + 1);
      if (end < 0) break;
      const tag = source.slice(index, end + 1);
      const match = tag.match(/^<\s*(\/?)\s*([\w:-]+)/);
      const closing = match?.[1] === "/";
      const name = match?.[2]?.toLowerCase();
      if (name === "script" || name === "style") rawTag = closing ? null : name;
      index = end + 1;
      continue;
    }
    if (!rawTag && source[index] === "&") {
      const end = source.indexOf(";", index + 1);
      if (end > index && end - index < 16) { index = end + 1; continue; }
    }
    const code = source.codePointAt(index) || 0;
    const character = String.fromCodePoint(code);
    if (!rawTag && shouldMapCharacter(character)) characters.push(character);
    index += character.length;
  }
  return characters;
}

function sourceNameWithSuffix(sourceName: string, suffix: string) {
  const stem = sourceName.toLowerCase().endsWith(".epub") ? sourceName.slice(0, -5) : sourceName;
  return `${stem}${suffix}.epub`;
}

async function ensureWoff2() {
  const { woff2 } = await loadFontRuntime();
  if (!woff2Ready) woff2Ready = woff2.init(WOFF2_WASM_URL);
  await woff2Ready;
}

async function parseFont(path: string, bytes: ArrayBuffer, subset?: number[]): Promise<ParsedFont> {
  const ext = extension(path);
  if (!FONT_EXTENSIONS.has(ext)) throw new Error(`不支持的字体格式：${path}`);
  if (ext === "woff2") await ensureWoff2();
  const options: any = { type: ext, hinting: true, kerning: true, compound2simple: false };
  if (subset?.length) options.subset = subset;
  if (ext === "woff") options.inflate = inflate;
  const { createFont } = await loadFontRuntime();
  const font = createFont(bytes, options);
  return { path, type: ext as ParsedFont["type"], font, glyphs: font.get().glyf || [] };
}

function writeFont(parsed: ParsedFont) {
  const outputType = parsed.type === "otf" ? "ttf" : parsed.type;
  const options: any = { type: outputType, hinting: true, kerning: true };
  if (outputType === "woff") options.deflate = deflate;
  return { bytes: parsed.font.write(options) as ArrayBuffer, outputType };
}

function collectFontPlainCodePoints(fonts: ParsedFont[]) {
  const supported = new Set<number>();
  for (const parsed of fonts) {
    for (const glyph of parsed.glyphs) {
      for (const code of glyph.unicode || []) if (isCjkCodePoint(code)) supported.add(code);
    }
  }
  return supported;
}

function collectFontPrivateCodePoints(fonts: ParsedFont[]) {
  const occupied = new Set<number>();
  for (const parsed of fonts) {
    for (const glyph of parsed.glyphs) {
      for (const code of glyph.unicode || []) if (isPrivateCodePoint(code)) occupied.add(code);
    }
  }
  return occupied;
}

function splitFontFamilies(value: string) {
  const families: string[] = [];
  let current = "";
  let quote = "";
  for (const character of value) {
    if (quote) {
      current += character;
      if (character === quote) quote = "";
    } else if (character === "\"" || character === "'") {
      quote = character;
      current += character;
    } else if (character === ",") {
      if (current.trim()) families.push(current.trim());
      current = "";
    } else {
      current += character;
    }
  }
  if (current.trim()) families.push(current.trim());
  return families;
}

function unquotedFontFamily(value: string) {
  return value.trim().replace(/^(['"])(.*)\1$/, "$2");
}

function normalizedFontFamily(value: string) {
  return unquotedFontFamily(value).toLowerCase();
}

function cssFontFamily(value: string) {
  return `"${value.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}

function appendFontFallbacks(value: string, fallbackFamilies: string[]) {
  const important = value.match(/\s*!important\s*$/i)?.[0] || "";
  const families = splitFontFamilies(important ? value.slice(0, -important.length) : value);
  const existing = new Set(families.map(normalizedFontFamily));
  const additions = fallbackFamilies.filter((family) => !existing.has(family.toLowerCase())).map(cssFontFamily);
  if (!additions.length) return value;
  const genericFamilies = new Set(["serif", "sans-serif", "monospace", "cursive", "fantasy", "system-ui", "ui-serif", "ui-sans-serif", "ui-monospace", "ui-rounded", "emoji", "math", "fangsong"]);
  const genericIndex = families.findIndex((family) => genericFamilies.has(normalizedFontFamily(family)));
  families.splice(genericIndex < 0 ? families.length : genericIndex, 0, ...additions);
  return `${families.join(", ")}${important}`;
}

function rewriteCssFontFallbacks(source: string, fallbackFamilies: string[]) {
  if (!fallbackFamilies.length) return source;
  const fontFaces: string[] = [];
  const masked = source.replace(/@font-face\s*\{[^{}]*\}/gi, (block) => {
    const marker = `__TEPUB_FONT_FACE_${fontFaces.length}__`;
    fontFaces.push(block);
    return marker;
  });
  const rewritten = masked.replace(/(font-family\s*:\s*)([^;}{]+)/gi, (_match, prefix: string, value: string) => (
    `${prefix}${appendFontFallbacks(value, fallbackFamilies)}`
  ));
  return rewritten.replace(/__TEPUB_FONT_FACE_(\d+)__/g, (_match, index: string) => fontFaces[Number(index)] || "");
}

function rewriteInlineFontFallbacks(source: string, fallbackFamilies: string[]) {
  if (!fallbackFamilies.length) return source;
  return source.replace(/(style\s*=\s*)(['"])(.*?)\2/gi, (_match, prefix: string, quote: string, style: string) => {
    const rewritten = style.replace(/(font-family\s*:\s*)([^;]+)/gi, (_declaration, declarationPrefix: string, value: string) => (
      `${declarationPrefix}${appendFontFallbacks(value, fallbackFamilies)}`
    ));
    return `${prefix}${quote}${rewritten}${quote}`;
  });
}

function injectObfuscatedTextCompatibility(source: string, fallbackFamilies: string[]) {
  if (/data-tepub-font-obfuscation\s*=/.test(source)) return source;
  const defaultFamily = fallbackFamilies.length
    ? `html { font-family: ${fallbackFamilies.map(cssFontFamily).join(", ")}, serif; }\n`
    : "";
  const style = `<style type="text/css" data-tepub-font-obfuscation="1">${defaultFamily}html, body { overflow-wrap: anywhere; word-break: break-all; }</style>`;
  if (/<\/head\s*>/i.test(source)) return source.replace(/<\/head\s*>/i, `${style}</head>`);
  return `${style}${source}`;
}

async function collectEmbeddedFontFamilies(zip: JSZip, fonts: ParsedFont[]) {
  const fontByPath = new Map(fonts.map((font) => [normalizePath(font.path).toLowerCase(), font]));
  const families: Array<{ family: string; score: number; order: number }> = [];
  const seen = new Set<string>();
  for (const path of Object.keys(zip.files).filter((name) => !zip.files[name].dir && extension(name) === "css")) {
    const source = await zip.files[path].async("text");
    for (const match of source.matchAll(/@font-face\s*\{([^{}]*)\}/gi)) {
      const body = match[1];
      const familyMatch = body.match(/font-family\s*:\s*([^;}{]+)/i);
      if (!familyMatch) continue;
      const family = unquotedFontFamily(familyMatch[1]);
      if (!family || seen.has(family.toLowerCase())) continue;
      const referencedFonts = [...body.matchAll(/url\(\s*(['"]?)(.*?)\1\s*\)/gi)].map((urlMatch) => {
        const relative = percentDecode(urlMatch[2].split(/[?#]/, 1)[0]);
        return fontByPath.get(joinPath(parentPath(path), relative).toLowerCase());
      }).filter((font): font is ParsedFont => Boolean(font));
      if (referencedFonts.length) {
        seen.add(family.toLowerCase());
        const score = Math.max(...referencedFonts.map((font) => font.glyphs.reduce(
          (count: number, glyph: TTF.Glyph) => count + Number((glyph.unicode || []).some(isCjkCodePoint)),
          0,
        )));
        families.push({ family, score, order: families.length });
      }
    }
  }
  return families.sort((left, right) => right.score - left.score || left.order - right.order).map(({ family }) => family);
}

function privateCharacters(count: number, excluded: Set<number>) {
  const values: number[] = [];
  for (const [start, end] of [[0xe000, 0xf8ff], [0xf0000, 0xffffd], [0x100000, 0x10fffd]]) {
    for (let code = start; code <= end && values.length < count; code += 1) {
      if (!excluded.has(code)) values.push(code);
    }
    if (values.length >= count) break;
  }
  if (values.length < count) throw new Error("可用私用区字符不足");
  if (typeof crypto !== "undefined") {
    for (let index = values.length - 1; index > 0; index -= 1) {
      const random = new Uint32Array(1);
      crypto.getRandomValues(random);
      const target = random[0] % (index + 1);
      [values[index], values[target]] = [values[target], values[index]];
    }
  }
  return values.map((code) => String.fromCodePoint(code));
}

const BODY_ENCRYPTION_CLASS = "tepub-font-encrypted-body";
const BODY_ENCRYPTION_FAMILY = "TEpubEncryptedBodyFont";

function tagClass(tag: string) {
  return tag.match(/\bclass\s*=\s*(['"])(.*?)\1/i)?.[2] || "";
}

function updateTagClass(tag: string, add: boolean) {
  const classMatch = tag.match(/\bclass\s*=\s*(['"])(.*?)\1/i);
  if (classMatch) {
    const classes = classMatch[2].split(/\s+/).filter(Boolean)
      .filter((name) => name !== BODY_ENCRYPTION_CLASS && !name.startsWith(`${BODY_ENCRYPTION_CLASS}-`));
    if (add) classes.push(BODY_ENCRYPTION_CLASS);
    if (!classes.length) return tag.replace(classMatch[0], "").replace(/\s+([/>])/g, "$1");
    return tag.replace(classMatch[0], `class=${classMatch[1]}${classes.join(" ")}${classMatch[1]}`);
  }
  if (!add) return tag;
  return tag.replace(/\s*(\/?)>$/, ` class="${BODY_ENCRYPTION_CLASS}"$1>`);
}

function transformBodyText(source: string, mapping: Map<string, string>, mode: "encrypt" | "decrypt") {
  let output = "";
  let paragraphDepth = 0;
  const markedStack: string[] = [];
  let rawTag: "script" | "style" | null = null;
  for (let index = 0; index < source.length;) {
    if (source[index] === "<") {
      const end = source.indexOf(">", index + 1);
      if (end < 0) { output += source.slice(index); break; }
      let tag = source.slice(index, end + 1);
      const match = tag.match(/^<\s*(\/?)\s*([\w:-]+)/);
      const closing = match?.[1] === "/";
      const name = match?.[2]?.toLowerCase() || "";
      const selfClosing = /\/\s*>$/.test(tag);
      const marked = tagClass(tag).split(/\s+/)
        .some((className) => className === BODY_ENCRYPTION_CLASS || className.startsWith(`${BODY_ENCRYPTION_CLASS}-`));
      if (closing) {
        const matchIndex = markedStack.lastIndexOf(name);
        while (matchIndex >= 0 && markedStack.length > matchIndex) {
          const markedName = markedStack.pop()!;
          paragraphDepth -= 1;
          if (markedName === name) break;
        }
      }
      if (!closing && !selfClosing) {
        const target = mode === "encrypt" ? BODY_BLOCK_TAGS.has(name) : marked;
        if (target) {
          paragraphDepth += 1;
          markedStack.push(name);
          tag = updateTagClass(tag, mode === "encrypt");
        }
      }
      if (name === "script" || name === "style") rawTag = closing ? null : name;
      output += tag;
      index = end + 1;
      continue;
    }
    if (!rawTag && source[index] === "&") {
      const end = source.indexOf(";", index + 1);
      if (end > index && end - index < 16) { output += source.slice(index, end + 1); index = end + 1; continue; }
    }
    const code = source.codePointAt(index) || 0;
    const character = String.fromCodePoint(code);
    output += !rawTag && paragraphDepth > 0 ? mapping.get(character) || character : character;
    index += character.length;
  }
  return output;
}

function collectBodyCharacters(source: string) {
  const characters: string[] = [];
  let paragraphDepth = 0;
  let rawTag: "script" | "style" | null = null;
  for (let index = 0; index < source.length;) {
    if (source[index] === "<") {
      const end = source.indexOf(">", index + 1);
      if (end < 0) break;
      const tag = source.slice(index, end + 1);
      const match = tag.match(/^<\s*(\/?)\s*([\w:-]+)/);
      const closing = match?.[1] === "/";
      const name = match?.[2]?.toLowerCase() || "";
      if (closing && name === "p" && paragraphDepth > 0) paragraphDepth -= 1;
      if (!closing && name === "p" && !/\/\s*>$/.test(tag)) paragraphDepth += 1;
      if (name === "script" || name === "style") rawTag = closing ? null : name;
      index = end + 1;
      continue;
    }
    if (!rawTag && source[index] === "&") {
      const end = source.indexOf(";", index + 1);
      if (end > index && end - index < 16) { index = end + 1; continue; }
    }
    const code = source.codePointAt(index) || 0;
    const character = String.fromCodePoint(code);
    if (!rawTag && paragraphDepth > 0 && shouldMapCharacter(character)) characters.push(character);
    index += character.length;
  }
  return characters;
}

function randomDerangement(characters: string[]) {
  const shuffled = [...characters];
  for (let index = shuffled.length - 1; index > 0; index -= 1) {
    let target = Math.floor(Math.random() * index);
    if (typeof crypto !== "undefined") {
      const random = new Uint32Array(1);
      crypto.getRandomValues(random);
      target = random[0] % index;
    }
    [shuffled[index], shuffled[target]] = [shuffled[target], shuffled[index]];
  }
  return shuffled;
}

function relativeZipPath(fromFile: string, target: string) {
  const from = parentPath(normalizePath(fromFile)).split("/").filter(Boolean);
  const to = normalizePath(target).split("/").filter(Boolean);
  while (from.length && to.length && from[0].toLowerCase() === to[0].toLowerCase()) {
    from.shift();
    to.shift();
  }
  return [...from.map(() => ".."), ...to].join("/");
}

function injectBodyFontStyle(source: string, htmlPath: string, fontPath: string, remove = false) {
  const pattern = /<style[^>]*data-tepub-body-font-encryption\s*=\s*(['"])1\1[^>]*>.*?<\/style>/gis;
  const cleaned = source.replace(pattern, "");
  if (remove) return cleaned;
  const fontUrl = relativeZipPath(htmlPath, fontPath).replace(/"/g, "%22");
  const style = `<style type="text/css" data-tepub-body-font-encryption="1">@font-face { font-family: ${cssFontFamily(BODY_ENCRYPTION_FAMILY)}; src: url("${fontUrl}"); } .${BODY_ENCRYPTION_CLASS} { font-family: ${cssFontFamily(BODY_ENCRYPTION_FAMILY)} !important; }</style>`;
  if (/<\/head\s*>/i.test(cleaned)) return cleaned.replace(/<\/head\s*>/i, `${style}</head>`);
  return `${style}${cleaned}`;
}

async function packageInfo(zip: JSZip) {
  const containerEntry = zip.file("META-INF/container.xml");
  if (!containerEntry) throw new Error("EPUB 缺少 META-INF/container.xml");
  const container = parseXml(await containerEntry.async("text"), "container.xml");
  const opfPath = elementsByLocalName(container, "rootfile")[0]?.getAttribute("full-path") || "";
  if (!opfPath || !zip.file(opfPath)) throw new Error("EPUB 的 OPF 路径无效");
  const opfSource = await zip.file(opfPath)!.async("text");
  const opf = parseXml(opfSource, "OPF");
  const opfDir = parentPath(opfPath);
  const manifestById = new Map<string, string>();
  for (const item of elementsByLocalName(opf, "item")) {
    const id = item.getAttribute("id") || "";
    const href = percentDecode(item.getAttribute("href")?.split(/[?#]/, 1)[0] || "");
    if (id && href) manifestById.set(id, joinPath(opfDir, href));
  }
  const spinePaths = elementsByLocalName(opf, "itemref")
    .map((item) => manifestById.get(item.getAttribute("idref") || "") || "")
    .filter(Boolean);
  return { opfPath, opfDir, opf, opfSource, spinePaths };
}

async function generateEpub(zip: JSZip) {
  zip.file("mimetype", "application/epub+zip", { compression: "STORE", createFolders: false });
  return zip.generateAsync({ type: "blob", mimeType: "application/epub+zip", compression: "DEFLATE", compressionOptions: { level: 6 } });
}

async function loadFonts(zip: JSZip) {
  const paths = Object.keys(zip.files).filter((path) => !zip.files[path].dir && FONT_EXTENSIONS.has(extension(path)));
  const parsed: ParsedFont[] = [];
  const failures: string[] = [];
  for (const path of paths) {
    try { parsed.push(await parseFont(path, await zip.files[path].async("arraybuffer"))); }
    catch (error) { failures.push(`${path}: ${error instanceof Error ? error.message : String(error)}`); }
  }
  if (!parsed.length) {
    if (failures.length) throw new Error(`内嵌字体解析失败：${failures.join("；")}`);
    throw new Error("EPUB 内未找到 TTF、OTF、WOFF 或 WOFF2 字体");
  }
  return parsed;
}

async function subsetAllFontsForEncryption(zip: JSZip) {
  const fonts = await loadFonts(zip);
  const usage = await collectUsedCodePointsByFont(zip, fonts);
  const pathMap = new Map(Object.keys(zip.files).map((path) => [path, path]));
  let changedFonts = 0;
  for (const font of fonts) {
    const codePoints = usage.get(font);
    if (!codePoints?.size) continue;
    const path = font.path;
    try {
      const source = await zip.files[path].async("arraybuffer");
      const parsed = await parseFont(path, source, [...codePoints]);
      const written = writeFont(parsed);
      if (written.bytes.byteLength >= source.byteLength && parsed.type !== "otf") continue;
      let outputPath = path;
      if (parsed.type === "otf") {
        outputPath = path.replace(/\.otf$/i, ".ttf");
        if (zip.file(outputPath) && outputPath !== path) outputPath = path.replace(/\.otf$/i, "-subset.ttf");
        pathMap.set(path, outputPath);
        zip.remove(path);
      }
      zip.file(outputPath, written.bytes);
      changedFonts += 1;
    } catch {
      // Keep fonts that cannot be safely subsetted and continue with encryption.
    }
  }
  if ([...pathMap].some(([from, to]) => from !== to)) {
    for (const path of Object.keys(zip.files).filter((name) => TEXT_EXTENSIONS.has(extension(name)))) {
      const source = await zip.files[path].async("text");
      zip.file(path, rewriteWebEpubTextLinks(source, path, path, pathMap));
    }
    const pkg = await packageInfo(zip);
    for (const item of elementsByLocalName(pkg.opf, "item")) {
      const href = percentDecode(item.getAttribute("href")?.split(/[?#]/, 1)[0] || "");
      if (joinPath(pkg.opfDir, href).toLowerCase().endsWith(".ttf")) item.setAttribute("media-type", "font/ttf");
    }
    zip.file(pkg.opfPath, new XMLSerializer().serializeToString(pkg.opf));
  }
  return changedFonts;
}

async function selectBodyFonts(zip: JSZip, fonts: ParsedFont[]) {
  const fontByPath = new Map(fonts.map((font) => [normalizePath(font.path).toLowerCase(), font]));
  const familyPaths = new Map<string, string[]>();
  const bodyFamilies: string[] = [];
  for (const path of Object.keys(zip.files).filter((name) => !zip.files[name].dir && extension(name) === "css")) {
    const source = await zip.files[path].async("text");
    for (const match of source.matchAll(/@font-face\s*\{([^{}]*)\}/gi)) {
      const body = match[1];
      const familyMatch = body.match(/font-family\s*:\s*([^;}{]+)/i);
      if (!familyMatch) continue;
      const family = normalizedFontFamily(familyMatch[1]);
      const paths = [...body.matchAll(/url\(\s*(['"]?)(.*?)\1\s*\)/gi)].map((urlMatch) => {
        const relative = percentDecode(urlMatch[2].split(/[?#]/, 1)[0]);
        return joinPath(parentPath(path), relative);
      }).filter((fontPath) => fontByPath.has(fontPath.toLowerCase()));
      if (paths.length) familyPaths.set(family, paths);
    }
    const withoutFaces = cleanCssForFontRules(source);
    for (const rule of withoutFaces.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      const selectors = rule[1].split(",");
      if (!selectors.some((selector) => /(^|[\s>+~])(?:body|p)(?=[:.#\[\s>+~]|$)/i.test(selector.trim()))) continue;
      const familyMatch = rule[2].match(/font-family\s*:\s*([^;}{]+)/i);
      if (!familyMatch) continue;
      for (const family of splitFontFamilies(familyMatch[1]).map(normalizedFontFamily)) {
        if (!bodyFamilies.includes(family)) bodyFamilies.push(family);
      }
    }
  }
  const selected: ParsedFont[] = [];
  for (const family of bodyFamilies) {
    for (const path of familyPaths.get(family) || []) {
      const font = fontByPath.get(path.toLowerCase());
      if (font && !selected.includes(font)) selected.push(font);
    }
  }
  if (selected.length) return selected;
  return [...fonts].sort((left, right) => collectFontPlainCodePoints([right]).size - collectFontPlainCodePoints([left]).size).slice(0, 1);
}

function injectBodyFontStackStyle(source: string, htmlPath: string, fonts: ParsedFont[]) {
  const pattern = /<style[^>]*data-tepub-body-font-encryption\s*=\s*(['"])1\1[^>]*>.*?<\/style>/gis;
  const cleaned = source.replace(pattern, "");
  const faces = fonts.map((font, index) => {
    const family = `${BODY_ENCRYPTION_FAMILY}${index + 1}`;
    const fontUrl = relativeZipPath(htmlPath, font.path).replace(/"/g, "%22");
    return `@font-face { font-family: ${cssFontFamily(family)}; src: url("${fontUrl}"); }`;
  });
  const stack = fonts.map((_font, index) => cssFontFamily(`${BODY_ENCRYPTION_FAMILY}${index + 1}`)).join(", ");
  const style = `<style type="text/css" data-tepub-body-font-encryption="1">${faces.join(" ")} .${BODY_ENCRYPTION_CLASS} { font-family: ${stack} !important; }</style>`;
  if (/<\/head\s*>/i.test(cleaned)) return cleaned.replace(/<\/head\s*>/i, `${style}</head>`);
  return `${style}${cleaned}`;
}

type CssFontRule = { selectors: string[]; families: string[]; important: boolean; order: number };

function fontFamilyDeclaration(source: string) {
  const declarations = [...source.matchAll(/(?:^|;)\s*font-family\s*:\s*([^;}{]+)/gi)];
  const value = declarations.at(-1)?.[1]?.trim();
  if (!value) return null;
  const important = /\s*!important\s*$/i.test(value);
  return {
    families: splitFontFamilies(value.replace(/\s*!important\s*$/i, "")).map(normalizedFontFamily),
    important,
  };
}

function isTitleLikeElement(element: Element | null) {
  for (let current = element; current; current = current.parentElement) {
    const tag = current.localName?.toLowerCase() || "";
    if (/^h[1-6]$/.test(tag) || tag === "title" || tag === "header" || tag === "nav") return true;
    const marker = `${current.getAttribute("class") || ""} ${current.getAttribute("id") || ""}`.toLowerCase();
    if (/(^|[\s_-])(title|subtitle|heading|headline)([\s_-]|$)/.test(marker)
      || /(^|[\s_-])(chapter|volume|part|section)[\s_-]*(title|subtitle|number|name)([\s_-]|$)/.test(marker)) return true;
  }
  return false;
}

function isEncryptableParagraph(element: Element) {
  return isEncryptableTextBlock(element);
}

const BODY_BLOCK_SELECTOR = "p, div, li, blockquote, dd, dt";
const BODY_BLOCK_TAGS = new Set(BODY_BLOCK_SELECTOR.split(", "));

function isEncryptableTextBlock(element: Element) {
  const tag = element.localName?.toLowerCase() || "";
  if (!BODY_BLOCK_TAGS.has(tag) || isTitleLikeElement(element)) return false;
  if (Array.from(element.querySelectorAll("*")).some((child) => isTitleLikeElement(child))) return false;
  if (![...(element.textContent || "")].some(shouldMapCharacter)) return false;
  return !Array.from(element.querySelectorAll(BODY_BLOCK_SELECTOR))
    .some((child) => child !== element && !isTitleLikeElement(child) && [...(child.textContent || "")].some(shouldMapCharacter));
}

function bodyBlockElements(document: Document) {
  return Array.from(document.querySelectorAll(BODY_BLOCK_SELECTOR));
}

function cleanCssForFontRules(source: string) {
  return source
    .replace(/\/\*.*?\*\//gs, "")
    .replace(/@font-face\s*\{[^{}]*\}/gi, "")
    .replace(/@(charset|import|namespace)\b[^;]*;/gi, "");
}

function selectorSpecificity(selector: string) {
  const withoutPseudoElements = selector.replace(/::[\w-]+(?:\([^)]*\))?/g, "");
  const ids = (withoutPseudoElements.match(/#[\w-]+/g) || []).length;
  const classes = (withoutPseudoElements.match(/\.[\w-]+|\[[^\]]+\]|:(?!:)[\w-]+(?:\([^)]*\))?/g) || []).length;
  const tags = (withoutPseudoElements.match(/(^|[\s>+~])(?:[a-z][\w-]*|\*)/gi) || [])
    .filter((match) => !match.trim().endsWith("*")).length;
  return ids * 100 + classes * 10 + tags;
}

async function collectUsedCodePointsByFont(zip: JSZip, fonts: ParsedFont[], outsideParagraphsOnly = false) {
  const fontByPath = new Map(fonts.map((font) => [normalizePath(font.path).toLowerCase(), font]));
  const fontByFamily = new Map<string, ParsedFont>();
  const rules: CssFontRule[] = [];
  const supportedByFont = new Map(fonts.map((font) => {
    const supported = new Set<number>();
    for (const glyph of font.glyphs) for (const code of glyph.unicode || []) supported.add(code);
    return [font, supported] as const;
  }));
  let order = 0;
  for (const path of Object.keys(zip.files).filter((name) => !zip.files[name].dir && extension(name) === "css")) {
    const source = await zip.files[path].async("text");
    for (const match of source.matchAll(/@font-face\s*\{([^{}]*)\}/gi)) {
      const body = match[1];
      const familyMatch = body.match(/font-family\s*:\s*([^;}{]+)/i);
      if (!familyMatch) continue;
      const family = normalizedFontFamily(familyMatch[1]);
      for (const urlMatch of body.matchAll(/url\(\s*(['"]?)(.*?)\1\s*\)/gi)) {
        const relative = percentDecode(urlMatch[2].split(/[?#]/, 1)[0]);
        const font = fontByPath.get(joinPath(parentPath(path), relative).toLowerCase());
        if (font) { fontByFamily.set(family, font); break; }
      }
    }
    const withoutFaces = cleanCssForFontRules(source);
    for (const match of withoutFaces.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      const declaration = fontFamilyDeclaration(match[2]);
      if (!declaration) continue;
      rules.push({
        selectors: match[1].split(",").map((selector) => selector.trim()).filter(Boolean),
        families: declaration.families,
        important: declaration.important,
        order: order++,
      });
    }
  }

  function matchingFamilies(element: Element) {
    const inline = fontFamilyDeclaration(element.getAttribute("style") || "");
    let winner: CssFontRule | null = null;
    let winnerSpecificity = -1;
    for (const rule of rules) {
      let matchedSpecificity = -1;
      for (const selector of rule.selectors) {
        try {
          const clean = selector.replace(/::[\w-]+(?:\([^)]*\))?/g, "");
          if (element.matches(clean)) matchedSpecificity = Math.max(matchedSpecificity, selectorSpecificity(clean));
        } catch { /* Ignore selectors unsupported by the XML DOM matcher. */ }
      }
      const beatsWinner = matchedSpecificity >= 0 && (
        !winner
        || (rule.important && !winner.important)
        || (rule.important === winner.important && (matchedSpecificity > winnerSpecificity
          || (matchedSpecificity === winnerSpecificity && rule.order >= winner.order)))
      );
      if (beatsWinner) {
        winner = rule;
        winnerSpecificity = matchedSpecificity;
      }
    }
    if (inline && (!winner?.important || inline.important)) return inline.families;
    return winner?.families || inline?.families || [];
  }

  const usage = new Map<ParsedFont, Set<number>>();
  const htmlPaths = Object.keys(zip.files).filter((path) => !zip.files[path].dir && HTML_EXTENSIONS.has(extension(path)));
  for (const path of htmlPaths) {
    const source = await zip.files[path].async("text");
    const document = new DOMParser().parseFromString(source.replace(/<link\b[^>]*>/gi, ""), "application/xml");
    for (const style of Array.from(document.querySelectorAll("style"))) {
      const styleSource = style.textContent || "";
      for (const match of styleSource.matchAll(/@font-face\s*\{([^{}]*)\}/gi)) {
        const familyMatch = match[1].match(/font-family\s*:\s*([^;}{]+)/i);
        if (!familyMatch) continue;
        const family = normalizedFontFamily(familyMatch[1]);
        for (const urlMatch of match[1].matchAll(/url\(\s*(['"]?)(.*?)\1\s*\)/gi)) {
          const relative = percentDecode(urlMatch[2].split(/[?#]/, 1)[0]);
          const font = fontByPath.get(joinPath(parentPath(path), relative).toLowerCase());
          if (font) { fontByFamily.set(family, font); break; }
        }
      }
      const withoutFaces = cleanCssForFontRules(styleSource);
      for (const match of withoutFaces.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
        const declaration = fontFamilyDeclaration(match[2]);
        if (!declaration) continue;
        rules.push({
          selectors: match[1].split(",").map((selector) => selector.trim()).filter(Boolean),
          families: declaration.families,
          important: declaration.important,
          order: order++,
        });
      }
    }
    const root = elementsByLocalName(document, "body")[0] || document.documentElement;
    const visit = (node: Node) => {
      if (node.nodeType === 3) {
        const text = node.nodeValue || "";
        if (!text.trim()) return;
        let element = node.parentElement;
        if (outsideParagraphsOnly) {
          let ancestor = element;
          while (ancestor && !BODY_BLOCK_TAGS.has(ancestor.localName?.toLowerCase() || "")) ancestor = ancestor.parentElement;
          if (ancestor && isEncryptableTextBlock(ancestor)) return;
        }
        let families: string[] = [];
        while (element && !families.length) {
          families = matchingFamilies(element);
          element = element.parentElement;
        }
        for (const character of text) {
          const code = character.codePointAt(0) || 0;
          if (code < 32) continue;
          const font = families.map((family) => fontByFamily.get(family))
            .find((candidate) => candidate && supportedByFont.get(candidate)?.has(code));
          if (!font) continue;
          const codePoints = usage.get(font) || new Set<number>();
          codePoints.add(code);
          usage.set(font, codePoints);
        }
        return;
      }
      if (node.nodeType !== 1 && node.nodeType !== 9) return;
      const tag = (node as Element).localName?.toLowerCase();
      if (tag === "script" || tag === "style") return;
      for (const child of Array.from(node.childNodes)) visit(child);
    };
    if (root) visit(root);
  }
  for (const codePoints of usage.values()) for (let code = 32; code <= 126; code += 1) codePoints.add(code);
  return usage;
}

async function resolveParagraphFonts(zip: JSZip, fonts: ParsedFont[], htmlSources: Map<string, string>) {
  const fontByPath = new Map(fonts.map((font) => [normalizePath(font.path).toLowerCase(), font]));
  const fontByFamily = new Map<string, ParsedFont>();
  const rules: CssFontRule[] = [];
  let order = 0;
  for (const path of Object.keys(zip.files).filter((name) => !zip.files[name].dir && extension(name) === "css")) {
    const source = await zip.files[path].async("text");
    for (const match of source.matchAll(/@font-face\s*\{([^{}]*)\}/gi)) {
      const body = match[1];
      const familyMatch = body.match(/font-family\s*:\s*([^;}{]+)/i);
      if (!familyMatch) continue;
      const family = normalizedFontFamily(familyMatch[1]);
      for (const urlMatch of body.matchAll(/url\(\s*(['"]?)(.*?)\1\s*\)/gi)) {
        const relative = percentDecode(urlMatch[2].split(/[?#]/, 1)[0]);
        const font = fontByPath.get(joinPath(parentPath(path), relative).toLowerCase());
        if (font) { fontByFamily.set(family, font); break; }
      }
    }
    const withoutFaces = source.replace(/@font-face\s*\{[^{}]*\}/gi, "");
    for (const match of withoutFaces.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
      const declaration = fontFamilyDeclaration(match[2]);
      if (!declaration) continue;
      rules.push({
        selectors: match[1].split(",").map((selector) => selector.trim()).filter(Boolean),
        families: declaration.families,
        important: declaration.important,
        order: order++,
      });
    }
  }

  const supportedByFont = new Map(fonts.map((font) => {
    const supported = new Set<number>();
    for (const glyph of font.glyphs) for (const code of glyph.unicode || []) if (isCjkCodePoint(code)) supported.add(code);
    return [font, supported] as const;
  }));
  const fallbackFont = [...fonts].sort((left, right) => (supportedByFont.get(right)?.size || 0) - (supportedByFont.get(left)?.size || 0))[0];
  const assignments = new Map<string, Array<ParsedFont | null>>();

  function matchingFamilies(element: Element) {
    const inline = fontFamilyDeclaration(element.getAttribute("style") || "");
    let winner: CssFontRule | null = null;
    let winnerSpecificity = -1;
    for (const rule of rules) {
      let matchedSpecificity = -1;
      for (const selector of rule.selectors) {
        try {
          const clean = selector.replace(/::[\w-]+(?:\([^)]*\))?/g, "");
          if (element.matches(clean)) matchedSpecificity = Math.max(matchedSpecificity, selectorSpecificity(clean));
        } catch { /* Ignore unsupported selectors. */ }
      }
      const beatsWinner = matchedSpecificity >= 0 && (
        !winner
        || (rule.important && !winner.important)
        || (rule.important === winner.important && (matchedSpecificity > winnerSpecificity
          || (matchedSpecificity === winnerSpecificity && rule.order >= winner.order)))
      );
      if (beatsWinner) {
        winner = rule;
        winnerSpecificity = matchedSpecificity;
      }
    }
    if (inline && (!winner?.important || inline.important)) return inline.families;
    return winner?.families || inline?.families || [];
  }

  for (const [path, source] of htmlSources) {
    const document = new DOMParser().parseFromString(source.replace(/<link\b[^>]*>/gi, ""), "application/xml");
    for (const style of Array.from(document.querySelectorAll("style"))) {
      const styleSource = style.textContent || "";
      for (const match of styleSource.matchAll(/@font-face\s*\{([^{}]*)\}/gi)) {
        const familyMatch = match[1].match(/font-family\s*:\s*([^;}{]+)/i);
        if (!familyMatch) continue;
        const family = normalizedFontFamily(familyMatch[1]);
        for (const urlMatch of match[1].matchAll(/url\(\s*(['"]?)(.*?)\1\s*\)/gi)) {
          const relative = percentDecode(urlMatch[2].split(/[?#]/, 1)[0]);
          const font = fontByPath.get(joinPath(parentPath(path), relative).toLowerCase());
          if (font) { fontByFamily.set(family, font); break; }
        }
      }
      const withoutFaces = cleanCssForFontRules(styleSource);
      for (const match of withoutFaces.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
        const declaration = fontFamilyDeclaration(match[2]);
        if (!declaration) continue;
        rules.push({
          selectors: match[1].split(",").map((selector) => selector.trim()).filter(Boolean),
          families: declaration.families,
          important: declaration.important,
          order: order++,
        });
      }
    }
    const paragraphFonts: Array<ParsedFont | null> = [];
    for (const paragraph of bodyBlockElements(document)) {
      if (!isEncryptableTextBlock(paragraph)) {
        paragraphFonts.push(null);
        continue;
      }
      let families: string[] = [];
      let current: Element | null = paragraph;
      while (current && !families.length) {
        families = matchingFamilies(current);
        current = current.parentElement;
      }
      let font = families.map((family) => fontByFamily.get(family)).find(Boolean) || null;
      if (!font && !families.length) font = fallbackFont || null;
      if (font) {
        const supported = supportedByFont.get(font) || new Set<number>();
        const hasSupportedText = [...(paragraph.textContent || "")].some((character) => supported.has(character.codePointAt(0) || 0));
        if (!hasSupportedText) font = null;
      }
      paragraphFonts.push(font);
    }
    assignments.set(path, paragraphFonts);
  }
  return { assignments, supportedByFont };
}

function transformParagraphsByFont(
  source: string,
  paragraphFonts: Array<ParsedFont | null>,
  mappings: Map<ParsedFont, Map<string, string>>,
  fontIndexes: Map<ParsedFont, number>,
) {
  let output = "";
  let paragraphIndex = 0;
  let activeMapping: Map<string, string> | null = null;
  const blockStack: Array<{ name: string; previous: Map<string, string> | null }> = [];
  let rawTag: "script" | "style" | null = null;
  for (let index = 0; index < source.length;) {
    if (source[index] === "<") {
      const end = source.indexOf(">", index + 1);
      if (end < 0) { output += source.slice(index); break; }
      let tag = source.slice(index, end + 1);
      const match = tag.match(/^<\s*(\/?)\s*([\w:-]+)/);
      const closing = match?.[1] === "/";
      const name = match?.[2]?.toLowerCase() || "";
      if (!closing && BODY_BLOCK_TAGS.has(name)) {
        const previous = activeMapping;
        const font = paragraphFonts[paragraphIndex++] || null;
        activeMapping = font ? mappings.get(font) || null : null;
        blockStack.push({ name, previous });
        if (font && activeMapping?.size) {
          const className = `${BODY_ENCRYPTION_CLASS}-${(fontIndexes.get(font) || 0) + 1}`;
          const classMatch = tag.match(/\bclass\s*=\s*(['"])(.*?)\1/i);
          tag = classMatch
            ? tag.replace(classMatch[0], `class=${classMatch[1]}${classMatch[2]} ${className}${classMatch[1]}`)
            : tag.replace(/\s*(\/?)>$/, ` class="${className}"$1>`);
        }
      } else if (closing && BODY_BLOCK_TAGS.has(name)) {
        while (blockStack.length) {
          const block = blockStack.pop()!;
          activeMapping = block.previous;
          if (block.name === name) break;
        }
      }
      if (name === "script" || name === "style") rawTag = closing ? null : name;
      output += tag;
      index = end + 1;
      continue;
    }
    if (!rawTag && source[index] === "&") {
      const end = source.indexOf(";", index + 1);
      if (end > index && end - index < 16) { output += source.slice(index, end + 1); index = end + 1; continue; }
    }
    const code = source.codePointAt(index) || 0;
    const character = String.fromCodePoint(code);
    output += !rawTag && activeMapping ? activeMapping.get(character) || character : character;
    index += character.length;
  }
  return output;
}

function injectIndependentBodyFontStyle(source: string, htmlPath: string, fonts: ParsedFont[]) {
  const faces = fonts.map((font, index) => {
    const family = `${BODY_ENCRYPTION_FAMILY}${index + 1}`;
    const fontUrl = relativeZipPath(htmlPath, font.path).replace(/"/g, "%22");
    return `@font-face { font-family: ${cssFontFamily(family)}; src: url("${fontUrl}"); } .${BODY_ENCRYPTION_CLASS}-${index + 1}, .${BODY_ENCRYPTION_CLASS}-${index + 1} * { font-family: ${cssFontFamily(family)} !important; }`;
  });
  const style = `<style type="text/css" data-tepub-body-font-encryption="1">${faces.join(" ")}</style>`;
  if (/<\/head\s*>/i.test(source)) return source.replace(/<\/head\s*>/i, `${style}</head>`);
  return `${style}${source}`;
}

async function encryptFonts(file: File, zip: JSZip): Promise<WebEpubFontProcessResult> {
  const fonts = await loadFonts(zip);
  const fallbackFamilies = await collectEmbeddedFontFamilies(zip, fonts);
  const supported = collectFontPlainCodePoints(fonts);
  if (!supported.size) throw new Error("内嵌字体未发现可加密汉字");
  const htmlPaths = Object.keys(zip.files).filter((path) => !zip.files[path].dir && HTML_EXTENSIONS.has(extension(path)));
  if (!htmlPaths.length) throw new Error("EPUB 内未找到 HTML/XHTML 文件");

  const seen = new Set<string>();
  const characters: string[] = [];
  const existingPrivate = collectFontPrivateCodePoints(fonts);
  const htmlSources = new Map<string, string>();
  for (const path of htmlPaths) {
    const source = await zip.files[path].async("text");
    htmlSources.set(path, source);
    for (const character of source) {
      const code = character.codePointAt(0) || 0;
      if (isPrivateCodePoint(code)) existingPrivate.add(code);
    }
    for (const character of collectHtmlCharacters(source)) {
      const code = character.codePointAt(0) || 0;
      if (supported.has(code) && !seen.has(character)) { seen.add(character); characters.push(character); }
    }
  }
  if (!characters.length) throw new Error("未找到可加密的正文汉字");
  const shadows = privateCharacters(characters.length, existingPrivate);
  const mapping = new Map(characters.map((character, index) => [character, shadows[index]]));

  let changedFiles = 0;
  for (const [path, source] of htmlSources) {
    const transformed = injectObfuscatedTextCompatibility(
      rewriteInlineFontFallbacks(transformHtmlText(source, mapping), fallbackFamilies),
      fallbackFamilies,
    );
    if (transformed !== source) { zip.file(path, transformed); changedFiles += 1; }
  }

  for (const path of Object.keys(zip.files).filter((name) => !zip.files[name].dir && extension(name) === "css")) {
    const source = await zip.files[path].async("text");
    const transformed = rewriteCssFontFallbacks(source, fallbackFamilies);
    if (transformed !== source) { zip.file(path, transformed); changedFiles += 1; }
  }

  const pathMap = new Map(Object.keys(zip.files).map((path) => [path, path]));
  let changedFonts = 0;
  for (const parsed of fonts) {
    let added = 0;
    for (const glyph of parsed.glyphs) {
      const unicodes = glyph.unicode || [];
      const additions: number[] = [];
      for (const code of unicodes) {
        const shadow = mapping.get(String.fromCodePoint(code));
        if (shadow) additions.push(shadow.codePointAt(0) || 0);
      }
      for (const code of additions) {
        if (!unicodes.includes(code)) { unicodes.push(code); added += 1; }
      }
      glyph.unicode = unicodes;
    }
    if (!added) continue;
    const written = writeFont(parsed);
    let outputPath = parsed.path;
    if (parsed.type === "otf") {
      outputPath = parsed.path.replace(/\.otf$/i, ".ttf");
      if (zip.file(outputPath) && outputPath !== parsed.path) outputPath = parsed.path.replace(/\.otf$/i, "-converted.ttf");
      pathMap.set(parsed.path, outputPath);
      zip.remove(parsed.path);
    }
    zip.file(outputPath, written.bytes);
    changedFonts += 1;
  }
  if (!changedFonts) throw new Error("字体 cmap 未能写入私用区映射，无法完成字体加密");

  if ([...pathMap].some(([from, to]) => from !== to)) {
    for (const path of Object.keys(zip.files).filter((name) => TEXT_EXTENSIONS.has(extension(name)))) {
      const source = await zip.files[path].async("text");
      zip.file(path, rewriteWebEpubTextLinks(source, path, path, pathMap));
    }
    const pkg = await packageInfo(zip);
    for (const item of elementsByLocalName(pkg.opf, "item")) {
      const href = percentDecode(item.getAttribute("href")?.split(/[?#]/, 1)[0] || "");
      if (joinPath(pkg.opfDir, href).toLowerCase().endsWith(".ttf")) item.setAttribute("media-type", "font/ttf");
    }
    zip.file(pkg.opfPath, new XMLSerializer().serializeToString(pkg.opf));
  }

  zip.file(MAP_PATH, JSON.stringify({ version: 1, map: [...mapping.entries()] }));
  const blob = await generateEpub(zip);
  return {
    sourceName: file.name,
    outputName: sourceNameWithSuffix(file.name, "_font_encrypt"),
    action: "font-encrypt",
    changedFiles,
    mappedCharacters: mapping.size,
    changedFonts,
    mode: "embedded-map",
    message: `字体加密完成：映射 ${mapping.size} 个汉字，更新 ${changedFonts} 个字体`,
    blob,
  };
}

async function encryptBodyFontPermutation(file: File, zip: JSZip): Promise<WebEpubFontProcessResult> {
  const fonts = await loadFonts(zip);
  const pkg = await packageInfo(zip);
  const htmlPaths = pkg.spinePaths.length
    ? pkg.spinePaths.filter((path) => zip.file(path) && HTML_EXTENSIONS.has(extension(path)))
    : Object.keys(zip.files).filter((path) => !zip.files[path].dir && HTML_EXTENSIONS.has(extension(path)));
  if (!htmlPaths.length) throw new Error("EPUB 内未找到正文 HTML/XHTML 文件");

  const htmlSources = new Map<string, string>();
  const bodyCharacters = new Set<string>();
  for (const path of htmlPaths) {
    const source = await zip.files[path].async("text");
    htmlSources.set(path, source);
    for (const character of collectBodyCharacters(source)) bodyCharacters.add(character);
  }
  if (bodyCharacters.size < 2) throw new Error("未找到可加密的正文段落汉字");

  const candidates = fonts.map((font) => {
    const supported = new Set<number>();
    for (const glyph of font.glyphs) {
      for (const code of glyph.unicode || []) if (isCjkCodePoint(code)) supported.add(code);
    }
    const characters = [...bodyCharacters].filter((character) => supported.has(character.codePointAt(0) || 0));
    return { font, characters };
  }).sort((left, right) => right.characters.length - left.characters.length);
  const selected = candidates[0];
  if (!selected || selected.characters.length < 2) throw new Error("正文字体中未找到足够的可加密汉字");

  const shuffled = randomDerangement(selected.characters);
  const mapping = new Map(selected.characters.map((character, index) => [character, shuffled[index]]));

  let changedFiles = 0;
  for (const [path, source] of htmlSources) {
    const encrypted = transformBodyText(source, mapping, "encrypt");
    const transformed = encrypted !== source ? injectBodyFontStyle(encrypted, path, selected.font.path) : source;
    if (transformed !== source) {
      zip.file(path, transformed);
      changedFiles += 1;
    }
  }
  if (!changedFiles) throw new Error("未发现可加密的正文段落");

  for (const glyph of selected.font.glyphs) {
    glyph.unicode = [...new Set((glyph.unicode || []).map((code) => {
      const cipher = mapping.get(String.fromCodePoint(code));
      return cipher ? cipher.codePointAt(0) || code : code;
    }))];
  }

  const pathMap = new Map(Object.keys(zip.files).map((path) => [path, path]));
  const written = writeFont(selected.font);
  let outputPath = selected.font.path;
  if (selected.font.type === "otf") {
    outputPath = selected.font.path.replace(/\.otf$/i, ".ttf");
    if (zip.file(outputPath) && outputPath !== selected.font.path) outputPath = selected.font.path.replace(/\.otf$/i, "-converted.ttf");
    pathMap.set(selected.font.path, outputPath);
    zip.remove(selected.font.path);
  }
  zip.file(outputPath, written.bytes);

  if ([...pathMap].some(([from, to]) => from !== to)) {
    for (const path of Object.keys(zip.files).filter((name) => TEXT_EXTENSIONS.has(extension(name)))) {
      const source = await zip.files[path].async("text");
      zip.file(path, rewriteWebEpubTextLinks(source, path, path, pathMap));
    }
    const updatedPkg = await packageInfo(zip);
    for (const item of elementsByLocalName(updatedPkg.opf, "item")) {
      const href = percentDecode(item.getAttribute("href")?.split(/[?#]/, 1)[0] || "");
      if (joinPath(updatedPkg.opfDir, href).toLowerCase().endsWith(".ttf")) item.setAttribute("media-type", "font/ttf");
    }
    zip.file(updatedPkg.opfPath, new XMLSerializer().serializeToString(updatedPkg.opf));
  }

  zip.remove(MAP_PATH);
  const blob = await generateEpub(zip);
  return {
    sourceName: file.name,
    outputName: sourceNameWithSuffix(file.name, "_font_encrypt"),
    action: "font-encrypt",
    changedFiles,
    mappedCharacters: mapping.size,
    changedFonts: 1,
    mode: "body-font-cjk-permutation",
    message: `字体加密完成：随机打乱 ${mapping.size} 个正文汉字，仅更新 1 个正文字体`,
    blob,
  };
}

async function encryptBodyFontsIndependently(file: File, zip: JSZip): Promise<WebEpubFontProcessResult> {
  const subsettedFonts = await subsetAllFontsForEncryption(zip);
  const fonts = await loadFonts(zip);
  const pkg = await packageInfo(zip);
  const htmlPaths = pkg.spinePaths.length
    ? pkg.spinePaths.filter((path) => zip.file(path) && HTML_EXTENSIONS.has(extension(path)))
    : Object.keys(zip.files).filter((path) => !zip.files[path].dir && HTML_EXTENSIONS.has(extension(path)));
  if (!htmlPaths.length) throw new Error("EPUB 内未找到正文 HTML/XHTML 文件");
  const htmlSources = new Map<string, string>();
  for (const path of htmlPaths) htmlSources.set(path, await zip.files[path].async("text"));

  const { assignments, supportedByFont } = await resolveParagraphFonts(zip, fonts, htmlSources);
  const protectedByFont = await collectUsedCodePointsByFont(zip, fonts, true);
  const charactersByFont = new Map<ParsedFont, Set<string>>();
  for (const [path, source] of htmlSources) {
    const document = new DOMParser().parseFromString(source.replace(/<link\b[^>]*>/gi, ""), "application/xml");
    const paragraphFonts = assignments.get(path) || [];
    bodyBlockElements(document).forEach((paragraph, index) => {
      const font = paragraphFonts[index];
      if (!font) return;
      const supported = supportedByFont.get(font) || new Set<number>();
      const protectedCodePoints = protectedByFont.get(font) || new Set<number>();
      const characters = charactersByFont.get(font) || new Set<string>();
      for (const character of paragraph.textContent || "") {
        const code = character.codePointAt(0) || 0;
        if (shouldMapCharacter(character) && supported.has(code) && !protectedCodePoints.has(code)) characters.add(character);
      }
      charactersByFont.set(font, characters);
    });
  }

  const mappings = new Map<ParsedFont, Map<string, string>>();
  for (const [font, characters] of charactersByFont) {
    const values = [...characters];
    if (values.length < 2) continue;
    const shuffled = randomDerangement(values);
    mappings.set(font, new Map(values.map((character, index) => [character, shuffled[index]])));
  }
  const encryptedFonts = [...mappings.keys()];
  if (!encryptedFonts.length) throw new Error("正文字体中未找到足够的可加密汉字");
  const fontIndexes = new Map(encryptedFonts.map((font, index) => [font, index]));

  let changedFiles = 0;
  for (const [path, source] of htmlSources) {
    const encrypted = transformParagraphsByFont(source, assignments.get(path) || [], mappings, fontIndexes);
    if (encrypted !== source) {
      zip.file(path, injectIndependentBodyFontStyle(encrypted, path, encryptedFonts));
      changedFiles += 1;
    }
  }
  if (!changedFiles) throw new Error("未发现可加密的正文段落");

  const pathMap = new Map(Object.keys(zip.files).map((path) => [path, path]));
  for (const [font, mapping] of mappings) {
    for (const glyph of font.glyphs) {
      glyph.unicode = [...new Set((glyph.unicode || []).map((code) => {
        const cipher = mapping.get(String.fromCodePoint(code));
        return cipher ? cipher.codePointAt(0) || code : code;
      }))];
    }
    const written = writeFont(font);
    let outputPath = font.path;
    if (font.type === "otf") {
      outputPath = font.path.replace(/\.otf$/i, ".ttf");
      if (zip.file(outputPath) && outputPath !== font.path) outputPath = font.path.replace(/\.otf$/i, "-converted.ttf");
      pathMap.set(font.path, outputPath);
      zip.remove(font.path);
    }
    zip.file(outputPath, written.bytes);
  }

  if ([...pathMap].some(([from, to]) => from !== to)) {
    for (const path of Object.keys(zip.files).filter((name) => TEXT_EXTENSIONS.has(extension(name)))) {
      const source = await zip.files[path].async("text");
      zip.file(path, rewriteWebEpubTextLinks(source, path, path, pathMap));
    }
    const updatedPkg = await packageInfo(zip);
    for (const item of elementsByLocalName(updatedPkg.opf, "item")) {
      const href = percentDecode(item.getAttribute("href")?.split(/[?#]/, 1)[0] || "");
      if (joinPath(updatedPkg.opfDir, href).toLowerCase().endsWith(".ttf")) item.setAttribute("media-type", "font/ttf");
    }
    zip.file(updatedPkg.opfPath, new XMLSerializer().serializeToString(updatedPkg.opf));
  }

  zip.remove(MAP_PATH);
  const mappedCharacters = [...mappings.values()].reduce((sum, mapping) => sum + mapping.size, 0);
  const blob = await generateEpub(zip);
  return {
    sourceName: file.name,
    outputName: sourceNameWithSuffix(file.name, "_font_encrypt"),
    action: "font-encrypt",
    changedFiles,
    mappedCharacters,
    changedFonts: encryptedFonts.length,
    mode: "independent-body-font-permutation",
    message: `字体加密完成：${encryptedFonts.length} 个正文字体分别随机映射 ${mappedCharacters} 个汉字；${subsettedFonts ? `加密前自动精简 ${subsettedFonts} 个字体` : "内嵌字体已是精简状态，无需重复子集化"}`,
    blob,
  };
}

function savedMapping(source: string) {
  const payload = JSON.parse(source);
  const pairs: unknown[][] = Array.isArray(payload?.map) ? payload.map : [];
  const entries: Array<[string, string]> = pairs.map((pair) => [String(pair?.[1] || ""), String(pair?.[0] || "")]);
  return new Map(entries.filter(([cipher, plain]) => Boolean(cipher && plain)));
}

function deriveMappingFromFonts(fonts: ParsedFont[]) {
  const votes = new Map<string, Map<string, number>>();
  for (const parsed of fonts) {
    for (const glyph of parsed.glyphs) {
      const plainCodes = (glyph.unicode || []).filter(isCjkCodePoint);
      const privateCodes = (glyph.unicode || []).filter(isPrivateCodePoint);
      for (const privateCode of privateCodes) {
        const cipher = String.fromCodePoint(privateCode);
        const counts = votes.get(cipher) || new Map<string, number>();
        for (const plainCode of plainCodes) {
          const plain = String.fromCodePoint(plainCode);
          counts.set(plain, (counts.get(plain) || 0) + 1);
        }
        votes.set(cipher, counts);
      }
    }
  }
  const mapping = new Map<string, string>();
  for (const [cipher, counts] of votes) {
    const winner = [...counts].sort((a, b) => b[1] - a[1])[0]?.[0];
    if (winner) mapping.set(cipher, winner);
  }
  return mapping;
}

function normalizeCandidates(source: string) {
  return [...source].filter(isMappingCandidate);
}

function mappingFromPlainText(cipherText: string, plainText: string) {
  const cipher = normalizeCandidates(cipherText);
  const plain = normalizeCandidates(plainText).filter((character) => isCjkCodePoint(character.codePointAt(0) || 0));
  if (cipher.length < 50 || plain.length < 50) throw new Error("EPUB 或 TXT 文本过短，无法可靠对齐");

  const offsets = new Map<number, number>();
  const positions = new Map<string, number[]>();
  plain.slice(0, 8000).forEach((character, index) => {
    if (!positions.has(character)) positions.set(character, []);
    if ((positions.get(character)?.length || 0) < 10) positions.get(character)!.push(index);
  });
  cipher.slice(0, 8000).forEach((character, index) => {
    if (!isCjkCodePoint(character.codePointAt(0) || 0)) return;
    for (const plainIndex of positions.get(character) || []) {
      const offset = plainIndex - index;
      if (Math.abs(offset) <= 2400) offsets.set(offset, (offsets.get(offset) || 0) + 1);
    }
  });
  const bestOffset = cipher.length === plain.length
    ? 0
    : [...offsets].sort((a, b) => b[1] - a[1] || Math.abs(a[0]) - Math.abs(b[0]))[0]?.[0] || 0;
  const counts = new Map<string, Map<string, number>>();
  for (let cipherIndex = 0; cipherIndex < cipher.length; cipherIndex += 1) {
    const plainIndex = cipherIndex + bestOffset;
    if (plainIndex < 0 || plainIndex >= plain.length) continue;
    const cipherCharacter = cipher[cipherIndex];
    const plainCharacter = plain[plainIndex];
    if (!isMappingCandidate(cipherCharacter) || !shouldMapCharacter(plainCharacter)) continue;
    const votes = counts.get(cipherCharacter) || new Map<string, number>();
    votes.set(plainCharacter, (votes.get(plainCharacter) || 0) + 1);
    counts.set(cipherCharacter, votes);
  }
  const mapping = new Map<string, string>();
  for (const [cipherCharacter, votes] of counts) {
    const sorted = [...votes].sort((a, b) => b[1] - a[1]);
    const total = sorted.reduce((sum, [, count]) => sum + count, 0);
    if (sorted[0] && sorted[0][1] / total >= 0.72) mapping.set(cipherCharacter, sorted[0][0]);
  }
  if (mapping.size < 30) throw new Error("TXT 与 EPUB 对齐失败，请确认两者是同一版本");
  return mapping;
}

async function orderedCipherText(zip: JSZip) {
  const pkg = await packageInfo(zip);
  const paths = pkg.spinePaths.length
    ? pkg.spinePaths.filter((path) => zip.file(path) && HTML_EXTENSIONS.has(extension(path)))
    : Object.keys(zip.files).filter((path) => !zip.files[path].dir && HTML_EXTENSIONS.has(extension(path)));
  const chunks: string[] = [];
  for (const path of paths) {
    const source = await zip.files[path].async("text");
    const bodySource = source.match(/<body\b[^>]*>([\s\S]*?)<\/body\s*>/i)?.[1] || source;
    const visibleSource = bodySource
      .replace(/<script\b[^>]*>[\s\S]*?<\/script\s*>/gi, "")
      .replace(/<style\b[^>]*>[\s\S]*?<\/style\s*>/gi, "");
    const document = new DOMParser().parseFromString(`<body>${visibleSource}</body>`, "text/html");
    chunks.push(document.body?.textContent || "");
  }
  return chunks.join("\n");
}

function neutralizeFontCss(source: string) {
  return source.replace(/@font-face\s*\{.*?\}/gis, "").replace(/font-family\s*:\s*[^;}{]+;?/gis, "font-family: serif;");
}

async function decryptFonts(file: File, zip: JSZip, plainText?: string): Promise<WebEpubFontProcessResult> {
  let mapping = new Map<string, string>();
  let mode = "";
  const mapEntry = zip.file(MAP_PATH);
  if (mapEntry) {
    mapping = savedMapping(await mapEntry.async("text"));
    mode = "saved-map";
  }
  let fonts: ParsedFont[] = [];
  if (!mapping.size) {
    fonts = await loadFonts(zip);
    mapping = deriveMappingFromFonts(fonts);
    mode = "font-cmap";
  }
  if (!mapping.size && plainText) {
    mapping = mappingFromPlainText(await orderedCipherText(zip), plainText);
    mode = "txt-alignment";
  }
  if (!mapping.size) throw new Error("未发现可用字体映射；请选择与 EPUB 对应的明文 TXT 后重试");

  let changedFiles = 0;
  for (const path of Object.keys(zip.files)) {
    if (zip.files[path].dir) continue;
    const ext = extension(path);
    if (HTML_EXTENSIONS.has(ext)) {
      const source = await zip.files[path].async("text");
      const bodyPermutation = source.includes(BODY_ENCRYPTION_CLASS) || source.includes("data-tepub-body-font-encryption");
      let transformed = bodyPermutation ? transformBodyText(source, mapping, "decrypt") : transformHtmlText(source, mapping);
      if (bodyPermutation) transformed = injectBodyFontStyle(transformed, path, "", true);
      if (mode === "txt-alignment") transformed = transformed.replace(/font-family\s*:\s*[^;"'}]+;?/gis, "font-family: serif;");
      if (transformed !== source) { zip.file(path, transformed); changedFiles += 1; }
    } else if (mode === "txt-alignment" && ext === "css") {
      const source = await zip.files[path].async("text");
      const transformed = neutralizeFontCss(source);
      if (transformed !== source) zip.file(path, transformed);
    }
  }
  if (!changedFiles) throw new Error("未发现可恢复的字体混淆正文");
  zip.remove(MAP_PATH);
  const blob = await generateEpub(zip);
  return {
    sourceName: file.name,
    outputName: sourceNameWithSuffix(file.name, "_font_decrypt"),
    action: "font-decrypt",
    changedFiles,
    mappedCharacters: mapping.size,
    changedFonts: fonts.length,
    mode,
    message: `字体解密完成：恢复 ${mapping.size} 个字符，更新 ${changedFiles} 个正文文件`,
    blob,
  };
}

function collectSubsetCodePoints(source: string, output: Set<number>) {
  let rawTag: "script" | "style" | null = null;
  for (let index = 0; index < source.length;) {
    if (source[index] === "<") {
      const end = source.indexOf(">", index + 1);
      if (end < 0) break;
      const tag = source.slice(index, end + 1);
      const match = tag.match(/^<\s*(\/?)\s*([\w:-]+)/);
      const closing = match?.[1] === "/";
      const name = match?.[2]?.toLowerCase();
      if (name === "script" || name === "style") rawTag = closing ? null : name;
      index = end + 1;
      continue;
    }
    if (!rawTag && source[index] === "&") {
      const end = source.indexOf(";", index + 1);
      if (end > index && end - index < 16) {
        const entity = source.slice(index, end + 1);
        const numeric = entity.match(/^&#(?:x([0-9a-f]+)|(\d+));$/i);
        if (numeric) {
          const code = Number.parseInt(numeric[1] || numeric[2], numeric[1] ? 16 : 10);
          if (code >= 32 && code <= 0x10ffff && !(code >= 0xd800 && code <= 0xdfff)) output.add(code);
        }
        index = end + 1;
        continue;
      }
    }
    const code = source.codePointAt(index) || 0;
    const character = String.fromCodePoint(code);
    if (!rawTag && code >= 32) output.add(code);
    index += character.length;
  }
}

async function subsetFonts(file: File, zip: JSZip): Promise<WebEpubFontProcessResult> {
  const fonts = await loadFonts(zip);
  const usage = await collectUsedCodePointsByFont(zip, fonts);
  const retainedCodePoints = [...usage.values()].reduce((sum, codePoints) => sum + codePoints.size, 0);
  if (!retainedCodePoints) throw new Error("未能根据 EPUB 样式确定内嵌字体实际使用的文字");
  const pathMap = new Map(Object.keys(zip.files).map((path) => [path, path]));
  let changedFonts = 0;
  let savedBytes = 0;
  for (const font of fonts) {
    const codePoints = usage.get(font);
    if (!codePoints?.size) continue;
    const path = font.path;
    try {
      const source = await zip.files[path].async("arraybuffer");
      const parsed = await parseFont(path, source, [...codePoints]);
      const written = writeFont(parsed);
      if (written.bytes.byteLength >= source.byteLength && parsed.type !== "otf") continue;
      let outputPath = path;
      if (parsed.type === "otf") {
        outputPath = path.replace(/\.otf$/i, ".ttf");
        if (zip.file(outputPath) && outputPath !== path) outputPath = path.replace(/\.otf$/i, "-subset.ttf");
        pathMap.set(path, outputPath);
        zip.remove(path);
      }
      zip.file(outputPath, written.bytes);
      savedBytes += Math.max(0, source.byteLength - written.bytes.byteLength);
      changedFonts += 1;
    } catch {
      // Keep unsupported fonts unchanged and continue with the remaining embedded fonts.
    }
  }
  if (!changedFonts) throw new Error("内嵌字体无需裁剪或当前字体格式无法子集化");

  if ([...pathMap].some(([from, to]) => from !== to)) {
    for (const path of Object.keys(zip.files).filter((name) => TEXT_EXTENSIONS.has(extension(name)))) {
      const source = await zip.files[path].async("text");
      zip.file(path, rewriteWebEpubTextLinks(source, path, path, pathMap));
    }
    const pkg = await packageInfo(zip);
    for (const item of elementsByLocalName(pkg.opf, "item")) {
      const href = percentDecode(item.getAttribute("href")?.split(/[?#]/, 1)[0] || "");
      if (joinPath(pkg.opfDir, href).toLowerCase().endsWith(".ttf")) item.setAttribute("media-type", "font/ttf");
    }
    zip.file(pkg.opfPath, new XMLSerializer().serializeToString(pkg.opf));
  }
  const blob = await generateEpub(zip);
  return {
    sourceName: file.name,
    outputName: sourceNameWithSuffix(file.name, "_font_subset"),
    action: "font-subset",
    changedFiles: 0,
    mappedCharacters: retainedCodePoints,
    changedFonts,
    mode: "unicode-subset",
    message: `字体子集化完成：按实际样式裁剪 ${changedFonts} 个字体，各字体合计保留 ${retainedCodePoints} 个码点，字体数据减少 ${(savedBytes / 1024 / 1024).toFixed(2)} MB`,
    blob,
  };
}

export async function processWebEpubFont(file: File, action: WebEpubFontAction, plainText?: string) {
  let zip: JSZip;
  try { zip = await JSZip.loadAsync(await file.arrayBuffer()); }
  catch (error) { throw new Error(`无法读取 EPUB：${String(error)}`); }
  if (action === "font-encrypt") return encryptBodyFontsIndependently(file, zip);
  if (action === "font-decrypt") return decryptFonts(file, zip, plainText);
  return subsetFonts(file, zip);
}

export const webEpubFontProcessTesting = {
  appendFontFallbacks,
  collectFontPrivateCodePoints,
  collectSubsetCodePoints,
  injectBodyFontStyle,
  injectObfuscatedTextCompatibility,
  privateCharacters,
  randomDerangement,
  rewriteCssFontFallbacks,
  transformBodyText,
};
