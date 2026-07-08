import JSZip from "jszip";

export type WebEpubManifestItem = {
  id: string;
  href: string;
  fullPath: string;
  mediaType: string;
  properties: string;
};

export type WebEpubSpineItem = {
  idref: string;
  linear: string;
  manifest?: WebEpubManifestItem;
  title: string;
};

export type WebEpubNavItem = {
  label: string;
  href: string;
  fullPath: string;
  children?: WebEpubNavItem[];
};

export type WebEpubFileEntry = {
  path: string;
  name: string;
  size: number;
  kind: "xhtml" | "css" | "xml" | "image" | "font" | "text" | "other";
  mediaType: string;
  editable: boolean;
};

export type WebEpubMetadata = {
  title: string;
  creator: string;
  language: string;
  identifier: string;
  description: string;
  publisher: string;
  date: string;
  subject: string;
};

export type WebEpubResourceContent = string | Blob | ArrayBuffer | Uint8Array;

export type WebEpubAddResourceOptions = {
  path: string;
  content: WebEpubResourceContent;
  mediaType?: string;
  addToManifest?: boolean;
  addToSpine?: boolean;
  afterPath?: string;
  properties?: string;
};

export type WebEpubDocument = {
  fileName: string;
  zip: JSZip;
  opfPath: string;
  opfDir: string;
  metadata: WebEpubMetadata;
  manifest: WebEpubManifestItem[];
  spine: WebEpubSpineItem[];
  navItems: WebEpubNavItem[];
  files: WebEpubFileEntry[];
};

const TEXT_MEDIA_TYPES = new Set([
  "application/xhtml+xml",
  "text/html",
  "text/css",
  "application/xml",
  "text/xml",
  "application/x-dtbncx+xml",
  "application/smil+xml",
  "text/plain",
]);

const DC_NS = "http://purl.org/dc/elements/1.1/";
const OPF_NS = "http://www.idpf.org/2007/opf";

function parseXml(source: string, label: string) {
  const doc = new DOMParser().parseFromString(source, "application/xml");
  const errorNode = doc.querySelector("parsererror");
  if (errorNode) throw new Error(`${label} 解析失败`);
  return doc;
}

function byLocalName(root: ParentNode, localName: string) {
  return Array.from(root.querySelectorAll("*")).filter((node) => node.localName === localName);
}

function directChildrenByLocalName(root: ParentNode, localName: string) {
  return Array.from(root.children || []).filter((node) => node.localName === localName) as Element[];
}

function firstText(root: ParentNode, localName: string) {
  return byLocalName(root, localName)[0]?.textContent?.trim() || "";
}

