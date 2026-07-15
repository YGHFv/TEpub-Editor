import {
  addWebEpubResource,
  exportWebEpubBlob,
  loadWebEpub,
  readWebEpubBlob,
  readWebEpubText,
  updateWebEpubBinary,
  updateWebEpubMetadata,
  updateWebEpubText,
  type WebEpubDocument,
} from "$lib/webEpub";

export type WebLibraryBookKind = "epub" | "txt";

export type WebLibraryBook = {
  id: string;
  kind: WebLibraryBookKind;
  fileName: string;
  title: string;
  author: string;
  subtitle: string;
  description: string;
  publisher: string;
  language: string;
  identifier: string;
  tags: string[];
  series: string;
  maker: string;
  fileSize: number;
  addedAt: number;
  modifiedAt: number;
  blob: Blob;
  coverBlob: Blob | null;
};

export type WebLibraryBookPatch = Partial<Pick<WebLibraryBook,
  "title" | "author" | "subtitle" | "description" | "publisher" | "language" | "identifier" | "tags" | "series" | "maker"
>>;

export type WebTextHistorySnapshot = {
  id: string;
  sourceKey: string;
  fileName: string;
  content: string;
  size: number;
  timestamp: number;
};

const DB_NAME = "tepub-web-library";
const DB_VERSION = 2;
const STORE_NAME = "books";
const HISTORY_STORE_NAME = "text-history";

function requestResult<T>(request: IDBRequest<T>) {
  return new Promise<T>((resolve, reject) => {
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error || new Error("IndexedDB 操作失败"));
  });
}

function transactionDone(transaction: IDBTransaction) {
  return new Promise<void>((resolve, reject) => {
    transaction.oncomplete = () => resolve();
    transaction.onerror = () => reject(transaction.error || new Error("IndexedDB 事务失败"));
    transaction.onabort = () => reject(transaction.error || new Error("IndexedDB 事务已中止"));
  });
}

async function openDatabase() {
  if (typeof indexedDB === "undefined") throw new Error("当前浏览器不支持 IndexedDB，无法使用 Web 书库");
  const request = indexedDB.open(DB_NAME, DB_VERSION);
  request.onupgradeneeded = () => {
    const database = request.result;
    if (!database.objectStoreNames.contains(STORE_NAME)) {
      const store = database.createObjectStore(STORE_NAME, { keyPath: "id" });
      store.createIndex("addedAt", "addedAt");
      store.createIndex("modifiedAt", "modifiedAt");
      store.createIndex("fileName", "fileName");
    }
    if (!database.objectStoreNames.contains(HISTORY_STORE_NAME)) {
      const store = database.createObjectStore(HISTORY_STORE_NAME, { keyPath: "id" });
      store.createIndex("sourceKey", "sourceKey");
      store.createIndex("timestamp", "timestamp");
    }
  };
  return requestResult(request);
}

function makeId() {
  return typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `book-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`;
}

function stem(fileName: string) {
  return fileName.replace(/\.(?:epub|txt)$/i, "") || "未命名图书";
}

function splitTags(subject: string) {
  return [...new Set(subject.split(/[,，;；|]/).map((tag) => tag.trim()).filter(Boolean))];
}

function joinTags(tags: string[]) {
  return [...new Set(tags.map((tag) => tag.trim()).filter(Boolean))].join(", ");
}

function coverPath(doc: WebEpubDocument) {
  const propertyCover = doc.manifest.find((item) => item.properties.split(/\s+/).includes("cover-image"));
  if (propertyCover) return propertyCover.fullPath;
  return doc.manifest.find((item) => /(?:^|[-_])cover(?:[-_.]|$)/i.test(item.id) && item.mediaType.startsWith("image/"))?.fullPath
    || doc.manifest.find((item) => /(?:^|\/)cover[^/]*\.(?:jpe?g|png|webp|gif|svg)$/i.test(item.fullPath))?.fullPath
    || "";
}

async function extractCover(doc: WebEpubDocument) {
  const path = coverPath(doc);
  if (!path) return null;
  try { return await readWebEpubBlob(doc, path, doc.manifest.find((item) => item.fullPath === path)?.mediaType); }
  catch { return null; }
}

function coverExtension(cover: File) {
  if (cover.type === "image/png") return "png";
  if (cover.type === "image/webp") return "webp";
  if (cover.type === "image/gif") return "gif";
  return "jpg";
}

