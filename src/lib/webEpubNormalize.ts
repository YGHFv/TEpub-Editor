/**
 * 浏览器端的 EPUB「预清洗」：让 web 版和安装版看到同一本书。
 *
 * 安装版打开图书前会走 `prepare_epub_for_open`（src-tauri/src/epub.rs）：
 * 解除伪加密标记、尝试 XOR 解混淆、把混淆文件名恢复成可读名字并同步改写
 * 引用。web 版此前直接 `JSZip.loadAsync(file)`，于是：
 *
 * - 伪加密书（ZIP 头 flag 位 0x0001 被置上）：JSZip 读中央目录时直接抛
 *   `Encrypted zip are not supported`，整本书打不开；
 * - XOR 混淆书：连 ZIP 签名都没有，报 `Can't find end of central directory`；
 * - 只混淆文件名的书：能打开，但文件名是一串 `_*:*::*...`，封面识别、
 *   目录标题、按名字找资源全部退化。
 *
 * 这里把三件事都补上，且在**字节层**先修好 flag 再交给 JSZip —— JSZip
 * 只检查中央目录里的 flag（zipEntry.js 的 readCentralPart），本地头不看，
 * 所以把两处都清掉既安全又必要（与 Rust 版 clear_zip_encryption_flags 一致）。
 *
 * 命名恢复规则、friendly 名字、manifest id → 文件名、引用改写全部照搬
 * Rust 实现，避免同一本书在两端清洗出不同结果。
 */
import JSZip from "jszip";

export type EpubByteRepair = {
  bytes: Uint8Array;
  /** 清掉过伪加密标记 */
  repairedEncryption: boolean;
  /** 解 XOR 用的密钥；null 表示没走解混淆 */
  xorKey: number | null;
};

export type EpubArchiveNormalization = {
  zip: JSZip;
  /** 有没有改名 / 重建过归档 */
  changed: boolean;
  renamedEntries: number;
  /** 重建后各条目的真实字节数（JSZip 的 size 元数据在新归档里就没了） */
  sizes: Map<string, number>;
  /** 旧路径 → 新路径（没改名时是恒等映射）。OPF 自己的路径也要靠它换算 */
  pathMap: Map<string, string>;
};

const TEXT_EXTENSIONS = new Set([
  "xhtml",
  "html",
  "htm",
  "xml",
  "opf",
  "ncx",
  "css",
  "svg",
]);

/* ------------------------------------------------------------------ */
/* 路径工具                                                            */
/* ------------------------------------------------------------------ */

export function normalizeEntryPath(path: string) {
  const parts: string[] = [];
  for (const part of path.replace(/\\/g, "/").split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") parts.pop();
    else parts.push(part);
  }
  return parts.join("/");
}

/** 父目录，不带结尾斜杠（"" 表示根） */
export function parentOf(path: string) {
  const index = path.lastIndexOf("/");
  return index >= 0 ? path.slice(0, index) : "";
}

export function baseName(path: string) {
  return path.split("/").pop() || path;
}

export function stemOf(path: string) {
  const name = baseName(path);
  const index = name.lastIndexOf(".");
  return index > 0 ? name.slice(0, index) : name;
}

export function extOf(path: string) {
  const name = baseName(path);
  const index = name.lastIndexOf(".");
  return index >= 0 ? name.slice(index + 1).toLowerCase() : "";
}

export function joinEntryPath(base: string, relative: string) {
  return relative.startsWith("/")
    ? normalizeEntryPath(relative.slice(1))
    : normalizeEntryPath(`${base}/${relative}`);
}

export function relativeEntryPath(fromFile: string, toFile: string) {
  const from = parentOf(fromFile).split("/").filter(Boolean);
  const to = toFile.split("/").filter(Boolean);
  let shared = 0;
  while (
    shared < from.length &&
    shared < to.length &&
    from[shared] === to[shared]
  )
    shared += 1;
  return (
    [...from.slice(shared).map(() => ".."), ...to.slice(shared)].join("/") ||
    "."
  );
}

export function percentDecode(value: string) {
  try {
    return decodeURIComponent(value);
  } catch {
    return value;
  }
}

export function isTextLikeEntry(path: string) {
  return TEXT_EXTENSIONS.has(extOf(path));
}

/* ------------------------------------------------------------------ */
/* ZIP 字节：伪加密标记 / XOR 解混淆                                    */
/* ------------------------------------------------------------------ */