const UUID_PATTERN = /\burn:uuid:[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\b|\b[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\b/i;
const MD5_PATTERN = /\b[a-f0-9]{32}\b/i;
const YUEWEI_MD5_CONTEXT = /md5|yuewei|yuewen|阅微/i;

type IdentifierCandidate = {
  value: string;
  context: string;
  priority: number;
};

function getAttributeCaseInsensitive(element: Element, name: string) {
  return element.getAttribute(name) || element.getAttribute(name.toLowerCase()) || element.getAttribute(name.toUpperCase()) || "";
}

function pushIdentifierCandidate(candidates: IdentifierCandidate[], value: string, context = "", priority = 0) {
  const cleanValue = value.trim();
  if (!cleanValue) return;
  candidates.push({
    value: cleanValue,
    context: `${context} ${cleanValue}`.trim(),
    priority,
  });
}

function collectIdentifierCandidates(opfDoc: XMLDocument) {
  const candidates: IdentifierCandidate[] = [];
  const packageElement = byLocalName(opfDoc, "package")[0] as Element | undefined;
  const uniqueIdentifierId = packageElement?.getAttribute("unique-identifier") || "";

  for (const node of byLocalName(opfDoc, "identifier")) {
    const element = node as Element;
    const id = element.getAttribute("id") || element.getAttribute("xml:id") || "";
    const scheme = element.getAttribute("opf:scheme") || element.getAttribute("scheme") || "";
    const text = element.textContent?.trim() || "";
    pushIdentifierCandidate(candidates, text, `${id} ${scheme}`, id && id === uniqueIdentifierId ? 100 : 80);
  }

  for (const node of byLocalName(opfDoc, "meta")) {
    const element = node as Element;
    const name = getAttributeCaseInsensitive(element, "name");
    const property = getAttributeCaseInsensitive(element, "property");
    const content = getAttributeCaseInsensitive(element, "content");
    const refines = getAttributeCaseInsensitive(element, "refines");
    const text = element.textContent?.trim() || "";
    const context = `${name} ${property} ${refines}`;
    pushIdentifierCandidate(candidates, content, context, YUEWEI_MD5_CONTEXT.test(`${context} ${content}`) ? 90 : 40);
    pushIdentifierCandidate(candidates, text, context, YUEWEI_MD5_CONTEXT.test(`${context} ${text}`) ? 90 : 30);
  }

  return candidates.sort((a, b) => b.priority - a.priority);
}

function preferredIdentifier(opfDoc: XMLDocument) {
  const candidates = collectIdentifierCandidates(opfDoc);
  const uuidCandidate = candidates.find((candidate) => UUID_PATTERN.test(candidate.value));
  const uuid = uuidCandidate?.value.match(UUID_PATTERN)?.[0];
  if (uuid) return uuid;

  const contextualMd5Candidate = candidates.find((candidate) => YUEWEI_MD5_CONTEXT.test(candidate.context) && MD5_PATTERN.test(candidate.value));
  const contextualMd5 = contextualMd5Candidate?.value.match(MD5_PATTERN)?.[0];
  if (contextualMd5) return contextualMd5;

  const md5Candidate = candidates.find((candidate) => MD5_PATTERN.test(candidate.value));
  const md5 = md5Candidate?.value.match(MD5_PATTERN)?.[0];
  if (md5) return md5;

  return candidates.find((candidate) => candidate.value)?.value || "";
}

function dirname(path: string) {
  const index = path.lastIndexOf("/");
  return index >= 0 ? path.slice(0, index + 1) : "";
}

export function normalizeZipPath(path: string) {
  const parts: string[] = [];
  for (const part of path.replace(/\\/g, "/").split("/")) {
    if (!part || part === ".") continue;
    if (part === "..") parts.pop();
    else parts.push(part);
  }
  return parts.join("/");
}

function resolveZipPath(baseDir: string, href: string) {
  const cleanHref = decodeURIComponent(href.split("#")[0] || "").replace(/^\//, "");
  return normalizeZipPath(`${baseDir}${cleanHref}`);
}

function fileNameOf(path: string) {
  return path.split("/").pop() || path;
}

function extension(path: string) {
  return path.split(".").pop()?.toLowerCase() || "";
}

export function guessWebEpubMediaType(path: string) {
  const ext = extension(path);
  if (ext === "xhtml" || ext === "html" || ext === "htm") return "application/xhtml+xml";
  if (ext === "css") return "text/css";
  if (ext === "opf" || ext === "xml") return "application/xml";
  if (ext === "ncx") return "application/x-dtbncx+xml";
  if (ext === "svg") return "image/svg+xml";
  if (ext === "png") return "image/png";
  if (ext === "jpg" || ext === "jpeg") return "image/jpeg";
  if (ext === "webp") return "image/webp";
  if (ext === "gif") return "image/gif";
  if (ext === "woff") return "font/woff";
  if (ext === "woff2") return "font/woff2";
  if (ext === "ttf") return "font/ttf";
  if (ext === "otf") return "font/otf";
  if (ext === "txt") return "text/plain";
  return "application/octet-stream";
}

function kindFor(path: string, mediaType = ""): WebEpubFileEntry["kind"] {
  const ext = extension(path);
  if (mediaType === "application/xhtml+xml" || ext === "xhtml" || ext === "html" || ext === "htm") return "xhtml";
  if (mediaType === "text/css" || ext === "css") return "css";
  if (mediaType.includes("xml") || ext === "xml" || ext === "opf" || ext === "ncx") return "xml";
  if (mediaType.startsWith("image/") || ["jpg", "jpeg", "png", "gif", "webp", "svg"].includes(ext)) return "image";
  if (mediaType.includes("font") || ["ttf", "otf", "woff", "woff2"].includes(ext)) return "font";
  if (mediaType.startsWith("text/") || ["txt", "md"].includes(ext)) return "text";
  return "other";
}

function isEditable(path: string, mediaType = "") {
  return TEXT_MEDIA_TYPES.has(mediaType) || ["xhtml", "html", "htm", "css", "xml", "opf", "ncx", "txt", "md", "svg"].includes(extension(path));
}

function parseManifest(opfDoc: XMLDocument, opfDir: string) {
  return byLocalName(opfDoc, "item").map((node) => {
    const element = node as Element;
    const href = element.getAttribute("href") || "";
    return {
      id: element.getAttribute("id") || "",
      href,
      fullPath: resolveZipPath(opfDir, href),
      mediaType: element.getAttribute("media-type") || "",
      properties: element.getAttribute("properties") || "",
    };
  });
}

function parseSpine(opfDoc: XMLDocument, manifest: WebEpubManifestItem[]) {
  const byId = new Map(manifest.map((item) => [item.id, item]));
  return byLocalName(opfDoc, "itemref").map((node, index) => {
    const element = node as Element;
    const idref = element.getAttribute("idref") || "";
    const manifestItem = byId.get(idref);
    return {
      idref,
      linear: element.getAttribute("linear") || "yes",
      manifest: manifestItem,
      title: manifestItem ? fileNameOf(manifestItem.fullPath) : `spine-${index + 1}`,
    };
  });
}

function parseNavDocument(source: string, navPath: string) {
  const navDoc = parseXml(source, "NAV");
  const navDir = dirname(navPath);
  const navs = byLocalName(navDoc, "nav") as Element[];
  const tocNav = navs.find((node) => {
    const type = `${node.getAttribute("epub:type") || ""} ${node.getAttribute("type") || ""}`;
    return /\btoc\b/i.test(type);
  }) || navs[0];
  const rootOl = directChildrenByLocalName(tocNav || navDoc.documentElement, "ol")[0]
    || byLocalName(tocNav || navDoc.documentElement, "ol")[0] as Element | undefined;

  function parseLi(li: Element): WebEpubNavItem | null {
    const link = directChildrenByLocalName(li, "a")[0];
    const span = directChildrenByLocalName(li, "span")[0];
    const labelNode = link || span;
    const href = link?.getAttribute("href") || "";
    const label = labelNode?.textContent?.replace(/\s+/g, " ").trim() || href;
    const childOl = directChildrenByLocalName(li, "ol")[0];
    const children = childOl ? directChildrenByLocalName(childOl, "li").map(parseLi).filter((item): item is WebEpubNavItem => !!item) : [];
    if (!label && children.length === 0) return null;
    return {
      label,
      href,
      fullPath: href ? resolveZipPath(navDir, href) : "",
      children: children.length ? children : undefined,
    };
  }

  if (!rootOl) return [];
  return directChildrenByLocalName(rootOl, "li").map(parseLi).filter((item): item is WebEpubNavItem => !!item);
}

function parseNcxDocument(source: string, ncxPath: string) {
  const ncxDoc = parseXml(source, "NCX");
  const ncxDir = dirname(ncxPath);
  function parseNavPoint(node: Element): WebEpubNavItem | null {
    const navLabel = directChildrenByLocalName(node, "navLabel")[0];
    const label = navLabel ? firstText(navLabel, "text") : firstText(node, "text");
    const content = directChildrenByLocalName(node, "content")[0];
    const href = content?.getAttribute("src") || "";
    const children = directChildrenByLocalName(node, "navPoint").map(parseNavPoint).filter((item): item is WebEpubNavItem => !!item);
    if (!href && !label && children.length === 0) return null;
    return {
      label: label || href,
      href,
      fullPath: href ? resolveZipPath(ncxDir, href) : "",
      children: children.length ? children : undefined,
    };
  }

  const navMap = byLocalName(ncxDoc, "navMap")[0] as Element | undefined;
  const roots = navMap ? directChildrenByLocalName(navMap, "navPoint") : directChildrenByLocalName(ncxDoc.documentElement, "navPoint");
  return roots.map(parseNavPoint).filter((item): item is WebEpubNavItem => !!item);
}

async function parseNavigation(zip: JSZip, manifest: WebEpubManifestItem[]) {
  const navItem = manifest.find((item) => item.properties.split(/\s+/).includes("nav"));
  if (navItem) {
    const source = await zip.file(navItem.fullPath)?.async("string");
    if (source) return parseNavDocument(source, navItem.fullPath);
  }

  const ncxItem = manifest.find((item) => item.mediaType === "application/x-dtbncx+xml");
  if (ncxItem) {
    const source = await zip.file(ncxItem.fullPath)?.async("string");
    if (source) return parseNcxDocument(source, ncxItem.fullPath);
  }

  return [];
}

function parseMetadata(opfDoc: XMLDocument): WebEpubMetadata {
  return {
    title: firstText(opfDoc, "title") || "未命名 EPUB",
    creator: firstText(opfDoc, "creator"),
    language: firstText(opfDoc, "language"),
    identifier: preferredIdentifier(opfDoc),
    description: firstText(opfDoc, "description"),
    publisher: firstText(opfDoc, "publisher"),
    date: firstText(opfDoc, "date"),
    subject: firstText(opfDoc, "subject"),
  };
}

function collectFiles(zip: JSZip, manifest: WebEpubManifestItem[]) {
  const mediaByPath = new Map(manifest.map((item) => [item.fullPath, item.mediaType]));
  return Object.values(zip.files)
    .filter((entry) => !entry.dir)
    .map((entry) => {
      const mediaType = mediaByPath.get(entry.name) || "";
      const kind = kindFor(entry.name, mediaType);
      return {
        path: entry.name,
        name: fileNameOf(entry.name),
        size: Number((entry as any)._data?.uncompressedSize || 0),
        kind,
        mediaType,
        editable: isEditable(entry.name, mediaType),
      };
    })
    .sort((a, b) => a.path.localeCompare(b.path));
}

async function refreshPackageState(doc: WebEpubDocument) {
  const opfSource = await readWebEpubText(doc, doc.opfPath);
  const opfDoc = parseXml(opfSource, "OPF");
  doc.metadata = parseMetadata(opfDoc);
  doc.manifest = parseManifest(opfDoc, doc.opfDir);
  doc.spine = parseSpine(opfDoc, doc.manifest);
  doc.navItems = await parseNavigation(doc.zip, doc.manifest);
  doc.files = collectFiles(doc.zip, doc.manifest);
}

function relativeHref(fromDir: string, targetPath: string) {
  const fromParts = normalizeZipPath(fromDir).split("/").filter(Boolean);
  const targetParts = normalizeZipPath(targetPath).split("/").filter(Boolean);
  while (fromParts.length && targetParts.length && fromParts[0] === targetParts[0]) {
    fromParts.shift();
    targetParts.shift();
  }
  return [...fromParts.map(() => ".."), ...targetParts].join("/");
}

function manifestIdBase(path: string) {
  const base = fileNameOf(path).replace(/\.[^.]+$/, "") || "item";
  const normalized = base.replace(/[^A-Za-z0-9_-]+/g, "_").replace(/^_+|_+$/g, "");
  return /^[A-Za-z_]/.test(normalized) ? normalized : `item_${normalized || "resource"}`;
}

function uniqueManifestId(opfDoc: XMLDocument, path: string) {
  const used = new Set(
    byLocalName(opfDoc, "item")
      .map((node) => (node as Element).getAttribute("id") || "")
      .filter(Boolean),
  );
  const base = manifestIdBase(path);
  let id = base;
  let index = 2;
  while (used.has(id)) {
    id = `${base}_${index}`;
    index += 1;
  }
  return id;
}

function opfManifestElement(opfDoc: XMLDocument) {
  const manifest = byLocalName(opfDoc, "manifest")[0] as Element | undefined;
  if (!manifest) throw new Error("OPF 缺少 manifest 节点");
  return manifest;
}

function findManifestElementByPath(opfDoc: XMLDocument, opfDir: string, path: string) {
  return byLocalName(opfDoc, "item")
    .map((node) => node as Element)
    .find((item) => resolveZipPath(opfDir, item.getAttribute("href") || "") === path);
}

function contentSize(content: WebEpubResourceContent) {
  if (typeof content === "string") return new Blob([content]).size;
  if (content instanceof Blob) return content.size;
  return content.byteLength;
}

export async function loadWebEpub(file: File): Promise<WebEpubDocument> {
  const zip = await JSZip.loadAsync(await file.arrayBuffer());
  const containerSource = await zip.file("META-INF/container.xml")?.async("string");
  if (!containerSource) throw new Error("缺少 META-INF/container.xml");

  const containerDoc = parseXml(containerSource, "container.xml");
  const rootfile = byLocalName(containerDoc, "rootfile")[0] as Element | undefined;
  const opfPath = rootfile?.getAttribute("full-path") || "";
  if (!opfPath) throw new Error("container.xml 未声明 OPF 路径");

  const opfSource = await zip.file(opfPath)?.async("string");
  if (!opfSource) throw new Error(`找不到 OPF 文件：${opfPath}`);

  const opfDoc = parseXml(opfSource, "OPF");
  const opfDir = dirname(opfPath);
  const manifest = parseManifest(opfDoc, opfDir);
  const spine = parseSpine(opfDoc, manifest);
  const navItems = await parseNavigation(zip, manifest);

  return {
    fileName: file.name,
    zip,
    opfPath,
    opfDir,
    metadata: parseMetadata(opfDoc),
    manifest,
    spine,
    navItems,
    files: collectFiles(zip, manifest),
  };
}

export async function readWebEpubText(doc: WebEpubDocument, path: string) {
  const file = doc.zip.file(path);
  if (!file) throw new Error(`文件不存在：${path}`);
  return file.async("string");
}

export async function readWebEpubBlob(doc: WebEpubDocument, path: string, fallbackType = "application/octet-stream") {
  const file = doc.zip.file(path);
  if (!file) throw new Error(`文件不存在：${path}`);
  const blob = await file.async("blob");
  return blob.type ? blob : new Blob([blob], { type: fallbackType });
}

export function updateWebEpubText(doc: WebEpubDocument, path: string, content: string) {
  doc.zip.file(path, content);
  const file = doc.files.find((entry) => entry.path === path);
  if (file) file.size = new Blob([content]).size;
}

function metadataElement(opfDoc: XMLDocument) {
  const element = byLocalName(opfDoc, "metadata")[0] as Element | undefined;
  if (!element) throw new Error("OPF 缺少 metadata 节点");
  return element;
}

function setFirstText(opfDoc: XMLDocument, parent: Element, localName: string, value: string) {
  let element = byLocalName(parent, localName)[0] as Element | undefined;
  if (!element) {
    element = opfDoc.createElementNS(DC_NS, `dc:${localName}`);
    parent.appendChild(element);
  }
  element.textContent = value.trim();
  return element;
}

function ensurePackageIdentifier(opfDoc: XMLDocument, identifierElement: Element) {
  const packageElement = byLocalName(opfDoc, "package")[0] as Element | undefined;
  if (!packageElement) return;
  if (!identifierElement.getAttribute("id")) identifierElement.setAttribute("id", "BookId");
  if (!packageElement.getAttribute("unique-identifier")) {
    packageElement.setAttribute("unique-identifier", identifierElement.getAttribute("id") || "BookId");
  }
}

function serializeXml(doc: XMLDocument, originalSource: string) {
  const serialized = new XMLSerializer().serializeToString(doc);
  const declaration = originalSource.match(/^\s*<\?xml[^>]*\?>/i)?.[0];
  return declaration && !serialized.trimStart().startsWith("<?xml")
    ? `${declaration}\n${serialized}`
    : serialized;
}

export async function updateWebEpubMetadata(doc: WebEpubDocument, metadata: WebEpubMetadata) {
  const opfSource = await readWebEpubText(doc, doc.opfPath);
  const opfDoc = parseXml(opfSource, "OPF");
  const metaNode = metadataElement(opfDoc);
  setFirstText(opfDoc, metaNode, "title", metadata.title || "未命名 EPUB");
  setFirstText(opfDoc, metaNode, "creator", metadata.creator);
  setFirstText(opfDoc, metaNode, "language", metadata.language || "zh-CN");
  const identifier = setFirstText(opfDoc, metaNode, "identifier", metadata.identifier);
  ensurePackageIdentifier(opfDoc, identifier);
  setFirstText(opfDoc, metaNode, "description", metadata.description);
  setFirstText(opfDoc, metaNode, "publisher", metadata.publisher);
  setFirstText(opfDoc, metaNode, "date", metadata.date);
  setFirstText(opfDoc, metaNode, "subject", metadata.subject);

  const updatedSource = serializeXml(opfDoc, opfSource);
  updateWebEpubText(doc, doc.opfPath, updatedSource);
  doc.metadata = parseMetadata(opfDoc);
  return doc.metadata;
}

async function updateManifestMediaType(doc: WebEpubDocument, path: string, mediaType: string) {
  const manifestItem = doc.manifest.find((item) => item.fullPath === path);
  if (!manifestItem) return;

  const opfSource = await readWebEpubText(doc, doc.opfPath);
  const opfDoc = parseXml(opfSource, "OPF");
  const opfItem = findManifestElementByPath(opfDoc, doc.opfDir, path);
  if (!opfItem) return;

  opfItem.setAttribute("media-type", mediaType);
  updateWebEpubText(doc, doc.opfPath, serializeXml(opfDoc, opfSource));
  manifestItem.mediaType = mediaType;
}

export async function updateWebEpubBinary(doc: WebEpubDocument, path: string, content: Blob | ArrayBuffer | Uint8Array, mediaType = "") {
  doc.zip.file(path, content);
  const file = doc.files.find((entry) => entry.path === path);
  const size = content instanceof Blob ? content.size : content.byteLength;
  if (file) {
    file.size = size;
    if (mediaType) {
      file.mediaType = mediaType;
      file.kind = kindFor(path, mediaType);
    }
  }
  if (mediaType) await updateManifestMediaType(doc, path, mediaType);
}

export async function addWebEpubResource(doc: WebEpubDocument, options: WebEpubAddResourceOptions) {
  const path = normalizeZipPath(options.path);
  if (!path || path.endsWith("/")) throw new Error("资源路径无效");
  if (doc.zip.file(path)) throw new Error(`资源已存在：${path}`);

  const mediaType = options.mediaType || guessWebEpubMediaType(path);
  doc.zip.file(path, options.content);

  const shouldAddToManifest = options.addToManifest ?? true;
  if (shouldAddToManifest) {
    const opfSource = await readWebEpubText(doc, doc.opfPath);
    const opfDoc = parseXml(opfSource, "OPF");
    const manifest = opfManifestElement(opfDoc);
    const item = opfDoc.createElementNS(OPF_NS, "item");
    const id = uniqueManifestId(opfDoc, path);
    item.setAttribute("id", id);
    item.setAttribute("href", relativeHref(doc.opfDir, path));
    item.setAttribute("media-type", mediaType);
    if (options.properties) item.setAttribute("properties", options.properties);
    manifest.appendChild(item);

    if (options.addToSpine) {
      const spine = byLocalName(opfDoc, "spine")[0] as Element | undefined;
      if (spine) {
        const itemRef = opfDoc.createElementNS(OPF_NS, "itemref");
        itemRef.setAttribute("idref", id);
        const afterItem = options.afterPath ? findManifestElementByPath(opfDoc, doc.opfDir, options.afterPath) : undefined;
        const afterId = afterItem?.getAttribute("id") || "";
        const afterRef = afterId
          ? byLocalName(spine, "itemref").find((node) => (node as Element).getAttribute("idref") === afterId)
          : undefined;
        if (afterRef?.nextSibling) spine.insertBefore(itemRef, afterRef.nextSibling);
        else spine.appendChild(itemRef);
      }
    }

    updateWebEpubText(doc, doc.opfPath, serializeXml(opfDoc, opfSource));
  } else {
    const kind = kindFor(path, mediaType);
    doc.files = [
      ...doc.files,
      {
        path,
        name: fileNameOf(path),
        size: contentSize(options.content),
        kind,
        mediaType,
        editable: isEditable(path, mediaType),
      },
    ].sort((a, b) => a.path.localeCompare(b.path));
  }

  await refreshPackageState(doc);
  return doc.files.find((entry) => entry.path === path);
}

export async function deleteWebEpubResource(doc: WebEpubDocument, path: string) {
  const cleanPath = normalizeZipPath(path);
  if (!cleanPath) throw new Error("资源路径无效");
  if (cleanPath === doc.opfPath || cleanPath === "mimetype" || cleanPath === "META-INF/container.xml") {
    throw new Error("不能删除 EPUB 核心文件");
  }

  const opfSource = await readWebEpubText(doc, doc.opfPath);
  const opfDoc = parseXml(opfSource, "OPF");
  const item = findManifestElementByPath(opfDoc, doc.opfDir, cleanPath);
  const id = item?.getAttribute("id") || "";
  item?.parentNode?.removeChild(item);

  if (id) {
    for (const itemRef of byLocalName(opfDoc, "itemref")) {
      const element = itemRef as Element;
      if (element.getAttribute("idref") === id) element.parentNode?.removeChild(element);
    }
  }

  doc.zip.remove(cleanPath);
  updateWebEpubText(doc, doc.opfPath, serializeXml(opfDoc, opfSource));
  await refreshPackageState(doc);
}

export async function renameWebEpubResource(doc: WebEpubDocument, path: string, newPath: string) {
  const cleanPath = normalizeZipPath(path);
  const cleanNewPath = normalizeZipPath(newPath);
  if (!cleanPath || !cleanNewPath || cleanNewPath.endsWith("/")) throw new Error("资源路径无效");
  if (cleanPath === cleanNewPath) return doc.files.find((entry) => entry.path === cleanPath);
  if (cleanPath === doc.opfPath || cleanPath === "mimetype" || cleanPath === "META-INF/container.xml") {
    throw new Error("不能重命名 EPUB 核心文件");
  }
  if (doc.zip.file(cleanNewPath)) throw new Error(`资源已存在：${cleanNewPath}`);

  const zipFile = doc.zip.file(cleanPath);
  if (!zipFile) throw new Error(`文件不存在：${cleanPath}`);
  const content = await zipFile.async("arraybuffer");
  doc.zip.file(cleanNewPath, content);
  doc.zip.remove(cleanPath);

  const opfSource = await readWebEpubText(doc, doc.opfPath);
  const opfDoc = parseXml(opfSource, "OPF");
  const item = findManifestElementByPath(opfDoc, doc.opfDir, cleanPath);
  if (item) {
    item.setAttribute("href", relativeHref(doc.opfDir, cleanNewPath));
    updateWebEpubText(doc, doc.opfPath, serializeXml(opfDoc, opfSource));
  }

  await refreshPackageState(doc);
  return doc.files.find((entry) => entry.path === cleanNewPath);
}

export async function exportWebEpubBlob(doc: WebEpubDocument) {
  doc.zip.file("mimetype", "application/epub+zip", {
    compression: "STORE",
    createFolders: false,
  });
  return doc.zip.generateAsync({
    type: "blob",
    mimeType: "application/epub+zip",
    compression: "DEFLATE",
    compressionOptions: { level: 6 },
  });
}
