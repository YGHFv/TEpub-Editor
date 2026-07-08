import { platform } from "$lib/platform";

export interface MobileExportResult {
    output_path: string;
    public_output: boolean;
    message: string;
    bytes?: number[];
    downloadUrl?: string;
    fileName?: string;
}

export interface MobileSelection {
    path: string;
    name: string;
}

export function safeFileName(name: string, fallbackExt: string) {
    const cleaned = name.replace(/[<>:"/\\|?*\u0000-\u001f]/g, "_").trim() || "selected";
    const hasExt = /\.[^.]+$/.test(cleaned);
    const ext = fallbackExt.replace(/^\./, "");
    return hasExt || !ext ? cleaned : `${cleaned}.${ext}`;
}

export function selectionName(path: string) {
    const clean = path.split(/[?#]/)[0];
    const name = clean.split(/[\\/]/).pop() || "selected";
    try {
        return decodeURIComponent(name);
    } catch {
        return name;
    }
}

export function buildMobileRoute(route: string, params: Record<string, string | null | undefined>) {
    const search = new URLSearchParams();
    for (const [key, value] of Object.entries(params)) {
        if (value && value.trim()) search.set(key, value);
    }
    const query = search.toString();
    return query ? `${route}?${query}` : route;
}

export function readMobileSelection(search: string): MobileSelection {
    const params = new URLSearchParams(search);
    const path = params.get("path")?.trim() ?? "";
    const name = params.get("name")?.trim() ?? "";
    return {
        path,
        name: name || selectionName(path),
    };
}

export function withTimeout<T>(task: Promise<T>, timeoutMs: number, label: string) {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const timeout = new Promise<T>((_, reject) => {
        timer = setTimeout(() => reject(new Error(label)), timeoutMs);
    });
    return Promise.race([task, timeout]).finally(() => {
        if (timer) clearTimeout(timer);
    });
}

async function cacheFileWithTauriFs(file: File, fallbackExt: string, bytes: Uint8Array) {
    const [{ appDataDir, join }, { mkdir, writeFile }] = await Promise.all([
        import("@tauri-apps/api/path"),
        import("@tauri-apps/plugin-fs"),
    ]);
    const root = await appDataDir();
    const dir = await join(root, "mobile-imports");
    await mkdir(dir, { recursive: true });
    const cachedPath = await join(dir, `${Date.now()}_${safeFileName(file.name, fallbackExt)}`);
    await withTimeout(writeFile(cachedPath, bytes), 45000, "write selected file cache timed out");
    return cachedPath;
}

export async function cacheBrowserFile(file: File, fallbackExt: string) {
    const bytes = new Uint8Array(
        await withTimeout(file.arrayBuffer(), 45000, "读取文件超时，请确认文件在本机可访问后重试。"),
    );
    if (!platform.isTauri) {
        return await withTimeout(
            platform.invoke<string>("mobile_cache_input_file", {
                sourceName: file.name,
                data: Array.from(bytes),
                fallbackExt,
            }),
            45000,
            "cache selected file timed out",
        );
    }
    return cacheFileWithTauriFs(file, fallbackExt, bytes);
}

export async function cacheBrowserFileStable(file: File, fallbackExt: string) {
    const bytes = new Uint8Array(await withTimeout(file.arrayBuffer(), 45000, "read selected file timed out"));

    if (bytes.byteLength <= 12 * 1024 * 1024) {
        try {
            return await withTimeout(
                platform.invoke<string>("mobile_cache_input_file", {
                    sourceName: file.name,
                    data: Array.from(bytes),
                    fallbackExt,
                }),
                45000,
                "cache selected file timed out",
            );
        } catch (err) {
            if (!platform.isTauri) throw err;
            console.warn("mobile_cache_input_file failed; falling back to plugin-fs cache write", err);
        }
    }

    if (!platform.isTauri) {
        throw new Error("Selected file is too large for the current web cache API.");
    }
    return cacheFileWithTauriFs(file, fallbackExt, bytes);
}

export async function offerSystemExport(path: string, fileName: string, bytes?: Uint8Array) {
    try {
        const data = bytes ?? (await platform.readFile(path));
        const outputName = safeFileName(fileName, "epub");
        const blob = new Blob([data], { type: "application/epub+zip" });
        const file = new File([blob], outputName, { type: "application/epub+zip" });
        const nav = navigator as Navigator & {
            canShare?: (data: ShareData & { files?: File[] }) => boolean;
            share?: (data: ShareData & { files?: File[] }) => Promise<void>;
        };

        if (nav.share && (!nav.canShare || nav.canShare({ files: [file] }))) {
            try {
                await nav.share({ files: [file], title: outputName, text: "TEpub-Editor EPUB 导出" });
                return `已打开系统分享/保存面板：${outputName}`;
            } catch (shareErr) {
                console.warn("System share/save failed; falling back to browser download", shareErr);
            }
        }

        const url = URL.createObjectURL(blob);
        const link = document.createElement("a");
        link.href = url;
        link.download = outputName;
        link.rel = "noopener";
        document.body.appendChild(link);
        link.click();
        link.remove();
        setTimeout(() => URL.revokeObjectURL(url), 30000);
        return `已触发系统下载：${outputName}`;
    } catch (err) {
        await platform.message(`系统另存/分享失败：${err}\n\n后端副本位置：${path}`, {
            title: "导出 EPUB",
            kind: "warning",
        });
        return `已导出后端副本：${path}`;
    }
}

export async function exportEpubPath(path: string, fileName: string) {
    const result = await platform.invoke<MobileExportResult>("mobile_export_epub", {
        epubPath: path,
        fileName,
    });
    const handoff = await offerSystemExport(
        result.downloadUrl || result.output_path,
        result.fileName || fileName,
        result.bytes ? new Uint8Array(result.bytes) : undefined,
    );
    return {
        ...result,
        message: result.public_output ? result.message : `${result.message}；${handoff}`,
    };
}
