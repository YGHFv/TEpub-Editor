let requestId = 0;

function createWorker() {
  return new Worker(new URL("./webImageData.worker.ts", import.meta.url), { type: "module" });
}

export async function embedImagePayloadInWorker(image: ImageData, payload: Uint8Array, signal?: AbortSignal) {
  if (typeof Worker === "undefined") return null;
  signal?.throwIfAborted();
  const worker = createWorker();
  const id = ++requestId;
  return await new Promise<ImageData | false>((resolve, reject) => {
    const cleanup = () => {
      signal?.removeEventListener("abort", abort);
      worker.terminate();
    };
    const abort = () => { cleanup(); reject(new DOMException("操作已取消", "AbortError")); };
    signal?.addEventListener("abort", abort, { once: true });
    worker.onerror = (event) => { cleanup(); reject(new Error(event.message)); };
    worker.onmessage = (event) => {
      if (event.data.id !== id) return;
      cleanup();
      resolve(event.data.ok ? new ImageData(new Uint8ClampedArray(event.data.pixels), image.width, image.height) : false);
    };
    const pixels = image.data.slice().buffer;
    const payloadBuffer = payload.slice().buffer;
    worker.postMessage({ id, mode: "embed", width: image.width, height: image.height, pixels, payload: payloadBuffer }, [pixels, payloadBuffer]);
  });
}

export async function extractImagePayloadInWorker(image: ImageData, signal?: AbortSignal) {
  if (typeof Worker === "undefined") return undefined;
  signal?.throwIfAborted();
  const worker = createWorker();
  const id = ++requestId;
  return await new Promise<string | null>((resolve, reject) => {
    const cleanup = () => {
      signal?.removeEventListener("abort", abort);
      worker.terminate();
    };
    const abort = () => { cleanup(); reject(new DOMException("操作已取消", "AbortError")); };
    signal?.addEventListener("abort", abort, { once: true });
    worker.onerror = (event) => { cleanup(); reject(new Error(event.message)); };
    worker.onmessage = (event) => {
      if (event.data.id !== id) return;
      cleanup();
      resolve(event.data.text ?? null);
    };
    const pixels = image.data.slice().buffer;
    worker.postMessage({ id, mode: "extract", width: image.width, height: image.height, pixels }, [pixels]);
  });
}
