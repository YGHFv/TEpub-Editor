import { describe, expect, it } from "vitest";
import { resolveImageCompareFlow } from "./imageCompareLayout";

describe("resolveImageCompareFlow", () => {
    it("portrait covers use side-by-side columns", () => {
        expect(resolveImageCompareFlow(1400, 2400)).toBe("columns");
        expect(resolveImageCompareFlow(1200, 1600)).toBe("columns");
    });

    it("wide header images use stacked rows", () => {
        expect(resolveImageCompareFlow(2048, 1024)).toBe("rows");
        expect(resolveImageCompareFlow(1080, 360)).toBe("rows");
    });

    it("square images keep the compact two-column layout", () => {
        expect(resolveImageCompareFlow(1000, 1000)).toBe("columns");
    });

    it("invalid dimensions fall back safely to columns", () => {
        expect(resolveImageCompareFlow(0, 1000)).toBe("columns");
        expect(resolveImageCompareFlow(Number.NaN, 1000)).toBe("columns");
        expect(resolveImageCompareFlow(null, undefined)).toBe("columns");
    });
});
