import JSZip from "jszip";

export type WebEpubProcessAction = "file-encrypt" | "file-decrypt" | "epub-reformat" | "image-convert";
export type WebImageFormat = "auto" | "png" | "jpeg";

export type WebEpubProcessOptions = {
  imageFormat?: WebImageFormat;
};

export type WebEpubProcessResult = {
  sourceName: string;
  outputName: string;
  action: WebEpubProcessAction;
  changed: boolean;
  message: string;
  processedEntries: number;
  blob: Blob;
};

type ManifestItem = {
  id: string;
  href: string;
  mediaType: string;
  absolutePath: string;
};

type PackageInfo = {
  opfPath: string;
  manifest: ManifestItem[];
};

type ImageConversion = {
  newPath: string;
  bytes: Uint8Array;
  mediaType: string;
};

const TEXT_EXTENSIONS = new Set(["xhtml", "html", "htm", "xml", "opf", "ncx", "css", "svg"]);
const WINDOWS_RESERVED = new Set([
  "CON", "PRN", "AUX", "NUL",
  "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9",
  "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
]);

function normalizePath(path: string) {
  const parts: string[] = [];
  for (const part of path.replace(/\\/g, "/").split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") parts.pop();
    else parts.push(part);
  }
  return parts.join("/");
}

function parentPath(path: string) {
  const index = path.lastIndexOf("/");
  return index >= 0 ? path.slice(0, index) : "";
}

function fileName(path: string) {
  return path.split("/").pop() || path;
}

function fileStem(path: string) {
  const name = fileName(path);
  const index = name.lastIndexOf(".");
  return index > 0 ? name.slice(0, index) : name;
}

function extension(path: string) {
  const name = fileName(path);
  const index = name.lastIndexOf(".");
  return index >= 0 ? name.slice(index + 1).toLowerCase() : "";
}

function joinPath(base: string, relative: string) {
  return relative.startsWith("/") ? normalizePath(relative.slice(1)) : normalizePath(`${base}/${relative}`);
}

function relativePath(fromFile: string, toFile: string) {
  const from = parentPath(fromFile).split("/").filter(Boolean);
  const to = toFile.split("/").filter(Boolean);
  let shared = 0;
  while (shared < from.length && shared < to.length && from[shared] === to[shared]) shared += 1;
  return [...from.slice(shared).map(() => ".."), ...to.slice(shared)].join("/") || ".";
}