const ZIP_LOCAL_SIGNATURE = 0x04034b50;
const ZIP_CENTRAL_SIGNATURE = 0x02014b50;
const ZIP_EOCD_SIGNATURE = 0x06054b50;

export function hasZipSignature(bytes: Uint8Array) {
  return (
    bytes.length >= 4 &&
    bytes[0] === 0x50 &&
    bytes[1] === 0x4b &&
    bytes[2] === 0x03 &&
    bytes[3] === 0x04
  );
}

function readU32(bytes: Uint8Array, offset: number) {
  return (
    (bytes[offset] |
      (bytes[offset + 1] << 8) |
      (bytes[offset + 2] << 16) |
      (bytes[offset + 3] << 24)) >>>
    0
  );
}

function readU16(bytes: Uint8Array, offset: number) {
  return bytes[offset] | (bytes[offset + 1] << 8);
}

function findEndOfCentralDirectory(bytes: Uint8Array) {
  // EOCD 之后最多 65535 字节注释；从尾部往前找签名
  const start = Math.max(0, bytes.length - 65557);
  for (let offset = bytes.length - 22; offset >= start; offset -= 1) {
    if (readU32(bytes, offset) === ZIP_EOCD_SIGNATURE) return offset;
  }
  return -1;
}

/**
 * 清掉 ZIP 条目头里的「加密」标志位。
 *
 * 国内不少书把 flag 的 bit0 置上来伪装加密（实际内容没加密），安装版也是
 * 直接清位。按结构走（EOCD → 中央目录 → 各本地头），不在整块字节里瞎扫 ——
 * 压缩数据里偶尔出现 50 4B 03 04 时，瞎扫会把内容改坏。
 */
export function clearZipEncryptionFlags(bytes: Uint8Array) {
  const eocd = findEndOfCentralDirectory(bytes);
  if (eocd < 0) return false;
  const count = readU16(bytes, eocd + 10);
  const centralOffset = readU32(bytes, eocd + 16);
  let changed = false;
  let offset = centralOffset;
  for (let index = 0; index < count; index += 1) {
    if (
      offset + 46 > bytes.length ||
      readU32(bytes, offset) !== ZIP_CENTRAL_SIGNATURE
    )
      break;
    const flagsOffset = offset + 8;
    if (readU16(bytes, flagsOffset) & 0x0001) {
      bytes[flagsOffset] = bytes[flagsOffset] & 0xfe;
      changed = true;
    }
    const localOffset = readU32(bytes, offset + 42);
    if (
      localOffset + 8 <= bytes.length &&
      readU32(bytes, localOffset) === ZIP_LOCAL_SIGNATURE &&
      readU16(bytes, localOffset + 6) & 0x0001
    ) {
      bytes[localOffset + 6] = bytes[localOffset + 6] & 0xfe;
      changed = true;
    }
    const nameLength = readU16(bytes, offset + 28);
    const extraLength = readU16(bytes, offset + 30);
    const commentLength = readU16(bytes, offset + 32);
    offset += 46 + nameLength + extraLength + commentLength;
  }
  return changed;
}

export function xorBytes(bytes: Uint8Array, key: number) {
  const output = new Uint8Array(bytes.length);
  for (let index = 0; index < bytes.length; index += 1)
    output[index] = bytes[index] ^ key;
  return output;
}

/**
 * 把上传的字节变成 JSZip 能吃的字节。
 *
 * 先认 ZIP 签名；不是 ZIP 就按安装版那套密钥试 XOR（0xFF/0xA5/0x5A，外加
 * 由 PK 头反推出来的密钥）。都不成立就原样返回，交给 JSZip 报错。
 */
export function repairEpubContainerBytes(input: Uint8Array): EpubByteRepair {
  if (hasZipSignature(input)) {
    const bytes = input.slice();
    const repairedEncryption = clearZipEncryptionFlags(bytes);
    return { bytes, repairedEncryption, xorKey: null };
  }

  const keys = [0xff, 0xa5, 0x5a];
  if (input.length >= 4) {
    const derived = input[0] ^ 0x50;
    if (
      (input[1] ^ 0x4b) === derived &&
      (input[2] ^ 0x03) === derived &&
      (input[3] ^ 0x04) === derived &&
      !keys.includes(derived)
    ) {
      keys.unshift(derived);
    }
  }
  for (const key of keys) {
    const decoded = xorBytes(input, key);
    if (!hasZipSignature(decoded)) continue;
    const repairedEncryption = clearZipEncryptionFlags(decoded);
    return { bytes: decoded, repairedEncryption, xorKey: key };
  }
  return { bytes: input, repairedEncryption: false, xorKey: null };
}

