export type ImageCompareFlow = "columns" | "rows";

/**
 * 对比预览按输出图比例排版，而不是按设备宽度或当前页面模式：
 * 竖图/方图并排更省高度，横图上下堆叠才不会把两张图都压成细条。
 */
export function resolveImageCompareFlow(
    width: number | null | undefined,
    height: number | null | undefined,
): ImageCompareFlow {
    const safeWidth = Number(width);
    const safeHeight = Number(height);
    if (
        !Number.isFinite(safeWidth) ||
        !Number.isFinite(safeHeight) ||
        safeWidth <= 0 ||
        safeHeight <= 0
    ) {
        return "columns";
    }
    return safeWidth > safeHeight ? "rows" : "columns";
}