async function addCoverToEpub(doc: WebEpubDocument, cover: File) {
  const directory = doc.opfDir ? `${doc.opfDir}/Images` : "Images";
  const extension = coverExtension(cover);
  let path = `${directory}/cover.${extension}`;
  let suffix = 2;
  while (doc.zip.file(path)) path = `${directory}/cover-${suffix++}.${extension}`;
  await addWebEpubResource(doc, { path, content: cover, mediaType: cover.type || "image/jpeg", properties: "cover-image" });

  const manifestItem = doc.manifest.find((item) => item.fullPath === path);
  if (!manifestItem) return;
  const source = await readWebEpubText(doc, doc.opfPath);
  const xml = new DOMParser().parseFromString(source, "application/xml");
  const metadata = Array.from(xml.querySelectorAll("*")).find((node) => node.localName === "metadata");
  if (!metadata) return;
  let coverMeta = Array.from(metadata.querySelectorAll("*")).find((node) => node.localName === "meta" && node.getAttribute("name")?.toLowerCase() === "cover");
  if (!coverMeta) {
    coverMeta = xml.createElementNS("http://www.idpf.org/2007/opf", "meta");
    coverMeta.setAttribute("name", "cover");
    metadata.appendChild(coverMeta);
  }
  coverMeta.setAttribute("content", manifestItem.id);
  updateWebEpubText(doc, doc.opfPath, new XMLSerializer().serializeToString(xml));
}

async function epubFields(file: File) {
  const doc = await loadWebEpub(file);
  return {
    title: doc.metadata.title || stem(file.name),
    author: doc.metadata.creator,
    description: doc.metadata.description,
    publisher: doc.metadata.publisher,
    language: doc.metadata.language || "zh-CN",
    identifier: doc.metadata.identifier,
    tags: splitTags(doc.metadata.subject),
    coverBlob: await extractCover(doc),
  };
}

export async function listWebLibraryBooks() {
  const database = await openDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, "readonly");
    const books = await requestResult(transaction.objectStore(STORE_NAME).getAll()) as WebLibraryBook[];
    return books;
  } finally {
    database.close();
  }
}

export async function getWebLibraryBook(id: string) {
  const database = await openDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, "readonly");
    return await requestResult(transaction.objectStore(STORE_NAME).get(id)) as WebLibraryBook | undefined;
  } finally {
    database.close();
  }
}

export async function addWebLibraryBook(file: File) {
  const kind: WebLibraryBookKind = file.name.toLowerCase().endsWith(".txt") ? "txt" : "epub";
  let parsed = {
    title: stem(file.name), author: "", description: "", publisher: "", language: kind === "epub" ? "zh-CN" : "",
    identifier: "", tags: [] as string[], coverBlob: null as Blob | null,
  };
  if (kind === "epub") parsed = await epubFields(file);
  const now = Date.now();
  const book: WebLibraryBook = {
    id: makeId(),
    kind,
    fileName: file.name,
    title: parsed.title,
    author: parsed.author,
    subtitle: "",
    description: parsed.description,
    publisher: parsed.publisher,
    language: parsed.language,
    identifier: parsed.identifier,
    tags: parsed.tags,
    series: "",
    maker: "",
    fileSize: file.size,
    addedAt: now,
    modifiedAt: file.lastModified || now,
    blob: file,
    coverBlob: parsed.coverBlob,
  };
  const database = await openDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, "readwrite");
    transaction.objectStore(STORE_NAME).add(book);
    await transactionDone(transaction);
    return book;
  } finally {
    database.close();
  }
}

export async function removeWebLibraryBook(id: string) {
  const database = await openDatabase();
  try {
    const transaction = database.transaction([STORE_NAME, HISTORY_STORE_NAME], "readwrite");
    transaction.objectStore(STORE_NAME).delete(id);
    const historyIndex = transaction.objectStore(HISTORY_STORE_NAME).index("sourceKey");
    const cursorRequest = historyIndex.openKeyCursor(IDBKeyRange.only(`library:${id}`));
    cursorRequest.onsuccess = () => {
      const cursor = cursorRequest.result;
      if (!cursor) return;
      transaction.objectStore(HISTORY_STORE_NAME).delete(cursor.primaryKey);
      cursor.continue();
    };
    await transactionDone(transaction);
  } finally {
    database.close();
  }
}

export async function putWebLibraryBook(book: WebLibraryBook) {
  const database = await openDatabase();
  try {
    const transaction = database.transaction(STORE_NAME, "readwrite");
    transaction.objectStore(STORE_NAME).put(book);
    await transactionDone(transaction);
    return book;
  } finally {
    database.close();
  }
}

