import JSZip from "jszip";

export type WebEpubDiagnosticLevel = "error" | "warning";

export type WebEpubDiagnosticIssue = {
  level: WebEpubDiagnosticLevel;
  kind: string;
  path: string | null;
  message: string;
};

export type WebEpubDiagnosticResult = {
  sourceName: string;
  sourceSize: number;
  opfPath: string | null;
  totalEntries: number;
  manifestItems: number;
  errorCount: number;
  warningCount: number;
  issues: WebEpubDiagnosticIssue[];
};

type ManifestItem = {
  id: string;
  absolutePath: string;
};

const RESOURCE_EXTENSIONS = new Set([
  "xhtml", "html", "htm", "css", "svg", "png", "jpg", "jpeg", "webp", "gif", "bmp",
  "ttf", "otf", "woff", "woff2", "ncx", "js", "mp3", "mp4", "m4a", "ogg", "opus", "wav",
]);

const TEXT_EXTENSIONS = new Set(["xhtml", "html", "htm", "xml", "opf", "ncx", "css", "svg"]);

function normalizePath(path: string) {
  const parts: string[] = [];
  for (const segment of path.replace(/\\/g, "/").split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") parts.pop();
    else parts.push(segment);
  }
  return parts.join("/");
}

function parentPath(path: string) {
  const index = path.lastIndexOf("/");
  return index >= 0 ? path.slice(0, index) : "";
}

function joinPath(base: string, relative: string) {
  if (relative.startsWith("/")) return normalizePath(relative.slice(1));
  return normalizePath(`${base}/${relative}`);
}

function extension(path: string) {
  return path.split(".").pop()?.toLowerCase() || "";
}

function percentDecode(value: string) {
  try {
    return decodeURIComponent(value);
  } catch {
    return value.replace(/%([0-9a-f]{2})/gi, (_, hex: string) => String.fromCharCode(Number.parseInt(hex, 16)));
  }
}

function parseXml(source: string) {
  const document = new DOMParser().parseFromString(source, "application/xml");
  if (document.querySelector("parsererror")) throw new Error("XML 格式无效");
  return document;
}

function elementsByLocalName(root: ParentNode, name: string) {
  return Array.from(root.querySelectorAll("*")).filter((node) => node.localName === name) as Element[];
}

function isExternalOrInlineReference(reference: string) {
  const lower = reference.trim().toLowerCase();
  return !lower
    || lower.startsWith("#")
    || lower.startsWith("data:")
    || lower.startsWith("http:")
    || lower.startsWith("https:")
    || lower.startsWith("mailto:")
    || lower.startsWith("javascript:")
    || lower.startsWith("urn:");
}

