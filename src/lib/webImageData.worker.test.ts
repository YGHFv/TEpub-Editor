import { afterEach, describe, expect, it, vi } from "vitest";

function checksum(bytes: Uint8Array) {
  let value = 0x811c9dc5;
  for (const byte of bytes) value = Math.imul(value ^ byte, 0x01000193) >>> 0;
  return value >>> 0;
}

function uint32Bytes(value: number) {
  return new Uint8Array([(value >>> 24) & 255, (value >>> 16) & 255, (value >>> 8) & 255, value & 255]);
}

function payload(text: string) {
  const magic = new TextEncoder().encode("TEPUBWM2");
  const data = new TextEncoder().encode(text);
  const result = new Uint8Array(magic.length + 8 + data.length);
  result.set(magic);
  result.set(uint32Bytes(data.length), magic.length);
  result.set(uint32Bytes(checksum(data)), magic.length + 4);
  result.set(data, magic.length + 8);
  return result;
}

afterEach(() => {
  vi.unstubAllGlobals();
  vi.resetModules();
});

describe("web image data worker", () => {
  it("round-trips the current TEPUBWM2 payload format", async () => {
    const replies: Array<Record<string, unknown>> = [];
    const scope = {
      onmessage: null as ((event: MessageEvent) => void) | null,
      postMessage(message: Record<string, unknown>) {
        replies.push(message);
      },
    };
    vi.stubGlobal("self", scope);
    await import("./webImageData.worker");

    const pixels = new Uint8ClampedArray(64 * 64 * 4).fill(128);
    scope.onmessage?.({ data: { id: 1, mode: "embed", width: 64, height: 64, pixels: pixels.buffer, payload: payload("测试水印").buffer } } as MessageEvent);
    const embedded = replies.shift();
    expect(embedded).toMatchObject({ id: 1, ok: true });

    scope.onmessage?.({ data: { id: 2, mode: "extract", width: 64, height: 64, pixels: embedded?.pixels } } as MessageEvent);
    expect(replies.shift()).toMatchObject({ id: 2, text: "测试水印" });
  });
});