export async function updateWebLibraryBookMetadata(id: string, patch: WebLibraryBookPatch) {
  const book = await getWebLibraryBook(id);
  if (!book) throw new Error("书库中找不到该图书");
  const next: WebLibraryBook = {
    ...book,
    ...patch,
    title: patch.title?.trim() || book.title,
    tags: patch.tags ? [...new Set(patch.tags.map((tag) => tag.trim()).filter(Boolean))] : book.tags,
    modifiedAt: Date.now(),
  };
  if (book.kind === "epub") {
    const file = new File([book.blob], book.fileName, { type: "application/epub+zip" });
    const doc = await loadWebEpub(file);
    await updateWebEpubMetadata(doc, {
      ...doc.metadata,
      title: next.title,
      creator: next.author,
      language: next.language || "zh-CN",
      identifier: next.identifier,
      description: next.description,
      publisher: next.publisher,
      subject: joinTags(next.tags),
    });
    next.blob = await exportWebEpubBlob(doc);
    next.fileSize = next.blob.size;
  }
  return putWebLibraryBook(next);
}

export async function replaceWebLibraryBookCover(id: string, cover: File) {
  const book = await getWebLibraryBook(id);
  if (!book) throw new Error("书库中找不到该图书");
  const next = { ...book, coverBlob: cover as Blob, modifiedAt: Date.now() };
  if (book.kind === "epub") {
    const file = new File([book.blob], book.fileName, { type: "application/epub+zip" });
    const doc = await loadWebEpub(file);
    const path = coverPath(doc);
    if (path) {
      await updateWebEpubBinary(doc, path, cover, cover.type || "image/jpeg");
    } else await addCoverToEpub(doc, cover);
    next.blob = await exportWebEpubBlob(doc);
    next.fileSize = next.blob.size;
  }
  return putWebLibraryBook(next);
}

export async function replaceWebLibraryBookBlob(id: string, blob: Blob, fileName?: string) {
  const book = await getWebLibraryBook(id);
  if (!book) throw new Error("书库中找不到该图书");
  const next: WebLibraryBook = { ...book, blob, fileName: fileName || book.fileName, fileSize: blob.size, modifiedAt: Date.now() };
  if (book.kind === "epub") {
    const parsed = await epubFields(new File([blob], next.fileName, { type: "application/epub+zip" }));
    next.title = parsed.title || next.title;
    next.author = parsed.author;
    next.description = parsed.description;
    next.publisher = parsed.publisher;
    next.language = parsed.language;
    next.identifier = parsed.identifier;
    next.tags = parsed.tags.length ? parsed.tags : next.tags;
    next.coverBlob = parsed.coverBlob || next.coverBlob;
  }
  return putWebLibraryBook(next);
}

export async function requestWebLibraryPersistence() {
  if (typeof navigator === "undefined" || !navigator.storage?.persist) return false;
  try { return await navigator.storage.persist(); } catch { return false; }
}

export async function listWebTextHistory(sourceKey: string) {
  const database = await openDatabase();
  try {
    const transaction = database.transaction(HISTORY_STORE_NAME, "readonly");
    const snapshots = await requestResult(
      transaction.objectStore(HISTORY_STORE_NAME).index("sourceKey").getAll(sourceKey),
    ) as WebTextHistorySnapshot[];
    return snapshots.sort((a, b) => b.timestamp - a.timestamp);
  } finally {
    database.close();
  }
}

export async function saveWebTextHistorySnapshot(sourceKey: string, fileName: string, content: string) {
  const existing = await listWebTextHistory(sourceKey);
  if (existing[0]?.content === content) return existing[0];
  const timestamp = Date.now();
  const snapshot: WebTextHistorySnapshot = {
    id: `${sourceKey}:${timestamp.toString(36)}:${Math.random().toString(36).slice(2, 8)}`,
    sourceKey,
    fileName,
    content,
    size: new Blob([content]).size,
    timestamp,
  };
  const database = await openDatabase();
  try {
    const transaction = database.transaction(HISTORY_STORE_NAME, "readwrite");
    const store = transaction.objectStore(HISTORY_STORE_NAME);
    store.add(snapshot);
    for (const stale of existing.slice(29)) store.delete(stale.id);
    await transactionDone(transaction);
    return snapshot;
  } finally {
    database.close();
  }
}

export async function removeWebTextHistorySnapshot(id: string) {
  const database = await openDatabase();
  try {
    const transaction = database.transaction(HISTORY_STORE_NAME, "readwrite");
    transaction.objectStore(HISTORY_STORE_NAME).delete(id);
    await transactionDone(transaction);
  } finally {
    database.close();
  }
}
