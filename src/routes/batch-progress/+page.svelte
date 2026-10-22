<script lang="ts">
    import { onMount } from "svelte";
    import { invoke } from "@tauri-apps/api/core";
    import { listen, type UnlistenFn } from "@tauri-apps/api/event";
    import { getCurrentWindow } from "@tauri-apps/api/window";
    import { ask, open, save } from "@tauri-apps/plugin-dialog";
    import { writeTextFile } from "@tauri-apps/plugin-fs";
    import CustomSelect from "$lib/CustomSelect.svelte";
    import { formatTimeLocalized, localizeUiData, t } from "$lib/i18n.svelte";
    import { batchTaskKey } from "$lib/storageKeys";

    type BatchEvent = {
        taskId: string;
        event: string;
        level: "info" | "warning" | "error" | string;
        index: number;
        total: number;
        inputPath?: string | null;
        outputPath?: string | null;
        message: string;
    };

    type QueueStatus = "waiting" | "running" | "done" | "warning" | "error";
    type LogLevel = "all" | "info" | "warning" | "error";
    type QueueFilter = "all" | QueueStatus | "issue";

    type QueueRow = {
        inputPath: string;
        outputPath: string;
        status: QueueStatus;
        message: string;
    };

    type LogRow = {
        level: LogLevel;
        text: string;
    };

    type BatchTaskConfig = {
        taskId: string;
        tool: string;
        toolTitle: string;
        inputPaths: string[];
        sourceDirectories?: string[];
        outputDir?: string;
        resolvedOutputDir?: string;
        imageFormat?: string;
        outputMode?: string;
        conflictMode?: string;
    };

    const OUTPUT_MODES: { value: string; label: string; hint: string }[] = [
        {
            value: "batch_dir",
            label: "输出到批量目录",
            hint: "统一放到所选输出目录（默认）",
        },
        {
            value: "same_dir",
            label: "输出到源文件同目录",
            hint: "与原 EPUB 并列存放",
        },
        {
            value: "preserve_structure",
            label: "保留目录结构",
            hint: "在输出目录下重建源文件相对子路径",
        },
    ];

    const CONFLICT_MODES: { value: string; label: string; hint: string }[] = [
        {
            value: "auto_number",
            label: "自动编号",
            hint: "重名时追加 _2/_3 等后缀（默认）",
        },
        { value: "skip", label: "跳过已存在", hint: "目标已存在则跳过该文件" },
        { value: "overwrite", label: "覆盖", hint: "直接覆盖同名目标文件" },
    ];

    const OUTPUT_MODE_OPTIONS = OUTPUT_MODES.map((mode) => ({
        value: mode.value,
        label: mode.label,
        meta: mode.hint,
    }));
    const CONFLICT_MODE_OPTIONS = CONFLICT_MODES.map((mode) => ({
        value: mode.value,
        label: mode.label,
        meta: mode.hint,
    }));

    type BatchSummary = {
        taskId: string;
        total: number;
        succeeded: number;
        failed: number;
    };

    type BatchScanResult = {
        inputPaths: string[];
        outputDir: string;
    };

    const params = new URLSearchParams(
        typeof window === "undefined" ? "" : window.location.search,
    );
    const taskId = params.get("taskId") ?? "";
    const toolTitleParam = params.get("tool") ?? "批量处理";

    const TOOL_META: Record<string, { title: string; detail: string }> = {
        "epub-diagnose": {
            title: "EPUB Diagnose",
            detail: "Batch check EPUB structure, references, and manifest issues",
        },
        "file-encrypt": {
            title: "文件加密",
            detail: "混淆 EPUB 内部文件名并同步引用",
        },
        "file-decrypt": {
            title: "文件解密",
            detail: "还原混淆文件名，支持单本、多本和目录扫描",
        },
        "font-encrypt": {
            title: "字体加密",
            detail: "对 EPUB 正文字形进行不可逆混淆处理",
        },
        "font-decrypt": {
            title: "字体解密",
            detail: "使用保存的映射或 TXT 对齐还原字体混淆",
        },
        "epub-reformat": {
            title: "EPUB 重构",
            detail: "整理 EPUB 目录结构并重写内部引用",
        },
        "image-convert": {
            title: "图片转换",
            detail: "转换 EPUB 内 WebP 图片并更新引用",
        },
    };

    let tool = "";
    let inputPaths: string[] = [];
    let sourceDirectories: string[] = [];
    let outputDir = "";
    let resolvedOutputDir = "";
    let imageFormat: string | undefined;
    let outputMode = "batch_dir";
    let conflictMode = "auto_number";
    let total = 0;
    let current = 0;
    let running = false;
    let cancelRequested = false;
    let scanning = false;
    let done = false;
    let summary = "请选择 EPUB 文件或扫描目录";
    let rows: QueueRow[] = [];
    let logs: LogRow[] = [];
    let logFilter: LogLevel = "all";
    let queueFilter: QueueFilter = "all";
    let queueSearch = "";

    $: currentMeta = localizeUiData(
        TOOL_META[tool] ?? {
            title: t(toolTitleParam),
            detail: "批量执行 EPUB 工具",
        },
    );
    $: percent = total > 0 ? Math.round((current / total) * 100) : 0;
    $: canStart = inputPaths.length > 0 && !running && !scanning;
    $: outputLabel =
        outputDir ||
        resolvedOutputDir ||
        t("默认：所选文件夹下 TEpub-batch-output");
    $: filteredLogs =
        logFilter === "all"
            ? logs
            : logs.filter((log) => log.level === logFilter);
    $: normalizedQueueSearch = queueSearch.trim().toLowerCase();
    $: visibleRows = rows.filter((row) => {
        const statusMatches =
            queueFilter === "all" ||
            row.status === queueFilter ||
            (queueFilter === "issue" &&
                (row.status === "warning" || row.status === "error"));
        if (!statusMatches) return false;
        if (!normalizedQueueSearch) return true;
        return `${basename(row.inputPath)} ${row.inputPath} ${row.outputPath} ${row.message}`
            .toLowerCase()
            .includes(normalizedQueueSearch);
    });
    $: queueStats = rows.reduce<Record<QueueStatus, number>>(
        (acc, row) => {
            acc[row.status] += 1;
            return acc;
        },
        { waiting: 0, running: 0, done: 0, warning: 0, error: 0 },
    );
    $: failedRows = rows.filter((row) => row.status === "error");
    $: completedRows = rows.filter((row) => row.status === "done");
    $: warningRows = rows.filter((row) => row.status === "warning");

    function basename(path: string) {
        return path.split(/[\\/]/).pop() || path;
    }

    function statusLabel(status: QueueStatus) {
        switch (status) {
            case "running":
                return t("处理中");
            case "done":
                return t("完成");
            case "warning":
                return t("跳过");
            case "error":
                return t("失败");
            default:
                return t("等待");
        }
    }

    function logLine(
        message: string,
        prefix = currentMeta.title,
        level: LogLevel = "info",
    ) {
        const time = formatTimeLocalized();
        logs = [
            { level, text: `[${time}] ${prefix}: ${message}` },
            ...logs,
        ].slice(0, 400);
    }

    function pushEventLog(event: BatchEvent) {
        const level =
            event.level === "error" || event.level === "warning"
                ? event.level
                : "info";
        logLine(
            event.message,
            event.inputPath ? basename(event.inputPath) : currentMeta.title,
            level,
        );
    }

    function upsertRow(event: BatchEvent, status: QueueStatus) {
        if (!event.inputPath) return;
        const previous = rows.find(
            (item) => item.inputPath === event.inputPath,
        );
        const row: QueueRow = {
            inputPath: event.inputPath,
            outputPath: event.outputPath ?? previous?.outputPath ?? "",
            status,
            message: event.message,
        };
        const existing = rows.findIndex(
            (item) => item.inputPath === event.inputPath,
        );
        if (existing >= 0) {
            rows = rows.map((item, index) =>
                index === existing ? { ...item, ...row } : item,
            );
        } else {
            rows = [...rows, row];
        }
    }

    function addInputRows(paths: string[], message: string) {
        const existing = new Set(
            rows.map((row) => row.inputPath.toLowerCase()),
        );
        const nextRows = [...rows];
        for (const path of paths) {
            const key = path.toLowerCase();
            if (existing.has(key)) continue;
            existing.add(key);
            nextRows.push({
                inputPath: path,
                outputPath: "",
                status: "waiting",
                message,
            });
        }
        rows = nextRows;
    }

    function addInputPaths(paths: string[], message: string) {
        const seen = new Set(inputPaths.map((path) => path.toLowerCase()));
        const added: string[] = [];
        for (const path of paths) {
            if (!path || seen.has(path.toLowerCase())) continue;
            seen.add(path.toLowerCase());
            added.push(path);
        }
        if (added.length === 0) return;
        inputPaths = [...inputPaths, ...added];
        addInputRows(added, message);
        summary = t("已加入 {count} 个输入源", {
            count: inputPaths.length,
        });
        logLine(t("加入 {count} 个输入源", { count: added.length }));
        saveTaskConfig();
    }

    function applyEvent(event: BatchEvent) {
        if (event.taskId !== taskId) return;
        total = event.total || total;
        if (event.outputPath && event.event === "scan-start") {
            resolvedOutputDir = event.outputPath;
        }
        if (event.event === "scan-start") {
            rows = [];
            current = 0;
            total = 0;
            summary = event.message;
        } else if (event.event === "scan-file") {
            summary = event.message;
            upsertRow(event, "waiting");
        } else if (event.event === "started") {
            summary = event.message;
        } else if (event.event === "file-start") {
            current = Math.max(current, event.index - 1);
            summary = event.message;
            upsertRow(event, "running");
        } else if (event.event === "file-stage") {
            current = Math.max(current, event.index - 1);
            summary = event.message;
            upsertRow(event, "running");
        } else if (event.event === "file-done") {
            current = Math.max(current, event.index);
            summary = event.message;
            upsertRow(event, event.level === "warning" ? "warning" : "done");
        } else if (event.event === "file-error") {
            current = Math.max(current, event.index);
            summary = event.message;
            upsertRow(event, "error");
        } else if (event.event === "finished") {
            current = event.total;
            running = false;
            cancelRequested = false;
            done = true;
            summary = event.message;
        }
        pushEventLog(event);
    }

    function readTaskConfig() {
        if (typeof window === "undefined") return null;
        const raw = localStorage.getItem(batchTaskKey(taskId));
        if (!raw) return null;
        try {
            return JSON.parse(raw) as BatchTaskConfig;
        } catch {
            return null;
        }
    }

    function saveTaskConfig() {
        if (typeof window === "undefined") return;
        localStorage.setItem(
            batchTaskKey(taskId),
            JSON.stringify({
                taskId,
                tool,
                toolTitle: currentMeta.title,
                inputPaths,
                sourceDirectories,
                outputDir: outputDir || undefined,
                resolvedOutputDir: resolvedOutputDir || undefined,
                imageFormat,
                outputMode,
                conflictMode,
            }),
        );
    }

    async function chooseFiles() {
        if (running || scanning) return;
        const selected = await open({
            multiple: true,
            filters: [{ name: "EPUB 文件", extensions: ["epub"] }],
        });
        if (!selected) return;
        addInputPaths(
            Array.isArray(selected) ? selected : [selected],
            "文件待处理",
        );
    }

    async function scanDirectory() {
        if (running || scanning) return;
        const selected = await open({
            directory: true,
            multiple: true,
            title: "选择 EPUB 目录",
        });
        if (!selected) return;
        const directories = Array.isArray(selected) ? selected : [selected];
        sourceDirectories = directories;
        await scanInputDirectories(directories);
    }

    async function scanInputDirectories(directories: string[]) {
        if (directories.length === 0 || running || scanning) return;
        scanning = true;
        done = false;
        total = 0;
        current = 0;
        summary = t("正在扫描目录中的 EPUB 文件");
        rows = directories.map((path) => ({
            inputPath: path,
            outputPath: "",
            status: "running",
            message: t("正在扫描目录"),
        }));
        logLine(t("开始扫描 {count} 个目录", { count: directories.length }));
        try {
            const result = await invoke<BatchScanResult>(
                "toolbox_scan_batch_inputs",
                {
                    inputPaths: directories,
                    outputDir: outputDir || undefined,
                },
            );
            resolvedOutputDir = result.outputDir || "";
            inputPaths = [];
            rows = [];
            addInputPaths(result.inputPaths, "扫描完成，等待执行");
            total = result.inputPaths.length;
            current = 0;
            summary =
                result.inputPaths.length > 0
                    ? t("扫描到 {count} 个 EPUB 文件", {
                          count: result.inputPaths.length,
                      })
                    : t("目录中未找到 EPUB 文件");
            logLine(summary);
            saveTaskConfig();
        } catch (e: any) {
            summary = t("目录扫描失败: {error}", { error: e });
            rows = directories.map((path) => ({
                inputPath: path,
                outputPath: "",
                status: "error",
                message: summary,
            }));
            logLine(summary, currentMeta.title, "error");
        } finally {
            scanning = false;
        }
    }

    async function rescanDirectories() {
        if (sourceDirectories.length === 0 || running || scanning) {
            summary = t("没有可重新扫描的目录");
            return;
        }
        await scanInputDirectories(sourceDirectories);
    }

    async function chooseOutputDir() {
        if (running || scanning) return;
        const selected = await open({
            directory: true,
            multiple: false,
            title: "选择输出目录",
        });
        if (!selected || Array.isArray(selected)) return;
        outputDir = selected;
        resolvedOutputDir = "";
        summary = t("已选择输出目录");
        logLine(t("输出目录: {path}", { path: outputDir }));
        saveTaskConfig();
    }

    function resetOutputDir() {
        if (running || scanning) return;
        outputDir = "";
        resolvedOutputDir = "";
        summary = t("已恢复默认输出目录");
        logLine(t("已恢复默认输出目录"));
        saveTaskConfig();
    }

    function clearQueue() {
        if (running || scanning) return;
        inputPaths = [];
        sourceDirectories = [];
        rows = [];
        total = 0;
        current = 0;
        done = false;
        resolvedOutputDir = "";
        summary = t("队列已清空");
        logLine(t("队列已清空"));
        saveTaskConfig();
    }

    function removeInputPath(path: string) {
        if (running || scanning) return;
        const key = path.toLowerCase();
        inputPaths = inputPaths.filter((item) => item.toLowerCase() !== key);
        rows = rows.filter((row) => row.inputPath.toLowerCase() !== key);
        total = rows.length;
        current = Math.min(current, total);
        summary =
            inputPaths.length > 0
                ? t("已加入 {count} 个输入源", {
                      count: inputPaths.length,
                  })
                : t("队列已清空");
        logLine(t("移除队列项: {name}", { name: basename(path) }));
        saveTaskConfig();
    }

    async function revealPath(path: string) {
        if (!path) return;
        try {
            await invoke("reveal_in_explorer", { path });
        } catch (e: any) {
            summary = t("无法打开位置: {error}", { error: e });
            logLine(summary, currentMeta.title, "error");
        }
    }

    async function openOutputDir() {
        const path = outputDir || resolvedOutputDir;
        if (!path) {
            summary = t("还没有可打开的输出目录");
            return;
        }
        await revealPath(path);
    }

    function clearLog() {
        logs = [];
    }

    async function exportLogs() {
        if (logs.length === 0) return;
        const selected = await save({
            title: t("导出批量处理日志"),
            defaultPath: `${currentMeta.title}-${t("批量日志")}.txt`,
            filters: [{ name: t("文本文件"), extensions: ["txt"] }],
        });
        if (!selected) return;
        const body = logs
            .slice()
            .reverse()
            .map((log) => log.text)
            .join("\n");
        try {
            await writeTextFile(selected, body);
            summary = t("日志已导出");
            logLine(t("日志已导出: {path}", { path: selected }));
        } catch (e: any) {
            summary = t("日志导出失败: {error}", { error: e });
            logLine(summary, currentMeta.title, "error");
        }
    }

    function buildBatchReport() {
        const lines: string[] = [];
        lines.push(`${currentMeta.title} 批量处理报告`);
        lines.push(`生成时间: ${new Date().toLocaleString()}`);
        lines.push(`工具: ${tool || currentMeta.title}`);
        lines.push(
            `输出目录: ${outputDir || resolvedOutputDir || "默认输出目录"}`,
        );
        lines.push(
            `输出位置: ${OUTPUT_MODES.find((m) => m.value === outputMode)?.label ?? outputMode}`,
        );
        lines.push(
            `冲突处理: ${CONFLICT_MODES.find((m) => m.value === conflictMode)?.label ?? conflictMode}`,
        );
        lines.push(`摘要: ${summary}`);
        lines.push(
            `队列: ${rows.length}，完成: ${completedRows.length}，跳过: ${warningRows.length}，失败: ${failedRows.length}`,
        );
        lines.push("");
        lines.push("文件队列:");
        rows.forEach((row, index) => {
            lines.push(
                `${index + 1}. [${statusLabel(row.status)}] ${basename(row.inputPath)}`,
            );
            lines.push(`   输入: ${row.inputPath}`);
            if (row.outputPath) lines.push(`   输出: ${row.outputPath}`);
            lines.push(`   信息: ${row.message}`);
        });
        lines.push("");
        lines.push("日志:");
        if (logs.length === 0) {
            lines.push("无日志");
        } else {
            logs.slice()
                .reverse()
                .forEach((log) => lines.push(log.text));
        }
        return lines.join("\n");
    }

    async function exportReport() {
        if (rows.length === 0) return;
        const selected = await save({
            title: t("导出批量处理报告"),
            defaultPath: `${currentMeta.title}-${t("批量处理报告")}.txt`,
            filters: [{ name: t("文本文件"), extensions: ["txt"] }],
        });
        if (!selected) return;
        try {
            await writeTextFile(selected, buildBatchReport());
            summary = t("批量处理报告已导出");
            logLine(t("批量处理报告已导出: {path}", { path: selected }));
        } catch (e: any) {
            summary = t("报告导出失败: {error}", { error: e });
            logLine(summary, currentMeta.title, "error");
        }
    }

    function retryFailedRows() {
        if (running || scanning || failedRows.length === 0) return;
        const retryPaths = failedRows.map((row) => row.inputPath);
        inputPaths = retryPaths;
        rows = retryPaths.map((path) => ({
            inputPath: path,
            outputPath: "",
            status: "waiting",
            message: t("等待重试"),
        }));
        total = rows.length;
        current = 0;
        done = false;
        cancelRequested = false;
        queueFilter = "all";
        summary = t("已准备重试 {count} 个失败文件", {
            count: rows.length,
        });
        logLine(summary, currentMeta.title, "warning");
        saveTaskConfig();
    }

    async function cancelBatch() {
        if (!running || cancelRequested) return;
        cancelRequested = true;
        summary = t("正在请求取消，当前文件处理完成后停止");
        logLine(t("已请求取消批量任务"), currentMeta.title, "warning");
        try {
            await invoke("toolbox_cancel_batch", { taskId });
        } catch (e: any) {
            cancelRequested = false;
            summary = t("取消请求失败: {error}", { error: e });
            logLine(summary, currentMeta.title, "error");
        }
    }

    async function startBatch() {
        if (!canStart) {
            summary = t("请先加入 EPUB 文件或目录");
            return;
        }
        running = true;
        cancelRequested = false;
        done = false;
        total = 0;
        current = 0;
        resolvedOutputDir = "";
        rows = [];
        summary = t("正在启动批量任务");
        logLine(t("开始执行批量任务"));
        saveTaskConfig();

        try {
            const result = await invoke<BatchSummary>("toolbox_run_batch", {
                taskId,
                tool,
                inputPaths,
                imageFormat,
                outputDir: outputDir || resolvedOutputDir || undefined,
                outputMode,
                conflictMode,
            });
            running = false;
            cancelRequested = false;
            done = true;
            total = result.total;
            current = result.total;
            if (result.total === 0) {
                summary = t("未找到 EPUB 文件");
            }
        } catch (e: any) {
            running = false;
            cancelRequested = false;
            done = true;
            summary = t("批量任务失败: {error}", { error: e });
            logLine(summary, currentMeta.title, "error");
        }
    }

    onMount(() => {
        const config = readTaskConfig();
        if (config) {
            tool = config.tool;
            inputPaths = config.inputPaths ?? [];
            sourceDirectories = config.sourceDirectories ?? [];
            outputDir = config.outputDir ?? "";
            resolvedOutputDir = config.resolvedOutputDir ?? "";
            imageFormat = config.imageFormat;
            outputMode = config.outputMode ?? "batch_dir";
            conflictMode = config.conflictMode ?? "auto_number";
            if (inputPaths.length > 0) {
                addInputRows(inputPaths, t("等待执行"));
                summary = t("已加入 {count} 个输入源", {
                    count: inputPaths.length,
                });
            }
        } else {
            summary = t("未找到批量任务配置，请从工具箱重新打开");
        }

        let unlisten: UnlistenFn | undefined;
        let unlistenClose: UnlistenFn | undefined;
        listen<BatchEvent>("toolbox-batch-event", (event) =>
            applyEvent(event.payload),
        ).then((fn) => {
            unlisten = fn;
        });
        getCurrentWindow()
            .onCloseRequested(async (event) => {
                if (!running && !scanning) return;
                event.preventDefault();
                const ok = await ask(
                    t("批量任务仍在执行或扫描，确定关闭窗口吗？"),
                    {
                        title: t("关闭批量窗口"),
                        kind: "warning",
                    },
                );
                if (!ok) return;
                if (running) {
                    await cancelBatch();
                }
                await getCurrentWindow().destroy();
            })
            .then((fn) => {
                unlistenClose = fn;
            });
        return () => {
            unlisten?.();
            unlistenClose?.();
        };
    });
