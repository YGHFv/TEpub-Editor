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

async function encryptFonts(file: File, zip: JSZip): Promise<WebEpubFontProcessResult> {
  const fonts = await loadFonts(zip);
  const supported = collectFontPlainCodePoints(fonts);
  if (!supported.size) throw new Error("内嵌字体未发现可加密汉字");
  const htmlPaths = Object.keys(zip.files).filter((path) => !zip.files[path].dir && HTML_EXTENSIONS.has(extension(path)));
  if (!htmlPaths.length) throw new Error("EPUB 内未找到 HTML/XHTML 文件");

  const seen = new Set<string>();
  const characters: string[] = [];
  const existingPrivate = new Set<number>();
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
    const transformed = transformHtmlText(source, mapping);
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
  const bestOffset = [...offsets].sort((a, b) => b[1] - a[1])[0]?.[0] || 0;
  const counts = new Map<string, Map<string, number>>();
  for (let cipherIndex = 0; cipherIndex < cipher.length; cipherIndex += 1) {
    const plainIndex = cipherIndex + bestOffset;
    if (plainIndex < 0 || plainIndex >= plain.length) continue;
    const cipherCharacter = cipher[cipherIndex];
    const plainCharacter = plain[plainIndex];
    if (!isPrivateCodePoint(cipherCharacter.codePointAt(0) || 0) || !shouldMapCharacter(plainCharacter)) continue;
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
  for (const path of paths) chunks.push(await zip.files[path].async("text"));
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
      let transformed = transformHtmlText(source, mapping);
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
  const codePoints = new Set<number>();
  for (let code = 32; code <= 126; code += 1) codePoints.add(code);
  for (const path of Object.keys(zip.files).filter((name) => !zip.files[name].dir && HTML_EXTENSIONS.has(extension(name)))) {
    collectSubsetCodePoints(await zip.files[path].async("text"), codePoints);
  }
  if (codePoints.size <= 95) throw new Error("EPUB 正文中没有可用于字体子集化的文字");

  const subset = [...codePoints];
  const fontPaths = Object.keys(zip.files).filter((path) => !zip.files[path].dir && FONT_EXTENSIONS.has(extension(path)));
  if (!fontPaths.length) throw new Error("EPUB 内未找到 TTF、OTF、WOFF 或 WOFF2 字体");
  const pathMap = new Map(Object.keys(zip.files).map((path) => [path, path]));
  let changedFonts = 0;
  let savedBytes = 0;
  for (const path of fontPaths) {
    try {
      const source = await zip.files[path].async("arraybuffer");
      const parsed = await parseFont(path, source, subset);
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
    mappedCharacters: codePoints.size,
    changedFonts,
    mode: "unicode-subset",
    message: `字体子集化完成：裁剪 ${changedFonts} 个字体，保留 ${codePoints.size} 个码点，字体数据减少 ${(savedBytes / 1024 / 1024).toFixed(2)} MB`,
    blob,
  };
}

export async function processWebEpubFont(file: File, action: WebEpubFontAction, plainText?: string) {
  let zip: JSZip;
  try { zip = await JSZip.loadAsync(await file.arrayBuffer()); }
  catch (error) { throw new Error(`无法读取 EPUB：${String(error)}`); }
  if (action === "font-encrypt") return encryptFonts(file, zip);
  if (action === "font-decrypt") return decryptFonts(file, zip, plainText);
  return subsetFonts(file, zip);
}

export const webEpubFontProcessTesting = { collectSubsetCodePoints };
