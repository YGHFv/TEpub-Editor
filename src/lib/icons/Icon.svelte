<script lang="ts" module>
    /**
     * 全应用统一图标集。
     *
     * 之前各处图标混用了四套语言：emoji（📂💾↩️，各系统字体不同）、
     * 文本符号（▦ ☷ ☰ ⚙ ⋯ ×）、CSS 拼的图形（.toolbox-entry-icon）、
     * 以及唯一一处 inline SVG（校对）。同一排按钮里粗细、大小、基线
     * 全不一样，跨平台更没法保证。
     *
     * 这里统一成一套 24 网格的描边图标：
     * - 只用 path，`fill:none` + `stroke:currentColor`，颜色跟随按钮状态；
     * - 线宽 1.8，圆角端点/连接，视觉重量与原先的校对图标一致；
     * - 无平台字体依赖，WebView2 / WebKit / Gecko 渲染完全相同。
     */
    export type IconName =
        | "folder"
        | "folder-open"
        | "save"
        | "undo"
        | "redo"
        | "book-open"
        | "books"
        | "search"
        | "proof"
        | "settings"
        | "monitor"
        | "grid"
        | "list"
        | "list-cover"
        | "refresh"
        | "close"
        | "plus"
        | "minus"
        | "chevron-down"
        | "chevron-right"
        | "expand-vertical"
        | "file"
        | "file-css"
        | "file-xml"
        | "image"
        | "font"
        | "copy"
        | "check"
        | "trash";

    const PATHS: Record<IconName, string[]> = {
        folder: [
            "M3 7.6A1.6 1.6 0 0 1 4.6 6h3.7l2 2.3h9.1A1.6 1.6 0 0 1 21 9.9v8.5a1.6 1.6 0 0 1-1.6 1.6H4.6A1.6 1.6 0 0 1 3 18.4Z",
            "M3 10.9h18",
        ],
        "folder-open": [
            "M3 8.4A1.6 1.6 0 0 1 4.6 6.8h3.7l2 2.3H19a1.6 1.6 0 0 1 1.6 1.6v.6",
            "M3 9.9h17.3l-1.9 8.2a1.7 1.7 0 0 1-1.66 1.3H5.36a1.7 1.7 0 0 1-1.66-1.3Z",
        ],
        save: [
            "M5.4 4h9.8L20 8.8v10.7A1.5 1.5 0 0 1 18.5 21h-13A1.5 1.5 0 0 1 4 19.5v-14A1.5 1.5 0 0 1 5.4 4Z",
            "M8 4v5.2h6.8V4",
            "M7.6 21v-5.6h8.8V21",
        ],
        undo: [
            "M8.2 14.6 4 10.4l4.2-4.2",
            "M4.2 10.4h9.5a5.4 5.4 0 0 1 0 10.8h-3.4",
        ],
        redo: [
            "M15.8 14.6 20 10.4l-4.2-4.2",
            "M19.8 10.4h-9.5a5.4 5.4 0 0 0 0 10.8h3.4",
        ],
        "book-open": [
            "M12 7.6C9.9 5.9 7.6 5.2 4.4 5.2v12.2c3.3 0 5.6.7 7.6 2.4 2-1.7 4.3-2.4 7.6-2.4V5.2c-3.2 0-5.5.7-7.6 2.4Z",
            "M12 7.6v12.2",
        ],
        books: [
            "M4.2 4.8h3.3v14.4H4.2Z",
            "M9.6 4.8h3.3v14.4H9.6Z",
            "m15 5.6 3.1-.85a.6.6 0 0 1 .74.42l3.05 12.1-3.1.85a.6.6 0 0 1-.74-.42Z",
        ],
        search: [
            "M10.9 4.4a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13Z",
            "m15.7 15.7 4.2 4.2",
        ],
        proof: [
            "M4.6 3.6h11.1a3.8 3.8 0 0 1 3.8 3.8v2.4",
            "M5 4.6h10.2a2.3 2.3 0 0 1 2.3 2.3v3.4",
            "M5 8.6h8.4",
            "M5 12.6h6.2",
            "M5 16.6h4.1",
            "m14.2 16.8 2.05 2.05 4.55-4.55",
        ],
        settings: [
            "M12 8.7a3.3 3.3 0 1 0 0 6.6 3.3 3.3 0 0 0 0-6.6Z",
            "M19.2 13.6a7.7 7.7 0 0 0 0-3.2l1.9-1.3-1.9-3.3-2.2.9a7.8 7.8 0 0 0-2.7-1.6L13.9 3h-3.8l-.4 2.1a7.8 7.8 0 0 0-2.7 1.6l-2.2-.9-1.9 3.3 1.9 1.3a7.7 7.7 0 0 0 0 3.2l-1.9 1.3 1.9 3.3 2.2-.9a7.8 7.8 0 0 0 2.7 1.6l.4 2.1h3.8l.4-2.1a7.8 7.8 0 0 0 2.7-1.6l2.2.9 1.9-3.3Z",
        ],
        monitor: ["M3.8 5.2h16.4v10.4H3.8Z", "M12 15.6v3.6", "M8.6 19.4h6.8"],
        grid: [
            "M4.4 4.4h6.2v6.2H4.4Z",
            "M13.4 4.4h6.2v6.2h-6.2Z",
            "M4.4 13.4h6.2v6.2H4.4Z",
            "M13.4 13.4h6.2v6.2h-6.2Z",
        ],
        list: [
            "M4.4 5.9h1.8v1.8H4.4Z",
            "M4.4 11.1h1.8v1.8H4.4Z",
            "M4.4 16.3h1.8v1.8H4.4Z",
            "M9.4 6.8h10.2",
            "M9.4 12h10.2",
            "M9.4 17.2h10.2",
        ],
        "list-cover": [
            "M4.4 4.6h4v5h-4Z",
            "M11 5.7h8.6",
            "M11 8.5h5.6",
            "M4.4 13.6h4v5h-4Z",
            "M11 14.7h8.6",
            "M11 17.5h5.6",
        ],
        refresh: ["M20 12a8 8 0 1 1-2.7-6", "M20 4.4V9h-4.6"],
        close: ["M6.4 6.4 17.6 17.6", "M17.6 6.4 6.4 17.6"],
        plus: ["M12 5.4v13.2", "M5.4 12h13.2"],
        minus: ["M5.4 12h13.2"],
        "chevron-down": ["m6.6 9.6 5.4 5.4 5.4-5.4"],
        "chevron-right": ["m9.6 6.6 5.4 5.4-5.4 5.4"],
        "expand-vertical": [
            "M12 4.6v14.8",
            "m8.6 8 3.4-3.4L15.4 8",
            "m8.6 16 3.4 3.4L15.4 16",
        ],
        file: ["M6.2 3.6h6.9l4.9 4.9v11.9H6.2Z", "M13.1 3.6v4.9h4.9"],
        "file-css": [
            "M6.2 3.6h6.9l4.9 4.9v11.9H6.2Z",
            "M13.1 3.6v4.9h4.9",
            "M10 12.4v5.6",
            "M14.2 12.4v5.6",
            "M8.9 13.9h6.4",
            "M8.9 16.6h6.4",
        ],
        "file-xml": [
            "M6.2 3.6h6.9l4.9 4.9v11.9H6.2Z",
            "M13.1 3.6v4.9h4.9",
            "m10.2 13-2 2.6 2 2.6",
            "m14 13 2 2.6-2 2.6",
        ],
        image: [
            "M4.2 5.4h15.6v13.2H4.2Z",
            "m4.2 15.3 4.6-4.6 3.4 3.4 2.7-2.7 4.9 4.9",
            "M9.5 9.1a1.35 1.35 0 1 0 0 2.7 1.35 1.35 0 0 0 0-2.7Z",
        ],
        font: ["M5 19.4 12 4.6l7 14.8", "M8.2 13.6h7.6"],
        copy: ["M9.2 9.2h9.4v9.4H9.2Z", "M6.6 14.8H5.4V5.4h9.4v1.2"],
        check: ["m5.2 12.4 4.6 4.6L19 6.8"],
        trash: [
            "M5.4 7.4h13.2",
            "M9.4 7.4V4.8h5.2v2.6",
            "m7.2 7.4.9 12.8h7.8l.9-12.8",
        ],
    };

    export function iconPaths(name: IconName): string[] {
        return PATHS[name];
    }
</script>

<script lang="ts">
    let {
        name,
        size = 18,
        strokeWidth = 1.8,
        label,
    }: {
        name: IconName;
        /** 光学尺寸（px）。按钮 34~36px 高时用 18，小按钮用 16。 */
        size?: number;
        strokeWidth?: number;
        /** 有值则作为无障碍名称；纯装饰图标不传，自动 aria-hidden */
        label?: string;
    } = $props();
</script>

<svg
    viewBox="0 0 24 24"
    width={size}
    height={size}
    fill="none"
    stroke="currentColor"
    stroke-width={strokeWidth}
    stroke-linecap="round"
    stroke-linejoin="round"
    role={label ? "img" : undefined}
    aria-label={label}
    aria-hidden={label ? undefined : "true"}
    focusable="false"
>
    {#each PATHS[name] as d (d)}
        <path {d} />
    {/each}
</svg>

<style>
    svg {
        display: block;
        flex: 0 0 auto;
        /* 图标只吃线条颜色，任何状态（hover/active/disabled）都由 currentColor 传递 */
        color: inherit;
    }
</style>