</script>

<main class="batch-app">
    <header class="batch-head">
        <div>
            <div class="eyebrow">功能执行</div>
            <h1>{currentMeta.title}</h1>
            <p>{currentMeta.detail}</p>
        </div>
    </header>

    <section class="source-panel" aria-label="输入源">
        <div class="source-copy">
            <div class="eyebrow">输入源</div>
            <h2>选择目录或添加 EPUB 文件</h2>
            <p>当前队列 {inputPaths.length} 个输入源。</p>
        </div>
        <div class="source-actions">
            <button
                class="primary-btn"
                type="button"
                on:click={scanDirectory}
                disabled={running || scanning}
            >
                {scanning ? "扫描中" : "选择目录"}
            </button>
            <button
                class="strong-btn"
                type="button"
                on:click={chooseFiles}
                disabled={running || scanning}>添加 EPUB</button
            >
            <button
                class="ghost-btn"
                type="button"
                on:click={clearQueue}
                disabled={running || scanning || inputPaths.length === 0}
                >清空队列</button
            >
        </div>
    </section>

    <section class="workspace">
        <div class="left-column">
            <section class="panel execute-panel" aria-label="任务配置">
                <div class="panel-head">
                    <div>
                        <div class="eyebrow">任务配置</div>
                        <h2>输出与执行</h2>
                    </div>
                    <div class="panel-actions">
                        <button
                            class="ghost-btn"
                            type="button"
                            on:click={chooseOutputDir}
                            disabled={running || scanning}>选择输出目录</button
                        >
                        <button
                            class="ghost-btn"
                            type="button"
                            on:click={openOutputDir}
                            disabled={running ||
                                scanning ||
                                !(outputDir || resolvedOutputDir)}
                            >打开输出目录</button
                        >
                        <button
                            class="ghost-btn"
                            type="button"
                            on:click={resetOutputDir}
                            disabled={running || scanning || !outputDir}
                            >重置输出路径</button
                        >
                    </div>
                </div>

                <div class="field-block">
                    <span>输出目录</span>
                    <strong title={outputLabel}>{outputLabel}</strong>
                </div>

                <div class="field-row">
                    <div
                        class="field-select"
                        title={t(
                            OUTPUT_MODES.find((m) => m.value === outputMode)
                                ?.hint ?? "",
                        )}
                    >
                        <span>输出位置</span>
                        <CustomSelect
                            className="batch-select"
                            value={outputMode}
                            options={OUTPUT_MODE_OPTIONS}
                            disabled={running || scanning}
                            on:change={(e) => {
                                outputMode = e.detail;
                                saveTaskConfig();
                            }}
                        />
                    </div>
                    <div
                        class="field-select"
                        title={t(
                            CONFLICT_MODES.find((m) => m.value === conflictMode)
                                ?.hint ?? "",
                        )}
                    >
                        <span>冲突处理</span>
                        <CustomSelect
                            className="batch-select"
                            value={conflictMode}
                            options={CONFLICT_MODE_OPTIONS}
                            disabled={running || scanning}
                            on:change={(e) => {
                                conflictMode = e.detail;
                                saveTaskConfig();
                            }}
                        />
                    </div>
                </div>

                <div class="tool-run-row">
                    <div class="tool-summary">
                        <strong>{currentMeta.title}</strong>
                        <span>{t(summary)}</span>
                    </div>
                    <button
                        class="start-btn"
                        type="button"
                        on:click={startBatch}
                        disabled={!canStart}
                    >
                        {running ? "执行中" : scanning ? "扫描中" : "开始执行"}
                    </button>
                    {#if running}
                        <button
                            class="cancel-btn"
                            type="button"
                            on:click={cancelBatch}
                            disabled={cancelRequested}
                        >
                            {cancelRequested ? "取消中" : "取消任务"}
                        </button>
                    {/if}
                    {#if done && failedRows.length > 0}
                        <button
                            class="retry-btn"
                            type="button"
                            on:click={retryFailedRows}
                            disabled={running || scanning}>重试失败</button
                        >
                    {/if}
                    {#if done && rows.length > 0}
                        <button
                            class="report-btn"
                            type="button"
                            on:click={exportReport}>导出报告</button
                        >
                    {/if}
                </div>

                <div class="progress-area" aria-label="批量进度">
                    <div class="progress-meta">
                        <span>{current} / {total}</span>
                        <strong>{percent}%</strong>
                    </div>
                    <div class="progress-track">
                        <div
                            class="progress-bar"
                            style={`width: ${percent}%`}
                        ></div>
                    </div>
                </div>
                {#if done && rows.length > 0}
                    <div class="completion-summary" aria-label="完成摘要">
                        <span>完成 {completedRows.length}</span>
                        <span>跳过 {warningRows.length}</span>
                        <span class:error={failedRows.length > 0}
                            >失败 {failedRows.length}</span
                        >
                    </div>
                {/if}
            </section>

            <section class="panel log-panel" aria-label="处理日志">
                <div class="panel-head">
                    <div>
                        <div class="eyebrow">过程</div>
                        <h2>处理日志</h2>
                    </div>
                    <div class="log-actions">
                        <div class="segmented" aria-label="日志筛选">
                            <button
                                type="button"
                                class:active={logFilter === "all"}
                                on:click={() => (logFilter = "all")}
                                >全部</button
                            >
                            <button
                                type="button"
                                class:active={logFilter === "error"}
                                on:click={() => (logFilter = "error")}
                                >错误</button
                            >
                            <button
                                type="button"
                                class:active={logFilter === "warning"}
                                on:click={() => (logFilter = "warning")}
                                >警告</button
                            >
                        </div>
                        <button
                            class="ghost-btn compact-btn"
                            type="button"
                            on:click={exportLogs}
                            disabled={logs.length === 0}>导出日志</button
                        >
                        <button
                            class="ghost-btn compact-btn"
                            type="button"
                            on:click={exportReport}
                            disabled={rows.length === 0}>导出报告</button
                        >
                        <button
                            class="ghost-btn compact-btn"
                            type="button"
                            on:click={clearLog}
                            disabled={logs.length === 0}>清空日志</button
                        >
                    </div>
                </div>
                <div class="log-list">
                    {#if filteredLogs.length === 0}
                        <div class="empty">尚未执行任务。</div>
                    {:else}
                        {#each filteredLogs as log}
                            <div class={`log-line level-${log.level}`}>
                                {t(log.text)}
                            </div>
                        {/each}
                    {/if}
                </div>
            </section>
        </div>

        <section class="panel queue-panel" aria-label="文件队列">
            <div class="panel-head">
                <div>
                    <div class="eyebrow">文件队列</div>
                    <h2>待处理列表</h2>
                </div>
                <div class="queue-head-actions">
                    {#if rows.length > 0}
                        <div class="status-counts" aria-label="队列状态统计">
                            <span>等待 {queueStats.waiting}</span>
                            <span>处理中 {queueStats.running}</span>
                            <span>完成 {queueStats.done}</span>
                            <span
                                >异常 {queueStats.warning +
                                    queueStats.error}</span
                            >
                        </div>
                    {/if}
                    <button
                        class="ghost-btn compact-btn"
                        type="button"
                        on:click={rescanDirectories}
                        disabled={running ||
                            scanning ||
                            sourceDirectories.length === 0}>重新扫描</button
                    >
                    <span class="count-pill">{rows.length}</span>
                </div>
            </div>
            <div class="queue-tools" aria-label="队列筛选">
                <div class="segmented queue-filter">
                    <button
                        type="button"
                        class:active={queueFilter === "all"}
                        on:click={() => (queueFilter = "all")}>全部</button
                    >
                    <button
                        type="button"
                        class:active={queueFilter === "waiting"}
                        on:click={() => (queueFilter = "waiting")}>等待</button
                    >
                    <button
                        type="button"
                        class:active={queueFilter === "running"}
                        on:click={() => (queueFilter = "running")}
                        >处理中</button
                    >
                    <button
                        type="button"
                        class:active={queueFilter === "done"}
                        on:click={() => (queueFilter = "done")}>完成</button
                    >
                    <button
                        type="button"
                        class:active={queueFilter === "issue"}
                        on:click={() => (queueFilter = "issue")}>异常</button
                    >
                </div>
                <input
                    class="queue-search"
                    type="search"
                    bind:value={queueSearch}
                    placeholder="搜索文件名或路径"
                    aria-label="搜索队列文件"
                />
                {#if rows.length > 0 && visibleRows.length !== rows.length}
                    <span class="queue-visible-count"
                        >显示 {visibleRows.length} / {rows.length}</span
                    >
                {/if}
            </div>
            <div class="queue-list">
                {#if rows.length === 0}
                    <div class="empty">还没有加入 EPUB 文件或目录。</div>
                {:else if visibleRows.length === 0}
                    <div class="empty">没有匹配的队列文件。</div>
                {:else}
                    {#each visibleRows as row}
                        <div class={`queue-row status-${row.status}`}>
                            <div class="queue-main">
                                <strong>{basename(row.inputPath)}</strong>
                                <span>{t(row.message)}</span>
                                <small title={row.outputPath || row.inputPath}
                                    >{row.outputPath || row.inputPath}</small
                                >
                            </div>
                            <div class="queue-actions">
                                <span class="queue-status"
                                    >{statusLabel(row.status)}</span
                                >
                                <button
                                    class="icon-text-btn"
                                    type="button"
                                    on:click={() =>
                                        revealPath(
                                            row.outputPath || row.inputPath,
                                        )}
                                    title="打开位置">定位</button
                                >
                                <button
                                    class="icon-text-btn danger"
                                    type="button"
                                    on:click={() =>
                                        removeInputPath(row.inputPath)}
                                    disabled={running || scanning}
                                    title="从队列移除">移除</button
                                >
                            </div>
                        </div>
                    {/each}
                {/if}
            </div>
        </section>
    </section>
</main>

<style>
    :global(body) {
        margin: 0;
        overflow: hidden;
        background: var(--color-canvas);
    }

    .batch-app {
        box-sizing: border-box;
        height: 100vh;
        display: grid;
        grid-template-columns: 1fr;
        grid-template-rows: auto minmax(0, 1fr);
        grid-template-areas:
            "source"
            "workspace";
        gap: 14px;
        padding: 20px 24px 22px;
        overflow: hidden;
        background: var(--color-canvas);
        color: var(--color-text);
        font-family: var(--font-ui);
    }

    .batch-head,
    .source-panel,
    .panel-head {
        display: flex;
        align-items: flex-start;
        justify-content: space-between;
        gap: 16px;
    }

    .batch-head h1,
    .source-copy h2,
    .panel-head h2 {
        margin: 0;
        color: var(--color-text);
        font-weight: 800;
        letter-spacing: 0;
    }

    .batch-head h1 {
        font-size: 22px;
        line-height: 1.25;
    }

    .batch-head {
        display: none;
    }

    .batch-head .eyebrow {
        display: none;
    }

    .source-copy h2,
    .panel-head h2 {
        font-size: 16px;
        line-height: 1.35;
    }

    .batch-head p,
    .source-copy p {
        margin: 8px 0 0;
        color: var(--color-muted);
        font-size: 13px;
        line-height: 1.45;
    }

    .eyebrow {
        margin-bottom: 6px;
        color: var(--color-accent-deep);
        font-size: 12px;
        line-height: 1.2;
        font-weight: 800;
    }

    .source-panel,
    .panel {
        box-sizing: border-box;
        min-width: 0;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-surface);
        box-shadow: var(--shadow-xs);
    }

    .source-panel {
        grid-area: source;
        align-items: center;
        padding: 14px 18px;
    }

    .source-actions,
    .panel-actions,
    .queue-head-actions {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        flex-wrap: wrap;
        gap: 10px;
    }

    button {
        box-sizing: border-box;
        min-height: 36px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        padding: 7px 14px;
        font: inherit;
        font-size: 13px;
        font-weight: 800;
        line-height: 1.2;
        cursor: pointer;
        transition:
            background var(--transition-fast),
            border-color var(--transition-fast),
            color var(--transition-fast),
            opacity var(--transition-fast);
    }

    button:disabled {
        cursor: not-allowed;
        opacity: 0.55;
    }

    button:focus-visible {
        outline: none;
        box-shadow: var(--focus-ring);
    }

    .primary-btn,
    .start-btn {
        border-color: color-mix(
            in srgb,
            var(--color-accent) 42%,
            var(--color-border)
        );
        background: var(--color-accent);
        color: var(--color-accent-contrast, var(--color-surface));
    }

    .cancel-btn {
        border-color: color-mix(
            in srgb,
            var(--color-danger) 28%,
            var(--color-border)
        );
        background: color-mix(
            in srgb,
            var(--color-danger) 10%,
            var(--color-surface)
        );
        color: var(--color-danger);
    }

    .retry-btn {
        border-color: color-mix(
            in srgb,
            var(--color-warning) 32%,
            var(--color-border)
        );
        background: color-mix(
            in srgb,
            var(--color-warning) 12%,
            var(--color-surface)
        );
        color: var(--color-warning);
    }

    .report-btn {
        border-color: var(--color-border);
        background: var(--color-canvas);
        color: var(--color-text);
    }

    .strong-btn {
        border-color: var(--color-border-strong);
        background: var(--color-text);
        color: var(--color-surface);
    }

    .ghost-btn {
        background: var(--color-surface);
        color: var(--color-text);
    }

    button:hover:not(:disabled) {
        border-color: var(--color-border-strong);
    }

    .workspace {
        grid-area: workspace;
        min-height: 0;
        display: grid;
        grid-template-columns: minmax(360px, 0.9fr) minmax(420px, 1.1fr);
        gap: 14px;
        overflow: hidden;
    }

    .left-column {
        min-width: 0;
        min-height: 0;
        display: grid;
        grid-template-rows: max-content minmax(0, 1fr);
        gap: 14px;
        overflow: hidden;
    }

    .panel {
        min-height: 0;
        padding: 14px;
        overflow: hidden;
    }

    .execute-panel {
        display: flex;
        flex-direction: column;
        gap: 10px;
        overflow-y: auto;
        scrollbar-gutter: stable;
    }

    .field-block {
        display: grid;
        gap: 6px;
        padding: 10px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-canvas);
    }

    .execute-panel .panel-actions button {
        min-height: 32px;
        padding: 6px 10px;
        font-size: 12px;
    }

    .field-block span {
        color: var(--color-muted);
        font-size: 12px;
        font-weight: 700;
    }

    .field-block strong {
        min-width: 0;
        color: var(--color-text);
        font-size: 13px;
        line-height: 1.4;
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .field-row {
        display: grid;
        grid-template-columns: 1fr 1fr;
        gap: 12px;
    }

    .field-select {
        display: grid;
        gap: 6px;
        padding: 10px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-canvas);
        min-width: 0;
    }

    .field-select span {
        color: var(--color-muted);
        font-size: 12px;
        font-weight: 700;
    }

    .field-select :global(.batch-select) {
        --control-height: 44px;
    }

    @media (max-width: 720px) {
        .field-row {
            grid-template-columns: 1fr;
        }
    }

    .tool-summary {
        min-width: 0;
        display: grid;
        gap: 6px;
    }

    .tool-summary strong {
        font-size: 15px;
        line-height: 1.3;
    }

    .tool-summary span {
        color: var(--color-muted);
        font-size: 13px;
        line-height: 1.4;
        overflow-wrap: anywhere;
    }

    .tool-run-row {
        display: grid;
        grid-template-columns: minmax(0, 1fr) repeat(4, auto);
        gap: 8px;
        align-items: center;
    }

    .start-btn,
    .cancel-btn,
    .retry-btn,
    .report-btn {
        min-width: 108px;
        min-height: 36px;
        padding-inline: 16px;
        flex: 0 0 auto;
    }

    .progress-area {
        margin-top: 0;
    }

    .progress-meta {
        display: flex;
        justify-content: space-between;
        margin-bottom: 8px;
        color: var(--color-muted);
        font-size: 12px;
        font-weight: 800;
    }

    .progress-meta strong {
        color: var(--color-text);
    }

    .progress-track {
        height: 10px;
        overflow: hidden;
        border-radius: 999px;
        background: var(--color-canvas);
        box-shadow: inset 0 0 0 1px var(--color-border);
    }

    .progress-bar {
        height: 100%;
        border-radius: inherit;
        background: var(--color-accent);
        transition: width var(--transition-fast);
    }

    .completion-summary {
        display: flex;
        flex-wrap: wrap;
        gap: 8px;
        padding-top: 2px;
    }

    .completion-summary span {
        padding: 5px 8px;
        border: 1px solid var(--color-border);
        border-radius: 999px;
        background: var(--color-canvas);
        color: var(--color-muted);
        font-size: 12px;
        font-weight: 800;
        line-height: 1.2;
    }

    .completion-summary span.error {
        border-color: color-mix(
            in srgb,
            var(--color-danger) 26%,
            var(--color-border)
        );
        background: color-mix(
            in srgb,
            var(--color-danger) 10%,
            var(--color-canvas)
        );
        color: var(--color-danger);
    }

    .queue-panel,
    .log-panel {
        min-height: 0;
        display: grid;
        gap: 12px;
    }

    .queue-panel {
        grid-template-rows: auto auto minmax(0, 1fr);
    }

    .log-panel {
        grid-template-rows: auto minmax(0, 1fr);
    }

    .log-actions {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        flex-wrap: wrap;
        gap: 8px;
    }

    .segmented {
        display: inline-flex;
        align-items: center;
        overflow: hidden;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-canvas);
    }

    .segmented button {
        min-height: 30px;
        border: 0;
        border-radius: 0;
        padding: 5px 10px;
        background: transparent;
        color: var(--color-muted);
        font-size: 12px;
    }

    .segmented button.active {
        background: var(--color-accent);
        color: var(--color-accent-contrast, var(--color-surface));
    }

    .count-pill {
        min-width: 34px;
        padding: 4px 8px;
        border-radius: 999px;
        background: var(--color-accent-quiet);
        color: var(--color-accent-deep);
        font-size: 12px;
        font-weight: 800;
        text-align: center;
    }

    .status-counts {
        display: flex;
        align-items: center;
        justify-content: flex-end;
        flex-wrap: wrap;
        gap: 6px;
    }

    .status-counts span {
        padding: 4px 7px;
        border: 1px solid var(--color-border);
        border-radius: 999px;
        background: var(--color-canvas);
        color: var(--color-muted);
        font-size: 11px;
        font-weight: 800;
        line-height: 1.2;
        white-space: nowrap;
    }

    .queue-tools {
        min-width: 0;
        display: grid;
        grid-template-columns: auto minmax(140px, 1fr) auto;
        align-items: center;
        gap: 8px;
    }

    .queue-filter {
        justify-self: start;
    }

    .queue-search {
        box-sizing: border-box;
        min-width: 0;
        min-height: 30px;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        padding: 5px 10px;
        background: var(--color-canvas);
        color: var(--color-text);
        font: inherit;
        font-size: 12px;
        line-height: 1.2;
    }

    .queue-search:focus {
        outline: none;
        box-shadow: var(--focus-ring);
    }

    .queue-visible-count {
        color: var(--color-muted);
        font-size: 12px;
        font-weight: 800;
        white-space: nowrap;
    }

    .compact-btn {
        min-height: 30px;
        padding: 5px 10px;
        font-size: 12px;
    }

    .queue-list,
    .log-list {
        box-sizing: border-box;
        min-height: 0;
        height: 100%;
        overflow-x: hidden;
        overflow-y: auto;
        border: 1px solid var(--color-border);
        border-radius: var(--radius-sm);
        background: var(--color-canvas);
    }

    .queue-row {
        min-height: 64px;
        box-sizing: border-box;
        display: grid;
        grid-template-columns: minmax(0, 1fr) auto;
        gap: 12px;
        align-items: center;
        padding: 10px 12px;
        border-bottom: 1px solid var(--color-border);
    }

    .queue-row:last-child {
        border-bottom: 0;
    }

    .queue-main {
        min-width: 0;
        display: flex;
        flex-direction: column;
        gap: 3px;
    }

    .queue-main strong,
    .queue-main span,
    .queue-main small {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .queue-main strong {
        font-size: 13px;
        line-height: 1.3;
    }

    .queue-main span,
    .queue-main small {
        color: var(--color-muted);
        font-size: 12px;
        line-height: 1.35;
    }

    .queue-status {
        padding: 4px 8px;
        border-radius: 999px;
        background: var(--color-accent-quiet);
        color: var(--color-accent-deep);
        font-size: 11px;
        font-weight: 800;
        line-height: 1.2;
    }

    .queue-actions {
        justify-self: end;
        display: flex;
        align-items: center;
        gap: 6px;
    }

    .icon-text-btn {
        min-height: 26px;
        padding: 4px 8px;
        border-color: transparent;
        background: transparent;
        color: var(--color-muted);
        font-size: 11px;
    }

    .icon-text-btn:hover:not(:disabled) {
        background: var(--color-surface);
        color: var(--color-text);
    }

    .icon-text-btn.danger:hover:not(:disabled) {
        color: var(--color-danger);
    }

    .status-running .queue-status {
        background: color-mix(in srgb, var(--color-accent) 18%, transparent);
    }

    .status-error .queue-status {
        background: color-mix(in srgb, var(--color-danger) 12%, transparent);
        color: var(--color-danger);
    }

    .status-warning .queue-status {
        background: color-mix(in srgb, var(--color-warning) 14%, transparent);
        color: var(--color-warning);
    }

    .log-list {
        max-height: 100%;
        padding: 10px 12px;
        color: var(--color-muted);
        font-family: ui-monospace, SFMono-Regular, Consolas, monospace;
        font-size: 12px;
        line-height: 1.5;
    }

    .log-line {
        overflow-wrap: anywhere;
    }

    .log-line.level-warning {
        color: var(--color-warning);
    }

    .log-line.level-error {
        color: var(--color-danger);
    }

    .empty {
        padding: 16px;
        color: var(--color-muted);
        font-size: 13px;
        line-height: 1.5;
        text-align: center;
    }

    @media (max-width: 820px) {
        :global(body) {
            overflow: auto;
        }

        .batch-app {
            grid-template-columns: 1fr;
            grid-template-rows: auto auto;
            grid-template-areas:
                "source"
                "workspace";
            height: auto;
            min-height: 100vh;
            padding: 16px;
            overflow: auto;
        }

        .source-panel,
        .batch-head,
        .panel-head {
            flex-direction: column;
            align-items: stretch;
        }

        .workspace {
            grid-template-columns: 1fr;
            gap: 14px;
            overflow: visible;
        }

        .left-column {
            overflow: visible;
        }

        .queue-panel {
            min-height: 320px;
        }

        .log-panel {
            min-height: 220px;
        }

        .source-actions,
        .panel-actions,
        .queue-head-actions,
        .log-actions {
            justify-content: flex-start;
        }

        .tool-run-row {
            grid-template-columns: 1fr;
        }

        .queue-tools {
            grid-template-columns: 1fr;
            align-items: stretch;
        }

        .queue-filter {
            max-width: 100%;
            overflow-x: auto;
        }

        .queue-visible-count {
            justify-self: start;
        }
    }

    /* ——— 手机形态（不看宽度） ———

       上面那条 @media (max-width: 820px) 只覆盖竖屏：横屏手机宽 844px
       命中不了，页根 .batch-app 仍是 height:100vh + overflow:hidden，
       两栏 grid 又只有 337-53=284px 可用 —— 工作区被压成 182px，
       「输出与执行」里一半的字段和整块日志被 overflow:hidden 直接裁掉，
       没有任何滚动入口。

       手机上改成整页自然高度 + 页面级滚动，grid 行高只跟内容走。
       桌面（宽窗、Tauri 窗口）不命中，行为完全不变。 */
    :global(:root[data-tepub-layout="phone"]) .batch-app {
        height: auto;
        min-height: 100%;
        grid-template-rows: max-content max-content;
        overflow: auto;
    }

    :global(:root[data-tepub-layout="phone"]) .batch-app .workspace {
        grid-template-columns: 1fr;
        grid-auto-rows: max-content;
        overflow: visible;
    }

    :global(:root[data-tepub-layout="phone"]) .batch-app .left-column {
        grid-template-rows: max-content max-content;
        overflow: visible;
    }

    /* 队列与日志本身是被设计成"固定高度、内部滚动"的面板；
       外层已经没有高度可分了，给它们各自留一屏可观的高度。 */
    :global(:root[data-tepub-layout="phone"]) .batch-app .queue-panel {
        min-height: 320px;
    }

    :global(:root[data-tepub-layout="phone"]) .batch-app .log-panel {
        min-height: 220px;
    }

    /* 队列/日志列表本身写着 height:100%。外层 .panel 现在是 max-content 高度
       （见上面两条），百分比高度失去参照，退回内容高度 —— 面板的
       min-height 撑起来了，列表却只有一行高，剩下的空白不属于任何滚动区。
       改成用面板高度减掉面板头部：列表自己滚动，面板不留死白。

       但不能就这么放开：列表一旦按内容撑开，200 个文件就是 200×64px，
       队列面板实测 2187px —— 日志被顶到一屏之外，批量跑的时候等于看不见。
       手机上给列表一个上限，滚动仍然收在列表内部。

       横屏只有 390px 高，纯 45vh 会把列表压到 175px（比一行文件多不了多少），
       所以用 clamp 兜一个下限 220px —— 整页本来就在滚，多出来的高度
       只是让用户多滑一屏，比"只能看见一行"强。 */
    :global(:root[data-tepub-layout="phone"]) .batch-app .queue-list,
    :global(:root[data-tepub-layout="phone"]) .batch-app .log-list {
        height: auto;
        max-height: clamp(220px, 45vh, 480px);
    }

    :global(:root[data-tepub-layout="phone"]) .batch-app .queue-panel,
    :global(:root[data-tepub-layout="phone"]) .batch-app .log-panel {
        display: flex;
        flex-direction: column;
    }

    /* 面板里的按钮（筛选页签、导出日志、重新扫描…）在桌面写了 30px，
       手指点不中。手机上统一抬到触控尺寸；这是纯 phones 规则，
       桌面窗口的密度不变。 */
    :global(:root[data-tepub-layout="phone"]) .batch-app .compact-btn,
    :global(:root[data-tepub-layout="phone"]) .batch-app .segmented button,
    :global(:root[data-tepub-layout="phone"]) .batch-app .icon-text-btn,
    :global(:root[data-tepub-layout="phone"]) .batch-app .primary-btn,
    :global(:root[data-tepub-layout="phone"]) .batch-app .strong-btn,
    :global(:root[data-tepub-layout="phone"]) .batch-app .ghost-btn,
    :global(:root[data-tepub-layout="phone"]) .batch-app .start-btn,
    /* 搜索框也是可点的：页面自己写了 min-height: 30px，特异性(0,2,0)
       高过 app.css 里零特异性的 :where(input)，所以它没被全局的
       --control-height 抬起来（实测 31px），和旁边 44px 的筛选页签
       并排时明显矮一截。 */
    :global(:root[data-tepub-layout="phone"]) .batch-app .queue-search {
        min-height: var(--tap-target);
    }

    :global(:root[data-tepub-layout="phone"]) .batch-app .queue-list,
    :global(:root[data-tepub-layout="phone"]) .batch-app .log-list {
        flex: 1 1 auto;
        min-height: 0;
    }
</style>