function percentDecode(value: string) {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

function parseXml(source: string, label: string) {
  const document = new DOMParser().parseFromString(source, "application/xml");
  if (document.querySelector("parsererror")) throw new Error(`${label} XML 格式无效`);
  return document;
}

function elementsByLocalName(root: ParentNode, name: string) {
  return Array.from(root.querySelectorAll("*")).filter((element) => element.localName === name) as Element[];
}

async function readPackage(zip: JSZip): Promise<PackageInfo> {
  let opfPath = "";
  const container = zip.file("META-INF/container.xml");
  if (container) {
    const document = parseXml(await container.async("text"), "container.xml");
    opfPath = elementsByLocalName(document, "rootfile")[0]?.getAttribute("full-path")?.replace(/\\/g, "/") || "";
  }
  if (!opfPath) opfPath = Object.keys(zip.files).find((path) => path.toLowerCase().endsWith(".opf")) || "";
  if (!opfPath) throw new Error("无法定位 OPF 文件");
  const opfEntry = zip.file(opfPath);
  if (!opfEntry) throw new Error(`找不到 OPF 文件：${opfPath}`);
  const document = parseXml(await opfEntry.async("text"), "OPF");
  const opfDir = parentPath(opfPath);
  const manifest = elementsByLocalName(document, "item")
    .map((item) => {
      const id = item.getAttribute("id")?.trim() || "";
      const href = item.getAttribute("href")?.trim() || "";
      if (!id || !href) return null;
      return {
        id,
        href: percentDecode(href),
        mediaType: item.getAttribute("media-type")?.trim() || "",
        absolutePath: joinPath(opfDir, percentDecode(href.split(/[?#]/, 1)[0] || "")),
      };
    })
    .filter((item): item is ManifestItem => !!item);
  return { opfPath, manifest };
}

function isExternalReference(value: string) {
  const lower = value.trim().toLowerCase();
  return !lower || lower.startsWith("#") || /^(?:data|https?|mailto|javascript|urn):/.test(lower);
}

function splitReference(value: string) {
  const index = value.search(/[?#]/);
  return index < 0 ? [value, ""] as const : [value.slice(0, index), value.slice(index)] as const;
}

function buildCaseInsensitiveMap(pathMap: Map<string, string>) {
  const candidates = new Map<string, string[]>();
  for (const [oldPath, newPath] of pathMap) {
    const key = oldPath.toLowerCase();
    candidates.set(key, [...(candidates.get(key) || []), newPath]);
  }
  const result = new Map<string, string>();
  for (const [key, values] of candidates) {
    if (new Set(values).size === 1) result.set(key, values[0]);
  }
  return result;
}

function rewriteReference(
  rawValue: string,
  currentOldPath: string,
  currentNewPath: string,
  pathMap: Map<string, string>,
  lowerPathMap: Map<string, string>,
) {
  if (isExternalReference(rawValue)) return rawValue;
  const [main, suffix] = splitReference(rawValue);
  if (isExternalReference(main)) return rawValue;
  const decoded = percentDecode(main);
  const oldAbsolute = currentOldPath.toLowerCase() === "meta-inf/container.xml"
    ? normalizePath(decoded)
    : joinPath(parentPath(currentOldPath), decoded);
  const newAbsolute = pathMap.get(oldAbsolute) || lowerPathMap.get(oldAbsolute.toLowerCase());
  if (!newAbsolute || newAbsolute.endsWith("/")) return rawValue;
  let rewritten = currentOldPath.toLowerCase() === "meta-inf/container.xml"
    ? newAbsolute
    : relativePath(currentNewPath, newAbsolute);
  if (main.includes("%")) rewritten = encodeURI(rewritten);
  if (main.startsWith("./") && !rewritten.startsWith("./") && !rewritten.startsWith("../")) rewritten = `./${rewritten}`;
  return `${rewritten}${suffix}`;
}

export function rewriteWebEpubTextLinks(text: string, oldPath: string, newPath: string, pathMap: Map<string, string>) {
  const lowerPathMap = buildCaseInsensitiveMap(pathMap);
  const quoted = /(["'])(.*?)\1/gis;
  const rewrittenQuoted = text.replace(quoted, (full, quote: string, value: string) => {
    const rewritten = rewriteReference(value, oldPath, newPath, pathMap, lowerPathMap);
    return rewritten === value ? full : `${quote}${rewritten}${quote}`;
  });
  return rewrittenQuoted.replace(/url\(\s*([^'"\)\s][^)]*?)\s*\)/gis, (full, value: string) => {
    const trimmed = value.trim();
    const rewritten = rewriteReference(trimmed, oldPath, newPath, pathMap, lowerPathMap);
    return rewritten === trimmed ? full : `url(${rewritten})`;
  });
}

function sanitizeComponent(part: string) {
  let output = [...part].map((character) => /[<>:"\\|?*]/.test(character) || character.charCodeAt(0) < 32 ? "_" : character).join("");
  output = output.replace(/[ .]+$/g, "") || "_";
  if (WINDOWS_RESERVED.has(output.toUpperCase())) output += "_";
  return output;
}

function sanitizeZipPath(path: string) {
  const isDirectory = path.replace(/\\/g, "/").endsWith("/");
  const normalized = path.replace(/\\/g, "/").replace(/^\/+/, "");
  const clean = normalized.split("/").filter((part) => part && part !== "." && part !== "..").map(sanitizeComponent).join("/") || "_";
  return isDirectory ? `${clean}/` : clean;
}

function hasInvalidComponent(path: string) {
  return path.replace(/\\/g, "/").split("/").some((raw) => {
    const part = raw.trim();
    return part === ".." || /[<>:"\\|?*]/.test(part) || /[ .]$/.test(raw) || [...part].some((character) => character.charCodeAt(0) < 32);
  });
}

function looksObfuscated(path: string) {
  const stem = fileStem(path);
  if (!stem) return true;
  const characters = [...stem];
  const allowed = characters.filter((character) => /[A-Za-z0-9_-]/.test(character) || /[\u4e00-\u9fff]/.test(character)).length;
  return 1 - allowed / characters.length > 0.35 || stem.includes("____") || stem.length > 40;
}

function ensureUniquePath(path: string, used: Set<string>) {
  if (!used.has(path)) {
    used.add(path);
    return path;
  }
  const isDirectory = path.endsWith("/");
  const bare = isDirectory ? path.slice(0, -1) : path;
  const dot = isDirectory ? -1 : bare.lastIndexOf(".");
  const stem = dot > bare.lastIndexOf("/") ? bare.slice(0, dot) : bare;
  const ext = dot > bare.lastIndexOf("/") ? bare.slice(dot) : "";
  let index = 2;
  while (true) {
    const candidate = `${stem}_${index}${ext}${isDirectory ? "/" : ""}`;
    if (!used.has(candidate)) {
      used.add(candidate);
      return candidate;
    }
    index += 1;
  }
}

function friendlyName(path: string, index: number) {
  const ext = extension(path);
  const prefix = ["xhtml", "html", "htm"].includes(ext) ? "chapter"
    : ext === "css" ? "style"
      : ["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg"].includes(ext) ? "image"
        : ["ttf", "otf", "woff", "woff2"].includes(ext) ? "font"
          : ext === "ncx" ? "toc" : ext === "opf" ? "content" : ext === "xml" ? "meta" : "file";
  return `${prefix}${String(index).padStart(3, "0")}${ext ? `.${ext}` : ""}`;
}

function nameFromManifestId(item: ManifestItem) {
  const hrefExt = extension(item.href) || "bin";
  const rawIdName = fileName(item.id);
  const dot = rawIdName.lastIndexOf(".");
  let stem = dot > 0 ? rawIdName.slice(0, dot) : rawIdName;
  const idExt = dot > 0 ? rawIdName.slice(dot + 1).toLowerCase() : "";
  const slim = /(?:~|-|_)?slim$/i.test(stem) || /(?:~|-|_)?slim$/i.test(fileStem(item.href));
  stem = stem.replace(/(?:~|-|_)?slim$/i, "") || "file";
  const ext = idExt && idExt === hrefExt ? idExt : hrefExt;
  return `${sanitizeComponent(stem)}${slim ? "~slim" : ""}.${ext}`;
}

function leftRotate(value: number, count: number) {
  return (value << count) | (value >>> (32 - count));
}

function md5(input: string) {
  const source = new TextEncoder().encode(input);
  const paddedLength = Math.ceil((source.length + 9) / 64) * 64;
  const bytes = new Uint8Array(paddedLength);
  bytes.set(source);
  bytes[source.length] = 0x80;
  const view = new DataView(bytes.buffer);
  const bitLength = source.length * 8;
  view.setUint32(paddedLength - 8, bitLength >>> 0, true);
  view.setUint32(paddedLength - 4, Math.floor(bitLength / 0x100000000), true);

  let a0 = 0x67452301;
  let b0 = 0xefcdab89;
  let c0 = 0x98badcfe;
  let d0 = 0x10325476;
  const shifts = [7,12,17,22, 5,9,14,20, 4,11,16,23, 6,10,15,21];
  const constants = Array.from({ length: 64 }, (_, index) => Math.floor(Math.abs(Math.sin(index + 1)) * 0x100000000) >>> 0);

  for (let offset = 0; offset < paddedLength; offset += 64) {
    const words = Array.from({ length: 16 }, (_, index) => view.getUint32(offset + index * 4, true));
    let a = a0, b = b0, c = c0, d = d0;
    for (let index = 0; index < 64; index += 1) {
      let f: number;
      let g: number;
      if (index < 16) { f = (b & c) | (~b & d); g = index; }
      else if (index < 32) { f = (d & b) | (~d & c); g = (5 * index + 1) % 16; }
      else if (index < 48) { f = b ^ c ^ d; g = (3 * index + 5) % 16; }
      else { f = c ^ (b | ~d); g = (7 * index) % 16; }
      const previousD = d;
      d = c;
      c = b;
      const shift = shifts[Math.floor(index / 16) * 4 + (index % 4)];
      b = (b + leftRotate((a + f + constants[index] + words[g]) | 0, shift)) | 0;
      a = previousD;
    }
    a0 = (a0 + a) | 0;
    b0 = (b0 + b) | 0;
    c0 = (c0 + c) | 0;
    d0 = (d0 + d) | 0;
  }
  const output = new Uint8Array(16);
  const outputView = new DataView(output.buffer);
  [a0, b0, c0, d0].forEach((value, index) => outputView.setUint32(index * 4, value >>> 0, true));
  return output;
}

function encryptedBasename(item: ManifestItem) {
  const ext = extension(item.href) || "bin";
  const idStem = item.id.split(".")[0] || item.id;
  const slim = /(?:~|-|_)?slim$/i.test(idStem) || /(?:~|-|_)?slim$/i.test(fileStem(item.href));
  let bits = "_";
  for (const byte of md5(idStem)) {
    for (let shift = 7; shift >= 0; shift -= 1) bits += ((byte >>> shift) & 1) ? "*" : ":";
  }
  return `${bits}${slim ? "~slim" : ""}.${ext}`;
}

function mediaTypeForPath(path: string) {
  const ext = extension(path);
  const types: Record<string, string> = {
    xhtml: "application/xhtml+xml", html: "text/html", htm: "text/html", css: "text/css", svg: "image/svg+xml",
    png: "image/png", jpg: "image/jpeg", jpeg: "image/jpeg", webp: "image/webp", gif: "image/gif", bmp: "image/bmp",
    ttf: "font/ttf", otf: "font/otf", woff: "font/woff", woff2: "font/woff2", ncx: "application/x-dtbncx+xml",
    js: "text/javascript", mp3: "audio/mpeg", m4a: "audio/mp4", aac: "audio/aac", ogg: "audio/ogg", opus: "audio/ogg",
    wav: "audio/wav", mp4: "video/mp4", webm: "video/webm",
  };
  return types[ext] || "";
}

function reformatTarget(path: string, item?: ManifestItem) {
  const lower = path.toLowerCase();
  if (lower === "mimetype" || lower.startsWith("meta-inf/")) return path;
  if (lower.endsWith(".opf")) return "OEBPS/content.opf";
  if (lower.endsWith(".ncx")) return "OEBPS/toc.ncx";
  const media = item?.mediaType.toLowerCase() || "";
  const ext = extension(path);
  const category = media === "application/xhtml+xml" || ["xhtml", "html", "htm"].includes(ext) ? "Text"
    : media === "text/css" || ext === "css" ? "Styles"
      : media.startsWith("image/") || ["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg"].includes(ext) ? "Images"
        : media.startsWith("font/") || ["ttf", "otf", "woff", "woff2"].includes(ext) ? "Fonts"
          : media.startsWith("audio/") || ["mp3", "m4a", "aac", "ogg", "opus", "wav"].includes(ext) ? "Audio"
            : media.startsWith("video/") || ["mp4", "webm"].includes(ext) ? "Video" : "Misc";
  return `OEBPS/${category}/${sanitizeComponent(fileName(path))}`;
}

function outputName(sourceName: string, suffix: string) {
  const stem = sourceName.toLowerCase().endsWith(".epub") ? sourceName.slice(0, -5) : sourceName;
  return `${stem}${suffix}.epub`;
}

async function buildMappedBlob(
  zip: JSZip,
  pathMap: Map<string, string>,
  options: {
    additions?: Map<string, Array<{ id: string; href: string; mediaType: string }>>;
    conversions?: Map<string, ImageConversion>;
    opfPath?: string;
  } = {},
) {
  const output = new JSZip();
  for (const [oldPath, entry] of Object.entries(zip.files)) {
    if (entry.dir) continue;
    const conversion = options.conversions?.get(oldPath);
    const newPath = conversion?.newPath || pathMap.get(oldPath) || oldPath;
    if (conversion) {
      output.file(newPath, conversion.bytes);
      continue;
    }
    if (TEXT_EXTENSIONS.has(extension(oldPath))) {
      let text = await entry.async("text");
      text = rewriteWebEpubTextLinks(text, oldPath, newPath, pathMap);
      if (options.opfPath === oldPath && options.conversions?.size) {
        const document = parseXml(text, "OPF");
        const newOpfDir = parentPath(newPath);
        const convertedByNewPath = new Map([...options.conversions.values()].map((item) => [item.newPath, item]));
        for (const item of elementsByLocalName(document, "item")) {
          const href = percentDecode(item.getAttribute("href")?.split(/[?#]/, 1)[0] || "");
          const absolute = joinPath(newOpfDir, href);
          const converted = convertedByNewPath.get(absolute);
          if (converted) item.setAttribute("media-type", converted.mediaType);
        }
        text = new XMLSerializer().serializeToString(document);
      }
      const additions = options.additions?.get(newPath) || [];
      if (additions.length) {
        const block = additions.map((item) => `\n    <item id="${item.id}" href="${item.href}" media-type="${item.mediaType}"/>`).join("");
        text = text.replace(/<\/manifest>/i, `${block}\n  </manifest>`);
      }
      output.file(newPath, text);
    } else {
      output.file(newPath, await entry.async("uint8array"));
    }
  }
  output.file("mimetype", "application/epub+zip", { compression: "STORE", createFolders: false });
  return output.generateAsync({ type: "blob", mimeType: "application/epub+zip", compression: "DEFLATE", compressionOptions: { level: 6 } });
}

async function canvasConvertWebp(entry: JSZip.JSZipObject, requestedFormat: WebImageFormat) {
  const sourceBlob = await entry.async("blob");
  const bitmap = await createImageBitmap(sourceBlob);
  const canvas = document.createElement("canvas");
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  const context = canvas.getContext("2d", { willReadFrequently: requestedFormat === "auto" });
  if (!context) throw new Error("浏览器无法创建图片转换画布");
  context.drawImage(bitmap, 0, 0);
  bitmap.close();
  let format = requestedFormat;
  if (format === "auto") {
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let hasAlpha = false;
    for (let index = 3; index < pixels.length; index += 4) {
      if (pixels[index] < 255) { hasAlpha = true; break; }
    }
    format = hasAlpha ? "png" : "jpeg";
  }
  if (format === "jpeg") {
    const flattened = document.createElement("canvas");
    flattened.width = canvas.width;
    flattened.height = canvas.height;
    const flattenedContext = flattened.getContext("2d");
    if (!flattenedContext) throw new Error("浏览器无法创建 JPEG 转换画布");
    flattenedContext.fillStyle = "#ffffff";
    flattenedContext.fillRect(0, 0, flattened.width, flattened.height);
    flattenedContext.drawImage(canvas, 0, 0);
    const blob = await new Promise<Blob>((resolve, reject) => flattened.toBlob((value) => value ? resolve(value) : reject(new Error("JPEG 编码失败")), "image/jpeg", 0.9));
    return { bytes: new Uint8Array(await blob.arrayBuffer()), ext: "jpg", mediaType: "image/jpeg" };
  }
  const blob = await new Promise<Blob>((resolve, reject) => canvas.toBlob((value) => value ? resolve(value) : reject(new Error("PNG 编码失败")), "image/png"));
  return { bytes: new Uint8Array(await blob.arrayBuffer()), ext: "png", mediaType: "image/png" };
}

async function processFileEncrypt(file: File, zip: JSZip, pkg: PackageInfo): Promise<WebEpubProcessResult> {
  if (!pkg.manifest.length) throw new Error("OPF manifest 为空，无法执行文件加密");
  const entries = Object.keys(zip.files);
  const byPath = new Map(pkg.manifest.map((item) => [item.absolutePath, item]));
  const used = new Set(entries.filter((path) => !byPath.has(path)));
  const pathMap = new Map<string, string>();
  let changed = false;
  for (const path of entries) {
    const item = byPath.get(path);
    if (!item || zip.files[path].dir) { pathMap.set(path, path); continue; }
    const parent = parentPath(path);
    const target = `${parent ? `${parent}/` : ""}${encryptedBasename(item)}`;
    const unique = ensureUniquePath(target, used);
    pathMap.set(path, unique);
    if (unique !== path) changed = true;
  }
  const blob = changed ? await buildMappedBlob(zip, pathMap) : file;
  return { sourceName: file.name, outputName: outputName(file.name, "_encrypt"), action: "file-encrypt", changed, message: changed ? "文件名混淆完成" : "未检测到可混淆的 manifest 文件项", processedEntries: pkg.manifest.length, blob };
}

async function processFileDecrypt(file: File, zip: JSZip, pkg: PackageInfo): Promise<WebEpubProcessResult> {
  const hints = new Map<string, string>();
  for (const item of pkg.manifest) {
    hints.set(item.absolutePath, nameFromManifestId(item));
    hints.set(item.absolutePath.toLowerCase(), nameFromManifestId(item));
  }
  const used = new Set<string>();
  const pathMap = new Map<string, string>();
  let friendlyIndex = 1;
  let changed = false;
  for (const path of Object.keys(zip.files)) {
    let clean = sanitizeZipPath(path);
    const invalid = hasInvalidComponent(path);
    const obfuscated = !zip.files[path].dir && looksObfuscated(path);
    if (!zip.files[path].dir && (invalid || obfuscated)) {
      const parent = parentPath(clean);
      const hint = hints.get(path) || hints.get(path.toLowerCase());
      const nextName = hint || (obfuscated ? friendlyName(clean, friendlyIndex++) : fileName(clean));
      clean = `${parent ? `${parent}/` : ""}${nextName}`;
    }
    const unique = ensureUniquePath(clean, used);
    pathMap.set(path, unique);
    if (unique !== path) changed = true;
  }
  const blob = changed ? await buildMappedBlob(zip, pathMap) : file;
  return { sourceName: file.name, outputName: outputName(file.name, "_decrypt"), action: "file-decrypt", changed, message: changed ? "混淆文件名已恢复并同步修复引用" : "未检测到文件名混淆，无需处理", processedEntries: pathMap.size, blob };
}

async function processReformat(file: File, zip: JSZip, pkg: PackageInfo): Promise<WebEpubProcessResult> {
  const entries = Object.keys(zip.files);
  const byPath = new Map(pkg.manifest.map((item) => [item.absolutePath, item]));
  const used = new Set<string>();
  const pathMap = new Map<string, string>();
  let changed = false;
  for (const path of entries) {
    const target = zip.files[path].dir ? sanitizeZipPath(path) : sanitizeZipPath(reformatTarget(path, byPath.get(path)));
    const unique = ensureUniquePath(target, used);
    pathMap.set(path, unique);
    if (unique !== path) changed = true;
  }
  const newOpfPath = pathMap.get(pkg.opfPath) || pkg.opfPath;
  const manifestPaths = new Set(pkg.manifest.map((item) => item.absolutePath));
  const usedIds = new Set(pkg.manifest.map((item) => item.id));
  const additions: Array<{ id: string; href: string; mediaType: string }> = [];
  for (const oldPath of entries) {
    if (zip.files[oldPath].dir || manifestPaths.has(oldPath) || oldPath.toLowerCase() === "mimetype" || oldPath.toLowerCase().startsWith("meta-inf/") || oldPath.toLowerCase().endsWith(".opf")) continue;
    const mediaType = mediaTypeForPath(oldPath);
    if (!mediaType) continue;
    const newPath = pathMap.get(oldPath) || oldPath;
    let id = `te-extra-${sanitizeComponent(fileStem(newPath)).replace(/^[^A-Za-z_]+/, "") || `resource-${additions.length + 1}`}`;
    const base = id;
    let suffix = 2;
    while (usedIds.has(id)) id = `${base}-${suffix++}`;
    usedIds.add(id);
    additions.push({ id, href: relativePath(newOpfPath, newPath), mediaType });
  }
  const additionsByOpf = new Map<string, Array<{ id: string; href: string; mediaType: string }>>();
  if (additions.length) additionsByOpf.set(newOpfPath, additions);
  const hasChanges = changed || additions.length > 0;
  const blob = hasChanges ? await buildMappedBlob(zip, pathMap, { additions: additionsByOpf }) : file;
  return { sourceName: file.name, outputName: outputName(file.name, "_reformat"), action: "epub-reformat", changed: hasChanges, message: hasChanges ? `EPUB 结构已重构${additions.length ? `，补登记 ${additions.length} 个资源` : ""}` : "EPUB 已经是规范结构，无需重构", processedEntries: pathMap.size, blob };
}

async function processImageConvert(file: File, zip: JSZip, pkg: PackageInfo, format: WebImageFormat): Promise<WebEpubProcessResult> {
  const entries = Object.keys(zip.files);
  const webpPaths = entries.filter((path) => !zip.files[path].dir && path.toLowerCase().endsWith(".webp"));
  if (!webpPaths.length) return { sourceName: file.name, outputName: outputName(file.name, "_transfer"), action: "image-convert", changed: false, message: "未发现需要转换的 WebP 图片", processedEntries: 0, blob: file };
  const used = new Set(entries.filter((path) => !path.toLowerCase().endsWith(".webp")));
  const pathMap = new Map(entries.map((path) => [path, path]));
  const conversions = new Map<string, ImageConversion>();
  for (const path of webpPaths) {
    const converted = await canvasConvertWebp(zip.files[path], format);
    const parent = parentPath(path);
    const target = `${parent ? `${parent}/` : ""}${fileStem(path)}.${converted.ext}`;
    const unique = ensureUniquePath(target, used);
    pathMap.set(path, unique);
    conversions.set(path, { newPath: unique, bytes: converted.bytes, mediaType: converted.mediaType });
  }
  const blob = await buildMappedBlob(zip, pathMap, { conversions, opfPath: pkg.opfPath });
  return { sourceName: file.name, outputName: outputName(file.name, "_transfer"), action: "image-convert", changed: true, message: `图片转换完成，共转换 ${conversions.size} 张 WebP`, processedEntries: conversions.size, blob };
}

export async function processWebEpub(file: File, action: WebEpubProcessAction, options: WebEpubProcessOptions = {}) {
  let zip: JSZip;
  try {
    zip = await JSZip.loadAsync(await file.arrayBuffer());
  } catch (error) {
    throw new Error(`无法读取 EPUB：${String(error)}`);
  }
  const pkg = await readPackage(zip);
  if (action === "file-encrypt") return processFileEncrypt(file, zip, pkg);
  if (action === "file-decrypt") return processFileDecrypt(file, zip, pkg);
  if (action === "epub-reformat") return processReformat(file, zip, pkg);
  return processImageConvert(file, zip, pkg, options.imageFormat || "auto");
}