function resolveReference(currentPath: string, rawReference: string) {
  const reference = rawReference.trim();
  if (isExternalOrInlineReference(reference)) return null;
  const main = reference.split(/[?#]/, 1)[0] || "";
  if (isExternalOrInlineReference(main)) return null;
  const decoded = percentDecode(main.trim());
  if (!decoded) return null;
  // container.xml 的 full-path 以 EPUB 根目录为基准，不以 META-INF 为基准。
  if (currentPath.toLowerCase() === "meta-inf/container.xml") return normalizePath(decoded);
  return joinPath(parentPath(currentPath), decoded);
}

function collectReferences(source: string) {
  const references = new Set<string>();
  const attributePattern = /\b(?:href|src|poster|xlink:href|full-path)\s*=\s*(["'])(.*?)\1/gis;
  const cssPattern = /url\(\s*(["']?)(.*?)\1\s*\)/gis;
  for (const pattern of [attributePattern, cssPattern]) {
    for (const match of source.matchAll(pattern)) {
      const value = (match[2] || "").trim().replace(/^["']|["']$/g, "");
      if (value) references.add(value);
    }
  }
  return [...references].sort();
}

function isResourceEntry(path: string) {
  const lower = path.toLowerCase();
  return !lower.endsWith("/")
    && !lower.startsWith("__macosx/")
    && !lower.startsWith("meta-inf/")
    && RESOURCE_EXTENSIONS.has(extension(lower));
}

function pushIssue(
  issues: WebEpubDiagnosticIssue[],
  level: WebEpubDiagnosticLevel,
  kind: string,
  path: string | null,
  message: string,
) {
  issues.push({ level, kind, path, message });
}

function caseMismatches(lowerToNames: Map<string, string[]>, expected: string) {
  const matches = lowerToNames.get(expected.toLowerCase()) || [];
  return matches.some((name) => name === expected) ? [] : matches;
}

function resultFor(
  file: File,
  opfPath: string | null,
  totalEntries: number,
  manifestItems: number,
  issues: WebEpubDiagnosticIssue[],
): WebEpubDiagnosticResult {
  return {
    sourceName: file.name,
    sourceSize: file.size,
    opfPath,
    totalEntries,
    manifestItems,
    errorCount: issues.filter((issue) => issue.level === "error").length,
    warningCount: issues.filter((issue) => issue.level === "warning").length,
    issues,
  };
}

export async function diagnoseWebEpub(file: File): Promise<WebEpubDiagnosticResult> {
  const issues: WebEpubDiagnosticIssue[] = [];
  let zip: JSZip;
  try {
    zip = await JSZip.loadAsync(file);
  } catch (error) {
    pushIssue(issues, "error", "invalid-zip", null, `无法读取 EPUB 压缩包：${String(error)}`);
    return resultFor(file, null, 0, 0, issues);
  }

  const entryNames = Object.keys(zip.files).map((name) => name.replace(/\\/g, "/"));
  const entrySet = new Set(entryNames);
  const lowerToNames = new Map<string, string[]>();
  for (const name of entryNames) {
    const key = name.toLowerCase();
    lowerToNames.set(key, [...(lowerToNames.get(key) || []), name]);
  }

  if (!entrySet.has("mimetype")) {
    pushIssue(issues, "warning", "missing-mimetype", "mimetype", "EPUB 顶层缺少 mimetype 文件。");
  }

  const containerPath = "META-INF/container.xml";
  if (!entrySet.has(containerPath)) {
    const mismatches = caseMismatches(lowerToNames, containerPath);
    if (mismatches.length) {
      pushIssue(issues, "warning", "case-mismatch", containerPath, `container.xml 的路径大小写不一致：${mismatches.join("、")}`);
    } else {
      pushIssue(issues, "error", "missing-container", containerPath, "EPUB 缺少 META-INF/container.xml。");
    }
  }

  let opfPath = "";
  const exactContainer = zip.file(containerPath);
  if (exactContainer) {
    try {
      const containerDocument = parseXml(await exactContainer.async("text"));
      opfPath = elementsByLocalName(containerDocument, "rootfile")[0]?.getAttribute("full-path")?.replace(/\\/g, "/") || "";
    } catch (error) {
      pushIssue(issues, "warning", "container-parse", containerPath, `container.xml 解析失败：${String(error)}`);
    }
  }
  if (!opfPath) opfPath = entryNames.find((name) => name.toLowerCase().endsWith(".opf")) || "";

  if (!opfPath) {
    pushIssue(issues, "error", "missing-opf", null, "无法定位 OPF 文件。");
    return resultFor(file, null, entryNames.length, 0, issues);
  }

  if (!entrySet.has(opfPath)) {
    const mismatches = caseMismatches(lowerToNames, opfPath);
    if (mismatches.length) {
      pushIssue(issues, "warning", "case-mismatch", opfPath, `OPF 文件的路径大小写不一致：${mismatches.join("、")}`);
    } else {
      pushIssue(issues, "error", "missing-opf", opfPath, "container.xml 指向的 OPF 文件不在压缩包中。");
    }
  }

  const actualOpfPath = entrySet.has(opfPath) ? opfPath : caseMismatches(lowerToNames, opfPath)[0];
  const opfEntry = actualOpfPath ? zip.file(actualOpfPath) : null;
  if (!opfEntry) return resultFor(file, opfPath, entryNames.length, 0, issues);

  let manifest: ManifestItem[] = [];
  try {
    const opfDocument = parseXml(await opfEntry.async("text"));
    const opfDir = parentPath(actualOpfPath || opfPath);
    manifest = elementsByLocalName(opfDocument, "item")
      .map((item) => {
        const id = item.getAttribute("id")?.trim() || "";
        const href = item.getAttribute("href")?.trim() || "";
        return id && href ? { id, absolutePath: joinPath(opfDir, percentDecode(href)) } : null;
      })
      .filter((item): item is ManifestItem => !!item);
  } catch (error) {
    pushIssue(issues, "error", "manifest-parse", opfPath, `OPF manifest 解析失败：${String(error)}`);
    return resultFor(file, opfPath, entryNames.length, 0, issues);
  }

  const manifestSet = new Set(manifest.map((item) => item.absolutePath));
  const requiredEntries = new Set(["mimetype", containerPath, actualOpfPath || opfPath]);
  for (const item of manifest) {
    if (entrySet.has(item.absolutePath)) continue;
    const mismatches = caseMismatches(lowerToNames, item.absolutePath);
    if (mismatches.length) {
      pushIssue(issues, "warning", "case-mismatch", item.absolutePath, `manifest 项“${item.id}”指向的资源大小写不一致：${mismatches.join("、")}`);
    } else {
      pushIssue(issues, "error", "manifest-missing", item.absolutePath, `manifest 项“${item.id}”指向的资源不存在。`);
    }
  }

  for (const name of entryNames) {
    if (!manifestSet.has(name) && !requiredEntries.has(name) && isResourceEntry(name)) {
      pushIssue(issues, "warning", "unregistered-resource", name, "该资源存在于压缩包中，但未登记到 OPF manifest。");
    }
  }

  const seenReferences = new Set<string>();
  for (const name of entryNames) {
    if (zip.files[name]?.dir || !TEXT_EXTENSIONS.has(extension(name))) continue;
    try {
      const text = await zip.files[name].async("text");
      for (const rawReference of collectReferences(text)) {
        const resolved = resolveReference(name, rawReference);
        if (!resolved) continue;
        const key = `${name}\n${rawReference}\n${resolved}`;
        if (seenReferences.has(key)) continue;
        seenReferences.add(key);
        if (entrySet.has(resolved)) continue;
        const mismatches = caseMismatches(lowerToNames, resolved);
        if (mismatches.length) {
          pushIssue(issues, "warning", "ref-case-mismatch", name, `引用“${rawReference}”解析为“${resolved}”，但实际资源大小写为：${mismatches.join("、")}`);
        } else {
          pushIssue(issues, "error", "missing-reference", name, `引用“${rawReference}”指向不存在的资源“${resolved}”。`);
        }
      }
    } catch (error) {
      pushIssue(issues, "warning", "read-entry", name, `无法读取文本资源：${String(error)}`);
    }
  }

  return resultFor(file, opfPath, entryNames.length, manifest.length, issues);
}

export function formatWebEpubDiagnosticReport(results: WebEpubDiagnosticResult[]) {
  const lines = ["TEpub Editor · Web EPUB 诊断报告", `生成时间：${new Date().toLocaleString()}`, ""];
  for (const result of results) {
    lines.push(
      `文件：${result.sourceName}`,
      `OPF：${result.opfPath || "未发现"}`,
      `ZIP 条目：${result.totalEntries}`,
      `manifest 条目：${result.manifestItems}`,
      `结果：${result.errorCount} 错误 / ${result.warningCount} 警告`,
    );
    if (!result.issues.length) lines.push("未发现明显结构问题。");
    else {
      for (const issue of result.issues) {
        lines.push(`- ${issue.level}/${issue.kind}${issue.path ? ` [${issue.path}]` : ""}: ${issue.message}`);
      }
    }
    lines.push("");
  }
  return lines.join("\n");
}
