export type WebTextHistorySnapshot = {
  id: string;
  sourceKey: string;
  fileName: string;
  content: string;
  size: number;
  timestamp: number;
};

// Keep the existing database name so editor history survives removal of the Web library UI.
const DB_NAME = "tepub-web-library";
const DB_VERSION = 2;
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
  if (typeof indexedDB === "undefined") throw new Error("当前浏览器不支持保存编辑历史");
  const request = indexedDB.open(DB_NAME, DB_VERSION);
  request.onupgradeneeded = () => {
    const database = request.result;
    if (!database.objectStoreNames.contains(HISTORY_STORE_NAME)) {
      const store = database.createObjectStore(HISTORY_STORE_NAME, { keyPath: "id" });
      store.createIndex("sourceKey", "sourceKey");
      store.createIndex("timestamp", "timestamp");
    }
  };
  return requestResult(request);
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
