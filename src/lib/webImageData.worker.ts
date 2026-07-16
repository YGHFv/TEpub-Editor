type WorkerRequest =
  | { id: number; mode: "embed"; width: number; height: number; pixels: ArrayBuffer; payload: ArrayBuffer }
  | { id: number; mode: "extract"; width: number; height: number; pixels: ArrayBuffer };

export {};

const MAGIC = new TextEncoder().encode("TEPUBWM2");
const workerScope = self as unknown as {
  onmessage: ((event: MessageEvent<WorkerRequest>) => void) | null;
  postMessage(message: unknown, transfer?: Transferable[]): void;
};

function checksum(bytes: Uint8Array) {
  let value = 2166136261;
  for (const byte of bytes) value = Math.imul(value ^ byte, 16777619) >>> 0;
  return value >>> 0;
}

function readUint32(bytes: Uint8Array, offset: number) {
  return (((bytes[offset] << 24) >>> 0) | (bytes[offset + 1] << 16) | (bytes[offset + 2] << 8) | bytes[offset + 3]) >>> 0;
}

workerScope.onmessage = (event: MessageEvent<WorkerRequest>) => {
  const request = event.data;
  const pixels = new Uint8ClampedArray(request.pixels);
  const capacity = Math.floor((request.width * request.height * 3) / 8);
  if (request.mode === "embed") {
    const payload = new Uint8Array(request.payload);
    if (capacity < payload.length) {
      workerScope.postMessage({ id: request.id, ok: false, pixels: request.pixels }, [request.pixels]);
      return;
    }
    for (let bitIndex = 0; bitIndex < payload.length * 8; bitIndex += 1) {
      const pixel = Math.floor(bitIndex / 3);
      const channel = bitIndex % 3;
      const bit = (payload[Math.floor(bitIndex / 8)] >>> (7 - (bitIndex % 8))) & 1;
      const offset = pixel * 4 + channel;
      pixels[offset] = (pixels[offset] & 0xfe) | bit;
    }
    workerScope.postMessage({ id: request.id, ok: true, pixels: pixels.buffer }, [pixels.buffer as ArrayBuffer]);
    return;
  }

  let bitIndex = 0;
  const read = (count: number) => {
    const output = new Uint8Array(count);
    for (let index = 0; index < count; index += 1) {
      let value = 0;
      for (let bit = 0; bit < 8; bit += 1) {
        const pixel = Math.floor(bitIndex / 3);
        const channel = bitIndex % 3;
        if (pixel >= request.width * request.height) return null;
        value = (value << 1) | (pixels[pixel * 4 + channel] & 1);
        bitIndex += 1;
      }
      output[index] = value;
    }
    return output;
  };
  const header = read(MAGIC.length + 8);
  if (!header || !MAGIC.every((byte, index) => header[index] === byte)) {
    workerScope.postMessage({ id: request.id, text: null });
    return;
  }
  const length = readUint32(header, MAGIC.length);
  const expected = readUint32(header, MAGIC.length + 4);
  if (length > capacity - header.length) {
    workerScope.postMessage({ id: request.id, text: null });
    return;
  }
  const data = read(length);
  workerScope.postMessage({ id: request.id, text: data && checksum(data) === expected ? new TextDecoder().decode(data) : null });
};