/* ------------------------------------------------------------------ */
/* 混淆文件名                                                          */
/* ------------------------------------------------------------------ */

const WINDOWS_RESERVED = new Set([
  "CON",
  "PRN",
  "AUX",
  "NUL",
  "COM1",
  "COM2",
  "COM3",
  "COM4",
  "COM5",
  "COM6",
  "COM7",
  "COM8",
  "COM9",
  "LPT1",
  "LPT2",
  "LPT3",
  "LPT4",
  "LPT5",
  "LPT6",
  "LPT7",
  "LPT8",
  "LPT9",
]);

export function sanitizeComponent(part: string) {
  let output = [...part]
    .map((character) =>
      /[<>:"\\|?*]/.test(character) || character.charCodeAt(0) < 32
        ? "_"
        : character,
    )
    .join("");
  output = output.replace(/[ .]+$/g, "") || "_";
  if (WINDOWS_RESERVED.has(output.toUpperCase())) output += "_";
  return output;
}

export function sanitizeEntryPath(path: string) {
  const isDirectory = path.replace(/\\/g, "/").endsWith("/");
  const normalized = path.replace(/\\/g, "/").replace(/^\/+/, "");
  const clean =
    normalized
      .split("/")
      .filter((part) => part && part !== "." && part !== "..")
      .map(sanitizeComponent)
      .join("/") || "_";
  return isDirectory ? `${clean}/` : clean;
}

export function hasInvalidComponent(path: string) {
  return path
    .replace(/\\/g, "/")
    .split("/")
    .some((raw) => {
      const part = raw.trim();
      return (
        part === ".." ||
        /[<>:"\\|?*]/.test(part) ||
        /[ .]$/.test(raw) ||
        [...part].some((character) => character.charCodeAt(0) < 32)
      );
    });
}

/** 与 Rust looks_obfuscated_basename 同一套判据 */
export function isObfuscatedEntryName(path: string) {
  const stem = stemOf(path);
  if (!stem) return true;
  const characters = [...stem];
  const allowed = characters.filter(
    (character) =>
      /[A-Za-z0-9_-]/.test(character) || /[\u4e00-\u9fff]/.test(character),
  ).length;
  return (
    1 - allowed / characters.length > 0.35 ||
    stem.includes("____") ||
    stem.length > 40
  );
}

export function friendlyEntryName(path: string, index: number) {
  const ext = extOf(path);
  const prefix = ["xhtml", "html", "htm"].includes(ext)
    ? "chapter"
    : ext === "css"
      ? "style"
      : ["jpg", "jpeg", "png", "gif", "webp", "bmp", "svg"].includes(ext)
        ? "image"
        : ["ttf", "otf", "woff", "woff2"].includes(ext)
          ? "font"
          : ext === "ncx"
            ? "toc"
            : ext === "opf"
              ? "content"
              : ext === "xml"
                ? "meta"
                : "file";
  return `${prefix}${String(index).padStart(3, "0")}${ext ? `.${ext}` : ""}`;
}

/** Rust `Path::file_name`：反斜杠也算分隔符，结尾斜杠忽略 */
function rustFileName(path: string) {
  const trimmed = path.replace(/\\/g, "/").replace(/\/+$/, "");
  return trimmed.split("/").pop() || "";
}

/** Rust `Path::extension`：`.hidden` 这种以点开头的文件名没有扩展名 */
function rustExtOf(name: string) {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(dot + 1).toLowerCase() : "";
}

/**
 * manifest 的 id 往往还是可读的（`Chapter0001.xhtml`、`cover.jpg`），
 * 用 id 反推文件名 —— 与 Rust build_name_from_id 逐条对齐：
 * 带 slim 的图片保留 `~slim` 后缀（缩略图不能顶替正图），
 * id 的扩展名与 href 冲突时以 href 为准，href 没有扩展名时才补 bin。
 */
export function nameFromManifestId(id: string, href: string) {
  const hrefName = rustFileName(href) || "file";
  const hrefExt = rustExtOf(hrefName);
  const idName = rustFileName(id) || id;

  const dot = idName.lastIndexOf(".");
  let stem = dot >= 0 ? idName.slice(0, dot) : idName;
  const idExt = dot >= 0 ? idName.slice(dot + 1).toLowerCase() : "";

  let slim = "";
  const lowerStem = stem.toLowerCase();
  if (lowerStem.includes("slim") || hrefName.toLowerCase().includes("slim")) {
    for (const suffix of ["~slim", "-slim", "_slim", "slim"]) {
      if (lowerStem.endsWith(suffix)) {
        stem = stem.slice(0, stem.length - suffix.length);
        break;
      }
    }
    slim = "~slim";
  }
  if (!stem) stem = "file";

  const ext = idExt
    ? hrefExt && idExt !== hrefExt
      ? hrefExt
      : idExt
    : hrefExt || "bin";
  return `${sanitizeComponent(stem)}${slim}.${ext}`;
}

export function ensureUniqueEntryPath(path: string, used: Set<string>) {
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

/**
 * 从 OPF 收集「混淆路径 → 可读文件名」的提示表。
 *
 * 混淆只改文件名、不改 manifest 的 id，所以 id 是唯一可靠的线索。用
 * DOMParser 而不是正则：href 里可能有 XML 实体，按 XML 解一次更稳。
 */
export function collectManifestNameHints(opfSource: string, opfPath: string) {
  const hints = new Map<string, string>();
  if (typeof DOMParser === "undefined") return hints;
  const document = new DOMParser().parseFromString(
    opfSource,
    "application/xml",
  );
  if (document.querySelector("parsererror")) return hints;
  const opfDir = parentOf(opfPath);
  for (const element of Array.from(document.querySelectorAll("*"))) {
    if (element.localName !== "item") continue;
    const id = element.getAttribute("id")?.trim() || "";
    const href = element.getAttribute("href")?.trim() || "";
    if (!id || !href) continue;
    const absolute = joinEntryPath(
      opfDir,
      percentDecode(href.split(/[?#]/)[0] || ""),
    );
    const name = nameFromManifestId(id, href);
    hints.set(absolute, name);
    hints.set(absolute.toLowerCase(), name);
  }
  return hints;
}

/**
 * 算出每个条目该改成什么名字（不读内容，纯路径计算）。
 * 目录条目只做字符清理，不改名。
 */
export function planEntryRenames(
  entries: string[],
  hints: Map<string, string>,
) {
  const used = new Set<string>();
  const plan = new Map<string, string>();
  let friendlyIndex = 1;
  for (const raw of entries) {
    const path = raw.replace(/\\/g, "/");
    const isDirectory = path.endsWith("/");
    let target = sanitizeEntryPath(path);
    if (!isDirectory) {
      const parent = parentOf(target);
      const invalid = hasInvalidComponent(path);
      const obfuscated = isObfuscatedEntryName(path);
      if (invalid || obfuscated) {
        const hint = hints.get(path) || hints.get(path.toLowerCase()) || "";
        const name =
          hint ||
          (obfuscated
            ? friendlyEntryName(target, friendlyIndex++)
            : baseName(target));
        target = parent ? `${parent}/${name}` : name;
      }
    }
    const unique = ensureUniqueEntryPath(target, used);
    plan.set(raw, unique);
  }
  return plan;
}

/* ------------------------------------------------------------------ */
/* 引用改写                                                            */
/* ------------------------------------------------------------------ */

export function isExternalReference(value: string) {
  const lower = value.trim().toLowerCase();
  return (
    !lower ||
    lower.startsWith("#") ||
    /^(?:data|https?|mailto|javascript|tel|blob|urn):/.test(lower)
  );
}

export function splitReference(value: string) {
  const index = value.search(/[?#]/);
  return index < 0
    ? ([value, ""] as const)
    : ([value.slice(0, index), value.slice(index)] as const);
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
  const oldAbsolute =
    currentOldPath.toLowerCase() === "meta-inf/container.xml"
      ? normalizeEntryPath(decoded)
      : joinEntryPath(parentOf(currentOldPath), decoded);
  const newAbsolute =
    pathMap.get(oldAbsolute) || lowerPathMap.get(oldAbsolute.toLowerCase());
  if (!newAbsolute || newAbsolute.endsWith("/")) return rawValue;
  let rewritten =
    currentOldPath.toLowerCase() === "meta-inf/container.xml"
      ? newAbsolute
      : relativeEntryPath(currentNewPath, newAbsolute);
  if (main.includes("%")) rewritten = encodeURI(rewritten);
  if (
    main.startsWith("./") &&
    !rewritten.startsWith("./") &&
    !rewritten.startsWith("../")
  )
    rewritten = `./${rewritten}`;
  return `${rewritten}${suffix}`;
}

/**
 * 按改名表改写文本里的相对引用（引号串 + CSS url()）。
 * 只改能对上表的值，其它原样保留 —— 正文里的中文引号内容不会被误伤。
 */
export function rewriteWebEpubTextLinks(
  text: string,
  oldPath: string,
  newPath: string,
  pathMap: Map<string, string>,
) {
  const lowerPathMap = buildCaseInsensitiveMap(pathMap);
  const rewrittenQuoted = text.replace(
    /(["'])(.*?)\1/gis,
    (full, quote: string, value: string) => {
      const rewritten = rewriteReference(
        value,
        oldPath,
        newPath,
        pathMap,
        lowerPathMap,
      );
      return rewritten === value ? full : `${quote}${rewritten}${quote}`;
    },
  );
  return rewrittenQuoted.replace(
    /url\(\s*([^'"\)\s][^)]*?)\s*\)/gis,
    (full, value: string) => {
      const trimmed = value.trim();
      const rewritten = rewriteReference(
        trimmed,
        oldPath,
        newPath,
        pathMap,
        lowerPathMap,
      );
      return rewritten === trimmed ? full : `url(${rewritten})`;
    },
  );
}

/* ------------------------------------------------------------------ */
/* 归档级清洗                                                          */
/* ------------------------------------------------------------------ */

function decodeUtf8(bytes: Uint8Array) {
  try {
    return new TextDecoder("utf-8").decode(bytes);
  } catch {
    let text = "";
    for (const byte of bytes) text += String.fromCharCode(byte);
    return text;
  }
}

/**
 * 把混淆文件名恢复成可读名字，并把所有文本资源里的引用同步改掉。
 *
 * 没有任何条目需要改名时**不重建归档**（返回原 zip），避免每次都整本
 * 解压再压缩；有改动时返回一个新的 JSZip，调用方应替换掉旧对象。
 */
export async function normalizeEpubArchive(
  zip: JSZip,
): Promise<EpubArchiveNormalization> {
  const entries = Object.keys(zip.files);
  const sizes = new Map<string, number>();
  const opfPath = entries.find(
    (path) => !zip.files[path].dir && path.toLowerCase().endsWith(".opf"),
  );

  let hints = new Map<string, string>();
  if (opfPath) {
    try {
      const source = await zip.file(opfPath)?.async("string");
      if (source) hints = collectManifestNameHints(source, opfPath);
    } catch {
      // OPF 读不出来就只做字符级清理，后面 parse 会给出真正的报错
    }
  }

  const plan = planEntryRenames(entries, hints);
  const renamedEntries = entries.filter(
    (path) => plan.get(path) !== path && !path.endsWith("/"),
  ).length;
  if (renamedEntries === 0) {
    return { zip, changed: false, renamedEntries: 0, sizes, pathMap: plan };
  }

  const output = new JSZip();
  for (const raw of entries) {
    const entry = zip.files[raw];
    const newPath = plan.get(raw) || raw;
    if (entry.dir) {
      output.folder(newPath);
      continue;
    }
    const bytes = await entry.async("uint8array");
    sizes.set(newPath, bytes.length);
    // mimetype 必须不压缩且不带目录项，否则部分阅读器判定不是 EPUB
    if (raw.toLowerCase() === "mimetype") {
      output.file(newPath, bytes, {
        compression: "STORE",
        createFolders: false,
      });
      continue;
    }
    if (isTextLikeEntry(raw)) {
      output.file(
        newPath,
        rewriteWebEpubTextLinks(decodeUtf8(bytes), raw, newPath, plan),
      );
    } else {
      output.file(newPath, bytes);
    }
  }
  if (!output.file("mimetype")) {
    output.file("mimetype", "application/epub+zip", {
      compression: "STORE",
      createFolders: false,
    });
  }
  return { zip: output, changed: true, renamedEntries, sizes, pathMap: plan };
}
